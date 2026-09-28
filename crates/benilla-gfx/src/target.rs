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

/// The float colour and depth the cameras draw into. Two colours share the depth, as Bevy's
/// `ViewTarget` ping-pongs its main texture: a post pass reads [`Self::current`] and writes the
/// other, which the later cameras and the present then use.
pub struct SceneTarget {
    device: GfxDevice,
    pub colors: [GfxTexture; 2],
    pub depth: GfxTexture,
    pub framebuffers: [GfxFramebuffer; 2],
    pub current: usize,
    pub size: UVec2,
    pub format: GfxFormat,
}

impl SceneTarget {
    pub(crate) fn new(device: GfxDevice, size: UVec2) -> Result<Self, String> {
        Self::with_format(device, size, SCENE_FORMAT, true)
    }

    /// A pair of `format` colours, sharing a depth when `depth`: a camera's main textures, as
    /// bevy makes them per render target (`prepare_view_targets`), `Hdr` float or else 8-bit sRGB.
    pub(crate) fn with_format(
        device: GfxDevice,
        size: UVec2,
        format: GfxFormat,
        depth: bool,
    ) -> Result<Self, String> {
        let size = size.max(UVec2::ONE);
        let mut target = Self {
            device,
            colors: [ptr::null_mut(); 2],
            depth: ptr::null_mut(),
            framebuffers: [ptr::null_mut(); 2],
            current: 0,
            size,
            format,
        };
        if depth {
            target.depth = render_texture(device, DEPTH_FORMAT, size)
                .ok_or("scene depth texture creation failed")?;
        }
        for i in 0..2 {
            target.colors[i] = render_texture(device, format, size)
                .ok_or("scene colour texture creation failed")?;
            target.framebuffers[i] = framebuffer(device, target.colors[i], target.depth, size)
                .ok_or("scene framebuffer creation failed")?;
        }
        Ok(target)
    }

    /// The colour the next draw lands in.
    pub fn color(&self) -> GfxTexture {
        self.colors[self.current]
    }

    pub fn framebuffer(&self) -> GfxFramebuffer {
        self.framebuffers[self.current]
    }

    /// The pipeline class of a draw into this target: its colour format and whether it has depth.
    pub fn class(&self) -> TargetClass {
        TargetClass {
            format: self.format,
            depth: !self.depth.is_null(),
        }
    }
}

/// What a pipeline is made against: a vk pipeline is only valid in a render pass of the same
/// attachment formats, so pipelines are kept per class, never per framebuffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TargetClass {
    pub format: GfxFormat,
    pub depth: bool,
}

/// A camera's `RenderTarget::Image`: the main pair every camera on the image draws into, and the
/// image's own texture, which bevy's `upscaling` blit writes over each camera's viewport and which
/// every draw sampling the image reads ([`crate::images::GpuImages`]).
///
/// On GL the cameras draw it upside down (a clip-space Y flip), so its rows run top-down like an
/// uploaded image's and every sampler reads it unchanged.
pub struct ImageTarget {
    device: GfxDevice,
    pub main: SceneTarget,
    pub output: GfxTexture,
    pub output_framebuffer: GfxFramebuffer,
    /// The image's size and format when the target was made.
    pub size: UVec2,
    pub image_format: bevy::render::render_resource::TextureFormat,
    pub hdr: bool,
    pub depth: bool,
    /// Set by the first blit of a frame, which clears the output as bevy's `OutputColorAttachment`
    /// does on its first use.
    pub written: bool,
}

impl ImageTarget {
    /// `output` is the image's texture, made by the caller with the image's sampler; the target
    /// owns it from here.
    pub(crate) fn new(
        device: GfxDevice,
        output: GfxTexture,
        size: UVec2,
        image_format: bevy::render::render_resource::TextureFormat,
        hdr: bool,
        depth: bool,
    ) -> Result<Self, String> {
        let format = if hdr { SCENE_FORMAT } else { UI_FORMAT };
        let main = match SceneTarget::with_format(device, size, format, depth) {
            Ok(m) => m,
            Err(e) => {
                // SAFETY: made on `device` by the caller and handed to this target.
                unsafe { ffi::gfx_dll_delete_texture(device, output) };
                return Err(e);
            }
        };
        let mut target = Self {
            device,
            main,
            output,
            output_framebuffer: ptr::null_mut(),
            size,
            image_format,
            hdr,
            depth,
            written: false,
        };
        target.output_framebuffer = framebuffer(device, output, ptr::null_mut(), size)
            .ok_or("image target framebuffer creation failed")?;
        Ok(target)
    }
}

impl Drop for ImageTarget {
    fn drop(&mut self) {
        // SAFETY: each handle was made on `self.device` and belongs to this target alone.
        unsafe {
            if !self.output_framebuffer.is_null() {
                ffi::gfx_dll_delete_framebuffer(self.device, self.output_framebuffer);
            }
            ffi::gfx_dll_delete_texture(self.device, self.output);
        }
    }
}

/// The UI lane's byte target: 8-bit sRGB, as the `Camera2d` main texture it stands for, so the
/// lane's premultiplied gamma values blend as the wgpu path's do and store encoded. No depth: a
/// `Mesh2d` draw's `GreaterEqual` test against the cleared 2D depth always passes.
pub struct UiTarget {
    device: GfxDevice,
    pub color: GfxTexture,
    pub framebuffer: GfxFramebuffer,
}

pub const UI_FORMAT: GfxFormat = GfxFormat::R8G8B8A8Srgb;

impl UiTarget {
    pub(crate) fn new(device: GfxDevice, size: UVec2) -> Result<Self, String> {
        let color = render_texture(device, UI_FORMAT, size.max(UVec2::ONE))
            .ok_or("UI colour texture creation failed")?;
        let mut target = Self {
            device,
            color,
            framebuffer: ptr::null_mut(),
        };
        target.framebuffer = framebuffer(device, color, ptr::null_mut(), size.max(UVec2::ONE))
            .ok_or("UI framebuffer creation failed")?;
        Ok(target)
    }
}

impl Drop for UiTarget {
    fn drop(&mut self) {
        // SAFETY: each handle was made on `self.device` and belongs to this target alone.
        unsafe {
            if !self.framebuffer.is_null() {
                ffi::gfx_dll_delete_framebuffer(self.device, self.framebuffer);
            }
            ffi::gfx_dll_delete_texture(self.device, self.color);
        }
    }
}

/// A framebuffer of one colour and an optional depth.
pub(crate) fn framebuffer(
    device: GfxDevice,
    color: GfxTexture,
    depth: GfxTexture,
    size: UVec2,
) -> Option<GfxFramebuffer> {
    let mut colors = [color];
    let info = ffi::GfxFramebufferCreateInfo {
        color_attachments: colors.as_mut_ptr(),
        depth_stencil_attachment: depth,
        color_count: 1,
        msaa_samples: 1,
        width: size.x,
        height: size.y,
    };
    let mut fb: GfxFramebuffer = ptr::null_mut();
    // SAFETY: `info` and `colors` are live for the call.
    unsafe { ffi::gfx_dll_create_framebuffer(device, &info, &mut fb) }.then_some(fb)
}

impl Drop for SceneTarget {
    fn drop(&mut self) {
        // SAFETY: each handle was made on `self.device` and belongs to this target alone.
        unsafe {
            for fb in self.framebuffers {
                if !fb.is_null() {
                    ffi::gfx_dll_delete_framebuffer(self.device, fb);
                }
            }
            for t in [self.colors[0], self.colors[1], self.depth] {
                if !t.is_null() {
                    ffi::gfx_dll_delete_texture(self.device, t);
                }
            }
        }
    }
}

/// A sampled render target, linear-filtered (the post passes tap between texels; the present
/// reads texel centres at 1:1, where linear is nearest) and clamped.
pub(crate) fn render_texture(
    device: GfxDevice,
    format: GfxFormat,
    size: UVec2,
) -> Option<GfxTexture> {
    let filter = if format == DEPTH_FORMAT {
        GfxFiltering::Nearest
    } else {
        GfxFiltering::Linear
    };
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
        min_filtering: filter,
        mag_filtering: filter,
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
    shader_state: GfxShaderState,
    /// The pipeline drawing into a [`CAPTURE_FORMAT`] framebuffer, made on first use.
    offscreen: GfxPipeline,
}

/// The window's pixels as a screenshot reads them: the present's output, stored in a texture.
pub const CAPTURE_FORMAT: GfxFormat = GfxFormat::R8G8B8A8Unorm;

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
            shader_state,
            offscreen: ptr::null_mut(),
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
        let blend = pipelines::blend_state(device, Blend::Replace, true);
        let depth = pipelines::depth_state(device, false, false, false);
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
        self.draw_with(self.pipeline, ptr::null_mut(), scene, size);
    }

    /// Draws `scene` into `target`, a [`CAPTURE_FORMAT`] framebuffer of `size` pixels, as it
    /// draws the window: its rows run as the window's on vk and d3d, bottom-up on GL.
    pub(crate) fn draw_into(&mut self, target: GfxFramebuffer, scene: GfxTexture, size: UVec2) {
        if self.offscreen.is_null() {
            let [blend, depth, raster] = self.states;
            let info = ffi::GfxPipelineCreateInfo {
                shader_state: self.shader_state,
                rasterizer_state: raster,
                depth_stencil_state: depth,
                blend_state: blend,
                input_layout: self.layout,
                primitive: GfxPrimitiveType::Triangles,
                target_framebuffer: target,
            };
            // SAFETY: every state in `info` is live and made on this device.
            if !unsafe { ffi::gfx_dll_create_pipeline(self.device, &info, &mut self.offscreen) } {
                self.offscreen = ptr::null_mut();
                return;
            }
        }
        self.draw_with(self.offscreen, target, scene, size);
    }

    fn draw_with(
        &self,
        pipeline: GfxPipeline,
        target: GfxFramebuffer,
        scene: GfxTexture,
        size: UVec2,
    ) {
        let device = self.device;
        let mut textures = [scene];
        // SAFETY: every handle is live and made on `device`, on its thread.
        unsafe {
            ffi::gfx_dll_bind_framebuffer(device, target);
            ffi::gfx_dll_set_viewport(device, 0, 0, size.x, size.y, 0.0, 1.0);
            ffi::gfx_dll_set_scissor(device, 0, 0, size.x, size.y);
            ffi::gfx_dll_bind_pipeline(device, pipeline);
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
            for p in [self.pipeline, self.offscreen] {
                if !p.is_null() {
                    ffi::gfx_dll_delete_pipeline(device, p);
                }
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
