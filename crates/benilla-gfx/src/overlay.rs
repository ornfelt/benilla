//! A camera's overlay: a `Camera2d` on the window whose draws a system hands over already
//! tessellated, as bevy_egui's pass takes them (bevy_egui 0.39.1 `render/systems.rs`,
//! `render_pass.rs`): one vertex and index stream, each draw an index range with its own texture
//! and scissor rect ([`GfxOverlays`], filled by [`crate::egui`]). The camera draws into the
//! overlay target, 8-bit sRGB as a `Camera2d`'s main texture, cleared as its `ClearColorConfig`
//! says, through the `egui` program (`egui.wgsl`, premultiplied blend, the whole target as its
//! viewport); bevy's `upscaling` then blends that over the frame as the window holds it through
//! the camera's output blend ([`crate::post::FfxPost::composite`]).

use std::ops::Range;
use std::ptr;

use bevy::asset::AssetId;
use bevy::ecs::entity::EntityHashMap;
use bevy::math::{URect, UVec2};
use bevy::prelude::*;

use crate::ffi::{
    self, GfxAttributesState, GfxBuffer, GfxDevice, GfxFormat, GfxInputLayout, GfxPipeline,
    GfxPrimitiveType, GfxStepMode, GfxTexture,
};
use crate::pipelines::{self, Blend};
use crate::shader_loader::ShaderLibrary;
use crate::target::UiTarget;

/// Bytes a vertex: position (2 x `f32`, logical points), uv (2 x `f32`), colour (4 x `u8`,
/// premultiplied sRGB), egui's `Vertex`.
pub const OVERLAY_VERTEX: usize = 20;

/// `std140` size of `egui_block`: the transform row.
pub(crate) const OVERLAY_BLOCK: usize = 16;

const PROGRAM: &str = "egui";

/// This frame's overlays by camera, rebuilt by their producers in
/// [`crate::GfxRenderSystems::Collect`].
#[derive(Resource, Default)]
pub struct GfxOverlays(pub EntityHashMap<GfxOverlayFrame>);

/// One camera's overlay for the frame.
#[derive(Debug, Default)]
pub struct GfxOverlayFrame {
    /// [`OVERLAY_VERTEX`] bytes a vertex.
    pub vertices: Vec<u8>,
    /// Into `vertices`, from 0.
    pub indices: Vec<u32>,
    pub draws: Vec<GfxOverlayDraw>,
    /// Logical points to clip space: the scale (x, y), then the translation (z, w), bevy_egui's
    /// `EguiTransform`.
    pub transform: [f32; 4],
}

/// One draw of an overlay: `indices` with `image`, clipped to `scissor` (physical pixels,
/// top-down, inside the camera's viewport, not empty).
#[derive(Debug, Clone)]
pub struct GfxOverlayDraw {
    pub image: AssetId<Image>,
    pub scissor: URect,
    pub indices: Range<u32>,
}

impl GfxOverlayFrame {
    /// Appends `vertices` (their [`OVERLAY_VERTEX`] bytes each) and `indices` into them as one
    /// draw with `image` clipped to `scissor`.
    pub fn push(
        &mut self,
        vertices: &[u8],
        indices: &[u32],
        image: AssetId<Image>,
        scissor: URect,
    ) {
        let base = (self.vertices.len() / OVERLAY_VERTEX) as u32;
        let first = self.indices.len() as u32;
        self.vertices.extend_from_slice(vertices);
        self.indices.extend(indices.iter().map(|i| i + base));
        self.draws.push(GfxOverlayDraw {
            image,
            scissor,
            indices: first..self.indices.len() as u32,
        });
    }
}

/// One draw staged for the device: its texture, its scissor as gfx counts it (x, y from the
/// bottom, size) and its range of the frame's index buffer.
struct StagedDraw {
    texture: GfxTexture,
    scissor: (i32, i32, UVec2),
    first: u32,
    count: u32,
}

/// The overlay pass on the device: the frame's streams of every overlay in one vertex and one
/// index buffer, the target and the pipeline made for it.
pub(crate) struct OverlayPass {
    device: GfxDevice,
    /// Blend, depth, the scissored rasterizer.
    states: [*mut std::ffi::c_void; 3],
    vertex: GfxBuffer,
    vertex_capacity: usize,
    index: GfxBuffer,
    index_capacity: usize,
    layout: GfxInputLayout,
    /// Made for the current buffers.
    attributes: GfxAttributesState,
    /// Made for the target's framebuffer.
    pipeline: GfxPipeline,
    target: Option<UiTarget>,
    vertices: Vec<u8>,
    indices: Vec<u32>,
    draws: Vec<StagedDraw>,
}

impl OverlayPass {
    pub(crate) fn new(device: GfxDevice) -> Self {
        Self {
            device,
            states: [
                pipelines::blend_state(device, Blend::Premultiplied, true),
                pipelines::depth_state(device, false, false, ffi::GfxCompareFunction::Always),
                pipelines::raster_state_scissored(device),
            ],
            vertex: ptr::null_mut(),
            vertex_capacity: 0,
            index: ptr::null_mut(),
            index_capacity: 0,
            layout: ptr::null_mut(),
            attributes: ptr::null_mut(),
            pipeline: ptr::null_mut(),
            target: None,
            vertices: Vec::new(),
            indices: Vec::new(),
            draws: Vec::new(),
        }
    }

    /// Starts the frame's staging.
    pub(crate) fn begin_frame(&mut self) {
        self.vertices.clear();
        self.indices.clear();
        self.draws.clear();
    }

    /// Stages `frame` for a `height` target, each draw's texture from `texture` (a draw whose
    /// image is not on the device is skipped, as bevy_egui skips one without a bind group);
    /// returns its range of the staged draws.
    pub(crate) fn stage(
        &mut self,
        frame: &GfxOverlayFrame,
        height: u32,
        mut texture: impl FnMut(AssetId<Image>) -> Option<GfxTexture>,
    ) -> Range<usize> {
        let start = self.draws.len();
        let base_vertex = (self.vertices.len() / OVERLAY_VERTEX) as u32;
        let base_index = self.indices.len() as u32;
        self.vertices.extend_from_slice(&frame.vertices);
        self.indices
            .extend(frame.indices.iter().map(|i| i + base_vertex));
        for draw in &frame.draws {
            let Some(texture) = texture(draw.image) else {
                continue;
            };
            let size = draw.scissor.size();
            self.draws.push(StagedDraw {
                texture,
                scissor: (
                    draw.scissor.min.x as i32,
                    height as i32 - draw.scissor.max.y as i32,
                    size,
                ),
                first: base_index + draw.indices.start,
                count: draw.indices.len() as u32,
            });
        }
        start..self.draws.len()
    }

    /// Uploads the staged streams, growing the buffers (and re-making the attributes state that
    /// names them) when they no longer fit. False when nothing is staged or the device refused.
    pub(crate) fn upload(&mut self, shaders: &mut ShaderLibrary) -> bool {
        if self.draws.is_empty() {
            return false;
        }
        if self.layout.is_null() {
            let Ok(program) = shaders.get(PROGRAM) else {
                warn_once!("gfx: the `egui` program did not load; overlays are not drawn");
                return false;
            };
            let bind = |format, offset| ffi::GfxInputLayoutBind {
                buffer: 0,
                format,
                stride: OVERLAY_VERTEX as u32,
                offset,
                step_mode: GfxStepMode::Vertex,
            };
            let binds = [
                bind(GfxFormat::R32G32Sfloat, 0),
                bind(GfxFormat::R32G32Sfloat, 8),
                bind(GfxFormat::R8G8B8A8Unorm, 16),
            ];
            match pipelines::create_layout(self.device, program.state, &binds) {
                Some(l) => self.layout = l,
                None => return false,
            }
        }
        let index_bytes: Vec<u8> = self.indices.iter().flat_map(|i| i.to_le_bytes()).collect();
        let grow_v = grow(
            self.device,
            &mut self.vertex,
            &mut self.vertex_capacity,
            ffi::GfxBufferType::Vertexes,
            self.vertices.len(),
        );
        let grow_i = grow(
            self.device,
            &mut self.index,
            &mut self.index_capacity,
            ffi::GfxBufferType::Indices,
            index_bytes.len(),
        );
        let (Some(grew_v), Some(grew_i)) = (grow_v, grow_i) else {
            return false;
        };
        if grew_v || grew_i || self.attributes.is_null() {
            if !self.attributes.is_null() {
                // SAFETY: made on this device; nothing is drawn with it past this point.
                unsafe { ffi::gfx_dll_delete_attributes_state(self.device, self.attributes) };
                self.attributes = ptr::null_mut();
            }
            let binds = [ffi::GfxAttributeBind {
                buffer: self.vertex,
            }; 3];
            let info = ffi::GfxAttributesStateCreateInfo {
                binds: binds.as_ptr(),
                count: binds.len() as u32,
                index_buffer: self.index,
                index_type: ffi::GfxIndexType::UInt32,
            };
            // SAFETY: `info` and `binds` are live for the call; both buffers are this device's.
            if !unsafe {
                ffi::gfx_dll_create_attributes_state(self.device, &info, &mut self.attributes)
            } {
                self.attributes = ptr::null_mut();
                return false;
            }
        }
        // SAFETY: each buffer holds at least its bytes (`grow`); the calls copy them.
        unsafe {
            ffi::gfx_dll_set_buffer_data(
                self.device,
                self.vertex,
                self.vertices.as_ptr().cast(),
                self.vertices.len() as u32,
                0,
            ) && ffi::gfx_dll_set_buffer_data(
                self.device,
                self.index,
                index_bytes.as_ptr().cast(),
                index_bytes.len() as u32,
                0,
            )
        }
    }

    /// The overlay target at the scene target's `size`, made on first use.
    pub(crate) fn target(&mut self, size: UVec2) -> Option<&UiTarget> {
        if self.target.is_none() {
            match UiTarget::new(self.device, size.max(UVec2::ONE)) {
                Ok(t) => self.target = Some(t),
                Err(e) => {
                    error!("gfx: overlay {e}");
                    return None;
                }
            }
        }
        self.target.as_ref()
    }

    /// Drops the target and the pipeline made for it: the scene target changed size.
    pub(crate) fn drop_target(&mut self) {
        if !self.pipeline.is_null() {
            // SAFETY: made on this device; nothing is drawn with it past this point.
            unsafe { ffi::gfx_dll_delete_pipeline(self.device, self.pipeline) };
            self.pipeline = ptr::null_mut();
        }
        self.target = None;
    }

    /// Draws `draws` of the staged ones into the target, cleared first with `clear`, through the
    /// transform block at `offset` in `ring`; returns the target's colour for the composite.
    pub(crate) fn draw(
        &mut self,
        shaders: &mut ShaderLibrary,
        ring: GfxBuffer,
        offset: u32,
        clear: Option<LinearRgba>,
        draws: Range<usize>,
    ) -> Option<GfxTexture> {
        let (fb, color, size) = {
            let t = self.target.as_ref()?;
            (t.framebuffer, t.color, t.size)
        };
        if self.pipeline.is_null() {
            let state = shaders.get(PROGRAM).ok()?.state;
            let [blend, depth, raster] = self.states;
            let info = ffi::GfxPipelineCreateInfo {
                shader_state: state,
                rasterizer_state: raster,
                depth_stencil_state: depth,
                blend_state: blend,
                input_layout: self.layout,
                primitive: GfxPrimitiveType::Triangles,
                target_framebuffer: fb,
            };
            // SAFETY: every state in `info` is live and made on this device.
            if !unsafe { ffi::gfx_dll_create_pipeline(self.device, &info, &mut self.pipeline) } {
                self.pipeline = ptr::null_mut();
                warn_once!("gfx: the overlay pipeline could not be made; overlays are not drawn");
                return None;
            }
        }
        // SAFETY: every handle is live and made on this device, on its thread; the ring holds
        // the block at `offset`, the buffers every staged index.
        unsafe {
            ffi::gfx_dll_bind_framebuffer(self.device, fb);
            ffi::gfx_dll_set_viewport(self.device, 0, 0, size.x, size.y, 0.0, 1.0);
            ffi::gfx_dll_set_scissor(self.device, 0, 0, size.x, size.y);
            if let Some(c) = clear {
                let c = ffi::GfxClearColor {
                    r: c.red,
                    g: c.green,
                    b: c.blue,
                    a: c.alpha,
                };
                ffi::gfx_dll_clear_color(self.device, fb, 0, &c);
            }
            ffi::gfx_dll_bind_pipeline(self.device, self.pipeline);
            ffi::gfx_dll_bind_attributes_state(self.device, self.attributes, self.layout);
            ffi::gfx_dll_bind_constant(self.device, 0, ring, OVERLAY_BLOCK as u32, offset);
            for d in &self.draws[draws] {
                let (x, y, extent) = d.scissor;
                let mut textures = [d.texture];
                ffi::gfx_dll_set_scissor(self.device, x, y, extent.x, extent.y);
                ffi::gfx_dll_bind_samplers(self.device, 0, 1, textures.as_mut_ptr());
                ffi::gfx_dll_draw_indexed(self.device, d.count, d.first);
            }
        }
        Some(color)
    }
}

impl Drop for OverlayPass {
    fn drop(&mut self) {
        self.drop_target();
        let device = self.device;
        let [blend, depth, raster] = self.states;
        // SAFETY: each handle was made on `device` and belongs to this pass alone.
        unsafe {
            if !self.attributes.is_null() {
                ffi::gfx_dll_delete_attributes_state(device, self.attributes);
            }
            if !self.layout.is_null() {
                ffi::gfx_dll_delete_input_layout(device, self.layout);
            }
            for b in [self.vertex, self.index] {
                if !b.is_null() {
                    ffi::gfx_dll_delete_buffer(device, b);
                }
            }
            if !blend.is_null() {
                ffi::gfx_dll_delete_blend_state(device, blend);
            }
            if !depth.is_null() {
                ffi::gfx_dll_delete_depth_stencil_state(device, depth);
            }
            if !raster.is_null() {
                ffi::gfx_dll_delete_rasterizer_state(device, raster);
            }
        }
    }
}

/// Makes `buffer` hold `needed` bytes, re-made at the next power of two (at least 64 KiB) when it
/// does not; whether it was re-made, `None` when the device refused.
fn grow(
    device: GfxDevice,
    buffer: &mut GfxBuffer,
    capacity: &mut usize,
    ty: ffi::GfxBufferType,
    needed: usize,
) -> Option<bool> {
    if !buffer.is_null() && needed <= *capacity {
        return Some(false);
    }
    if !buffer.is_null() {
        // SAFETY: made on this device; the attributes state naming it is re-made after.
        unsafe { ffi::gfx_dll_delete_buffer(device, *buffer) };
        *buffer = ptr::null_mut();
    }
    let size = needed.next_power_of_two().max(64 * 1024);
    let info = ffi::GfxBufferCreateInfo {
        buffer_type: ty,
        usage: ffi::GfxBufferUsage::Dynamic,
        data: ptr::null(),
        size: size as u32,
    };
    // SAFETY: `info` is live for the call.
    if !unsafe { ffi::gfx_dll_create_buffer(device, &info, buffer) } {
        *buffer = ptr::null_mut();
        *capacity = 0;
        return None;
    }
    *capacity = size;
    Some(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pushed_draw_rebases_its_indices_onto_the_frame() {
        let mut frame = GfxOverlayFrame::default();
        let quad = [0u8; OVERLAY_VERTEX * 4];
        let rect = URect::new(0, 0, 10, 10);
        frame.push(&quad, &[0, 1, 2, 0, 2, 3], AssetId::default(), rect);
        frame.push(
            &quad[..OVERLAY_VERTEX * 3],
            &[0, 1, 2],
            AssetId::default(),
            rect,
        );
        assert_eq!(frame.indices[6..], [4, 5, 6]);
        assert_eq!(frame.draws[1].indices, 6..9);
        assert_eq!(frame.vertices.len(), OVERLAY_VERTEX * 7);
    }
}
