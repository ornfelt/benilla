//! The per-camera post pass benilla's views run after their main passes: the FFXGlow chain
//! (`benilla-world/src/ffx_glow.rs`): Box4 down to a quarter-size target (each side at least 8),
//! Gauss4 across then down, and the combine into the scene target's other colour, which becomes
//! current, as `ViewTarget::post_process_write` flips Bevy's main texture. The combine owns the
//! frame's one gamma decode, so a camera without it hands on gamma values.
//!
//! A camera runs it when it carries [`GfxFfxGlow`], its combine uniform, which the owner of the
//! pass writes each frame.

use std::collections::HashMap;
use std::ptr;

use bevy::math::UVec2;
use bevy::prelude::*;

use crate::ffi::{
    self, GfxBuffer, GfxDevice, GfxDeviceBackend, GfxFormat, GfxFramebuffer, GfxInputLayout,
    GfxPipeline, GfxPrimitiveType, GfxStepMode, GfxTexture,
};
use crate::pipelines::{self, Blend};
use crate::shader_loader::ShaderLibrary;
use crate::target::{self, SceneTarget, SCENE_FORMAT};

/// A camera's FFXGlow combine uniform: `lane` = (the zone glow weight, the death gate, the haze
/// mix, the dither arm), `wave` = the GlowWave phases (unused until the underwater warp lands).
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct GfxFfxGlow {
    pub lane: [f32; 4],
    pub wave: [f32; 4],
}

/// `std140` size of `post_block`: `lane`, `wave`, `texel`.
pub(crate) const POST_BLOCK: usize = 48;

const PROGRAMS: [&str; 3] = ["ffx_downsample", "ffx_gauss", "ffx_combine"];

/// One program drawing into one framebuffer.
struct Pass {
    layout: GfxInputLayout,
    attributes: ffi::GfxAttributesState,
    pipeline: GfxPipeline,
}

/// The quarter-size pair the downsample and blurs ping-pong through.
struct Quarter {
    size: UVec2,
    colors: [GfxTexture; 2],
    framebuffers: [GfxFramebuffer; 2],
}

pub struct FfxPost {
    device: GfxDevice,
    /// Image-down in V: -1 on GL, whose texture rows run bottom-up.
    y_sign: f32,
    triangle: GfxBuffer,
    states: [*mut std::ffi::c_void; 3],
    quarter: Option<Quarter>,
    passes: HashMap<(&'static str, usize), Pass>,
}

impl FfxPost {
    pub(crate) fn new(device: GfxDevice, backend: GfxDeviceBackend) -> Option<Self> {
        let gl = matches!(
            backend,
            GfxDeviceBackend::Gl3 | GfxDeviceBackend::Gl4 | GfxDeviceBackend::Gles3
        );
        // (x, y, u, v) over one screen-covering triangle; V follows the device as the present's.
        let v = |y: f32| if gl { y } else { 1.0 - y };
        let quad: [f32; 12] = [
            -1.0,
            -1.0,
            0.0,
            v(0.0),
            3.0,
            -1.0,
            2.0,
            v(0.0),
            -1.0,
            3.0,
            0.0,
            v(2.0),
        ];
        let bytes: Vec<u8> = quad.iter().flat_map(|f| f.to_le_bytes()).collect();
        let info = ffi::GfxBufferCreateInfo {
            buffer_type: ffi::GfxBufferType::Vertexes,
            usage: ffi::GfxBufferUsage::Immutable,
            data: bytes.as_ptr().cast(),
            size: bytes.len() as u32,
        };
        let mut triangle = ptr::null_mut();
        // SAFETY: `info.data` points at `bytes`, live for the call.
        if !unsafe { ffi::gfx_dll_create_buffer(device, &info, &mut triangle) } {
            return None;
        }
        Some(Self {
            device,
            y_sign: if gl { -1.0 } else { 1.0 },
            triangle,
            states: [
                pipelines::blend_state(device, Blend::Replace, true),
                pipelines::depth_state(device, false, false, false),
                pipelines::raster_state(device, None),
            ],
            quarter: None,
            passes: HashMap::new(),
        })
    }

    /// The four passes' blocks for a `size` target, in pass order.
    pub(crate) fn blocks(&self, glow: &GfxFfxGlow, size: UVec2) -> [[f32; POST_BLOCK / 4]; 4] {
        let q = quarter_size(size).as_vec2();
        let full = size.as_vec2();
        let block = |texel: [f32; 4]| {
            let mut b = [0.0; POST_BLOCK / 4];
            b[..4].copy_from_slice(&glow.lane);
            b[4..8].copy_from_slice(&glow.wave);
            b[8..].copy_from_slice(&texel);
            b
        };
        [
            block([1.0 / full.x, 1.0 / full.y, self.y_sign, full.y]),
            block([1.0 / q.x, 0.0, self.y_sign, q.y]),
            block([0.0, 1.0 / q.y, self.y_sign, q.y]),
            block([1.0 / full.x, 1.0 / full.y, self.y_sign, full.y]),
        ]
    }

    /// Runs the chain on `scene`'s current colour; `offsets` are the four blocks in `ring`.
    pub(crate) fn run(
        &mut self,
        shaders: &mut ShaderLibrary,
        scene: &mut SceneTarget,
        ring: GfxBuffer,
        offsets: [u32; 4],
    ) {
        let size = scene.size;
        let q = quarter_size(size);
        if self.quarter.as_ref().is_none_or(|t| t.size != q) {
            self.drop_targets();
            self.quarter = make_quarter(self.device, q);
        }
        let Some(quarter) = &self.quarter else {
            return;
        };
        let (qc, qf) = (quarter.colors, quarter.framebuffers);
        let out = 1 - scene.current;
        let steps: [(&'static str, GfxFramebuffer, UVec2, [GfxTexture; 2], u32); 4] = [
            (PROGRAMS[0], qf[0], q, [scene.color(), ptr::null_mut()], 1),
            (PROGRAMS[1], qf[1], q, [qc[0], ptr::null_mut()], 1),
            (PROGRAMS[1], qf[0], q, [qc[1], ptr::null_mut()], 1),
            (
                PROGRAMS[2],
                scene.framebuffers[out],
                size,
                [scene.color(), qc[0]],
                2,
            ),
        ];
        for ((program, fb, extent, mut textures, count), offset) in steps.into_iter().zip(offsets) {
            let Some(pass) = self.pass(shaders, program, fb) else {
                warn_once!("gfx: an FFXGlow pass could not be made; the view stays undecoded");
                return;
            };
            let (pipeline, attributes, layout) = (pass.pipeline, pass.attributes, pass.layout);
            // SAFETY: every handle is live and made on this device, on its thread; the ring
            // holds the block at `offset`.
            unsafe {
                ffi::gfx_dll_bind_framebuffer(self.device, fb);
                ffi::gfx_dll_set_viewport(self.device, 0, 0, extent.x, extent.y, 0.0, 1.0);
                ffi::gfx_dll_set_scissor(self.device, 0, 0, extent.x, extent.y);
                ffi::gfx_dll_bind_pipeline(self.device, pipeline);
                ffi::gfx_dll_bind_attributes_state(self.device, attributes, layout);
                ffi::gfx_dll_bind_constant(self.device, 0, ring, POST_BLOCK as u32, offset);
                ffi::gfx_dll_bind_samplers(self.device, 0, count, textures.as_mut_ptr());
                ffi::gfx_dll_draw(self.device, 3, 0);
            }
        }
        scene.current = out;
    }

    fn pass(
        &mut self,
        shaders: &mut ShaderLibrary,
        program: &'static str,
        fb: GfxFramebuffer,
    ) -> Option<&Pass> {
        let key = (program, fb as usize);
        if !self.passes.contains_key(&key) {
            let state = shaders.get(program).ok()?.state;
            let bind = |offset| ffi::GfxInputLayoutBind {
                buffer: 0,
                format: GfxFormat::R32G32Sfloat,
                stride: 16,
                offset,
                step_mode: GfxStepMode::Vertex,
            };
            let layout = pipelines::create_layout(self.device, state, &[bind(0), bind(8)])?;
            let binds = [ffi::GfxAttributeBind {
                buffer: self.triangle,
            }; 2];
            let info = ffi::GfxAttributesStateCreateInfo {
                binds: binds.as_ptr(),
                count: 2,
                index_buffer: ptr::null_mut(),
                index_type: ffi::GfxIndexType::UInt16,
            };
            let mut attributes = ptr::null_mut();
            // SAFETY: `info` and `binds` are live for the call.
            unsafe { ffi::gfx_dll_create_attributes_state(self.device, &info, &mut attributes) };
            let [blend, depth, raster] = self.states;
            let info = ffi::GfxPipelineCreateInfo {
                shader_state: state,
                rasterizer_state: raster,
                depth_stencil_state: depth,
                blend_state: blend,
                input_layout: layout,
                primitive: GfxPrimitiveType::Triangles,
                target_framebuffer: fb,
            };
            let mut pipeline = ptr::null_mut();
            // SAFETY: every state in `info` is live and made on this device.
            let made = !attributes.is_null()
                && unsafe { ffi::gfx_dll_create_pipeline(self.device, &info, &mut pipeline) };
            let pass = Pass {
                layout,
                attributes,
                pipeline,
            };
            if !made {
                self.delete_pass(pass);
                return None;
            }
            self.passes.insert(key, pass);
        }
        self.passes.get(&key)
    }

    fn delete_pass(&self, pass: Pass) {
        // SAFETY: each handle was made on this device and belongs to this pass alone.
        unsafe {
            if !pass.pipeline.is_null() {
                ffi::gfx_dll_delete_pipeline(self.device, pass.pipeline);
            }
            if !pass.attributes.is_null() {
                ffi::gfx_dll_delete_attributes_state(self.device, pass.attributes);
            }
            if !pass.layout.is_null() {
                ffi::gfx_dll_delete_input_layout(self.device, pass.layout);
            }
        }
    }

    /// Drops the quarter targets and every pass (their pipelines name a framebuffer); for a
    /// re-made scene target too.
    pub(crate) fn drop_targets(&mut self) {
        for (_, pass) in std::mem::take(&mut self.passes) {
            self.delete_pass(pass);
        }
        if let Some(q) = self.quarter.take() {
            // SAFETY: made on this device, owned by this pass alone.
            unsafe {
                for fb in q.framebuffers {
                    ffi::gfx_dll_delete_framebuffer(self.device, fb);
                }
                for t in q.colors {
                    ffi::gfx_dll_delete_texture(self.device, t);
                }
            }
        }
    }
}

impl Drop for FfxPost {
    fn drop(&mut self) {
        self.drop_targets();
        let device = self.device;
        let [blend, depth, raster] = self.states;
        // SAFETY: each handle was made on `device` and belongs to this pass alone.
        unsafe {
            ffi::gfx_dll_delete_buffer(device, self.triangle);
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

/// A quarter of `size`, each side at least 8, as the reference sizes its targets (`0x6cdb40`).
fn quarter_size(size: UVec2) -> UVec2 {
    (size / 4).max(UVec2::splat(8))
}

fn make_quarter(device: GfxDevice, size: UVec2) -> Option<Quarter> {
    let mut q = Quarter {
        size,
        colors: [ptr::null_mut(); 2],
        framebuffers: [ptr::null_mut(); 2],
    };
    for i in 0..2 {
        q.colors[i] = target::render_texture(device, SCENE_FORMAT, size)?;
        q.framebuffers[i] = target::framebuffer(device, q.colors[i], ptr::null_mut(), size)?;
    }
    Some(q)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_quarter_floors_at_eight() {
        assert_eq!(quarter_size(UVec2::new(1600, 900)), UVec2::new(400, 225));
        assert_eq!(quarter_size(UVec2::new(20, 40)), UVec2::new(8, 10));
    }
}
