//! The scene target and the present pass. The world camera renders `Hdr` with
//! `Tonemapping::None` (`benilla-world` `worldview.rs`, `player/setup.rs`): linear light into an
//! `Rgba16Float` main texture, blended there, then written to the sRGB swapchain, whose store
//! encodes. The gfx window is plain `R8G8B8A8Unorm`, so the same shape here is a float target
//! with a `Depth32Float`-like depth, and a present pass that clamps and sRGB-encodes each pixel
//! into the window.
//!
//! A render-target texture's rows come back upside down on GL next to vk and d3d (gfx keeps GL's
//! clip space everywhere; the vk viewport is flipped): the present quad's V follows the device.

use std::ptr;

use bevy::math::UVec2;

use crate::ffi::{
    self, texture_usage, GfxAttributesState, GfxBuffer, GfxDevice, GfxDeviceBackend, GfxFiltering,
    GfxFormat, GfxFramebuffer, GfxInputLayout, GfxPipeline, GfxPrimitiveType, GfxShaderState,
    GfxStepMode, GfxTexture, GfxTextureAddressing, GfxTextureType,
};
use crate::pipelines::{self, Blend};

pub const SCENE_FORMAT: GfxFormat = GfxFormat::R16G16B16A16Sfloat;
pub const DEPTH_FORMAT: GfxFormat = GfxFormat::D32Sfloat;

/// The float colour and depth the cameras draw into.
pub struct SceneTarget {
    device: GfxDevice,
    pub color: GfxTexture,
    pub depth: GfxTexture,
    pub framebuffer: GfxFramebuffer,
    pub size: UVec2,
}

impl SceneTarget {
    pub(crate) fn new(device: GfxDevice, size: UVec2) -> Result<Self, String> {
        let size = size.max(UVec2::ONE);
        let mut target = Self {
            device,
            color: ptr::null_mut(),
            depth: ptr::null_mut(),
            framebuffer: ptr::null_mut(),
            size,
        };
        target.color = render_texture(device, SCENE_FORMAT, size)
            .ok_or("scene colour texture creation failed")?;
        target.depth = render_texture(device, DEPTH_FORMAT, size)
            .ok_or("scene depth texture creation failed")?;
        let mut colors = [target.color];
        let info = ffi::GfxFramebufferCreateInfo {
            color_attachments: colors.as_mut_ptr(),
            depth_stencil_attachment: target.depth,
            color_count: 1,
            msaa_samples: 1,
            width: size.x,
            height: size.y,
        };
        // SAFETY: `info` and `colors` are live for the call.
        if !unsafe { ffi::gfx_dll_create_framebuffer(device, &info, &mut target.framebuffer) } {
            return Err("scene framebuffer creation failed".into());
        }
        Ok(target)
    }
}

impl Drop for SceneTarget {
    fn drop(&mut self) {
        // SAFETY: each handle was made on `self.device` and belongs to this target alone.
        unsafe {
            if !self.framebuffer.is_null() {
                ffi::gfx_dll_delete_framebuffer(self.device, self.framebuffer);
            }
            for t in [self.color, self.depth] {
                if !t.is_null() {
                    ffi::gfx_dll_delete_texture(self.device, t);
                }
            }
        }
    }
}

fn render_texture(device: GfxDevice, format: GfxFormat, size: UVec2) -> Option<GfxTexture> {
    let info = ffi::GfxTextureCreateInfo {
        texture_type: GfxTextureType::Texture2D,
        usage: texture_usage::RENDER_TARGET | texture_usage::SAMPLED,
        format,
        levels: 1,
        width: size.x,
        height: size.y,
        depth: 1,
        addressing_s: GfxTextureAddressing::Clamp,
        addressing_t: GfxTextureAddressing::Clamp,
        addressing_r: GfxTextureAddressing::Clamp,
        min_filtering: GfxFiltering::Nearest,
        mag_filtering: GfxFiltering::Nearest,
        mip_filtering: GfxFiltering::None,
        anisotropy: 0,
        border_color: [0.0; 4],
    };
    let mut texture: GfxTexture = ptr::null_mut();
    // SAFETY: `info` is live for the call.
    unsafe { ffi::gfx_dll_create_texture(device, &info, &mut texture) }.then_some(texture)
}

/// `present.{vs,fs}.gfxs` over one screen-covering triangle.
pub struct Present {
    device: GfxDevice,
    vertices: GfxBuffer,
    layout: GfxInputLayout,
    attributes: GfxAttributesState,
    pipeline: GfxPipeline,
    states: [*mut std::ffi::c_void; 3],
}

impl Present {
    pub(crate) fn new(
        device: GfxDevice,
        backend: GfxDeviceBackend,
        shader_state: GfxShaderState,
    ) -> Result<Self, String> {
        let flip = !matches!(
            backend,
            GfxDeviceBackend::Gl3 | GfxDeviceBackend::Gl4 | GfxDeviceBackend::Gles3
        );
        // (x, y, u, v): clip (-1,-1) is the bottom-left of the window and of a GL texture.
        let v = |y: f32| if flip { 1.0 - y } else { y };
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
        let mut present = Self {
            device,
            vertices: ptr::null_mut(),
            layout: ptr::null_mut(),
            attributes: ptr::null_mut(),
            pipeline: ptr::null_mut(),
            states: [ptr::null_mut(); 3],
        };
        // SAFETY: `info.data` points at `bytes`, live for the call.
        if !unsafe { ffi::gfx_dll_create_buffer(device, &info, &mut present.vertices) } {
            return Err("present vertex buffer creation failed".into());
        }
        let bind = |offset| ffi::GfxInputLayoutBind {
            buffer: 0,
            format: GfxFormat::R32G32Sfloat,
            stride: 16,
            offset,
            step_mode: GfxStepMode::Vertex,
        };
        present.layout = pipelines::create_layout(device, shader_state, &[bind(0), bind(8)])
            .ok_or("present input layout creation failed")?;
        let binds = [ffi::GfxAttributeBind {
            buffer: present.vertices,
        }; 2];
        let info = ffi::GfxAttributesStateCreateInfo {
            binds: binds.as_ptr(),
            count: 2,
            index_buffer: ptr::null_mut(),
            index_type: ffi::GfxIndexType::UInt16,
        };
        // SAFETY: `info` and `binds` are live for the call.
        if !unsafe { ffi::gfx_dll_create_attributes_state(device, &info, &mut present.attributes) }
        {
            return Err("present attribute state creation failed".into());
        }
        let blend = pipelines::blend_state(device, Blend::Replace);
        let depth = pipelines::depth_state(device, false, false);
        let raster = pipelines::raster_state(device, None);
        present.states = [blend, depth, raster];
        let info = ffi::GfxPipelineCreateInfo {
            shader_state,
            rasterizer_state: raster,
            depth_stencil_state: depth,
            blend_state: blend,
            input_layout: present.layout,
            primitive: GfxPrimitiveType::Triangles,
            target_framebuffer: ptr::null_mut(),
        };
        // SAFETY: every state in `info` is live and made on this device.
        if !unsafe { ffi::gfx_dll_create_pipeline(device, &info, &mut present.pipeline) } {
            return Err("present pipeline creation failed".into());
        }
        Ok(present)
    }

    /// Draws `scene` over the whole window, `size` pixels.
    pub(crate) fn draw(&self, scene: GfxTexture, size: UVec2) {
        let device = self.device;
        let mut textures = [scene];
        // SAFETY: every handle is live and made on `device`, on its thread.
        unsafe {
            ffi::gfx_dll_bind_framebuffer(device, ptr::null_mut());
            ffi::gfx_dll_set_viewport(device, 0, 0, size.x, size.y, 0.0, 1.0);
            ffi::gfx_dll_set_scissor(device, 0, 0, size.x, size.y);
            ffi::gfx_dll_bind_pipeline(device, self.pipeline);
            ffi::gfx_dll_bind_attributes_state(device, self.attributes, self.layout);
            ffi::gfx_dll_bind_samplers(device, 0, 1, textures.as_mut_ptr());
            ffi::gfx_dll_draw(device, 3, 0);
        }
    }
}

impl Drop for Present {
    fn drop(&mut self) {
        let device = self.device;
        // SAFETY: each handle was made on `device` and belongs to this pass alone.
        unsafe {
            if !self.pipeline.is_null() {
                ffi::gfx_dll_delete_pipeline(device, self.pipeline);
            }
            if !self.attributes.is_null() {
                ffi::gfx_dll_delete_attributes_state(device, self.attributes);
            }
            if !self.layout.is_null() {
                ffi::gfx_dll_delete_input_layout(device, self.layout);
            }
            if !self.vertices.is_null() {
                ffi::gfx_dll_delete_buffer(device, self.vertices);
            }
            let [blend, depth, raster] = self.states;
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
