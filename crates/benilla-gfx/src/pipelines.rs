//! gfx pipelines and the fixed-function states they are built from, made once per key and kept
//! until the target they were made for is re-made. The states follow bevy_pbr's mesh pipeline:
//! reverse-Z (`GreaterEqual`, depth cleared to 0), counter-clockwise front faces, back faces
//! culled unless the material says otherwise, and the blend state of its alpha mode.

use std::collections::HashMap;
use std::ptr;

use bevy::render::render_resource::Face;

use crate::ffi::{
    self, color_mask, GfxBlendEquation, GfxBlendFunction, GfxCompareFunction, GfxCullMode,
    GfxDevice, GfxFillMode, GfxFramebuffer, GfxFrontFace, GfxInputLayout, GfxPipeline,
    GfxPrimitiveType, GfxShaderState, GfxStencilOperation, GfxStencilState, GfxStepMode,
};
use crate::material::{GfxAlpha, GfxProgram};
use crate::meshes::vertex_format;

/// The blend state of an alpha mode (bevy_pbr `MeshPipeline::specialize`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Blend {
    /// No blending.
    Replace,
    /// `BlendState::ALPHA_BLENDING`.
    Alpha,
    /// `BlendState::PREMULTIPLIED_ALPHA_BLENDING` (premultiplied and additive).
    Premultiplied,
    /// Colour `Dst * src + (1 - src_alpha) * dst`, alpha `OVER`.
    Multiply,
    /// Colour `ONE, ONE`; alpha kept (`ZERO, ONE`): benilla's gamma-space additive.
    Add,
    /// Colour `SRC_ALPHA, ONE`; alpha kept: FrameXML `alphaMode="ADD"` (`AddUiMaterial`).
    AddAlpha,
    /// Colour `DST_COLOR, ZERO`; alpha kept: the M2/WMO Mod blend.
    Modulate,
    /// Colour `DST_COLOR, SRC_COLOR`; alpha kept: the Mod2x blend.
    Modulate2x,
}

impl Blend {
    pub fn of(alpha: GfxAlpha) -> Self {
        match alpha {
            GfxAlpha::Opaque | GfxAlpha::Mask(_) => Self::Replace,
            GfxAlpha::Blend => Self::Alpha,
            GfxAlpha::Premultiplied | GfxAlpha::Add => Self::Premultiplied,
            GfxAlpha::Multiply => Self::Multiply,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PipelineKey {
    pub program: &'static str,
    pub blend: Blend,
    pub color_write: bool,
    /// The face culled, after the camera's `invert_culling`.
    pub cull: Option<Face>,
    pub depth_write: bool,
    pub depth_test: bool,
    /// `Always` in place of `GreaterEqual`.
    pub depth_always: bool,
    /// `Greater` in place of `GreaterEqual`.
    pub depth_strict: bool,
    /// The rasterizer depth bias, wgpu's `DepthBiasState`: the constant and the slope scale's
    /// bits (`f32` is not `Hash`).
    pub depth_bias: (i32, u32),
    pub primitive: GfxPrimitiveType,
    /// The attachments it draws into.
    pub target: crate::target::TargetClass,
}

/// A program's input layout: stream `i` of the attribute state is input `i`.
struct Layout {
    layout: GfxInputLayout,
}

pub struct Pipelines {
    device: GfxDevice,
    pipelines: HashMap<PipelineKey, GfxPipeline>,
    layouts: HashMap<&'static str, Layout>,
    blend: HashMap<(Blend, bool), ffi::GfxBlendState>,
    depth: HashMap<(bool, bool, GfxCompareFunction), ffi::GfxDepthStencilState>,
    raster: HashMap<(Option<Face>, (i32, u32)), ffi::GfxRasterizerState>,
}

impl Pipelines {
    pub fn new(device: GfxDevice) -> Self {
        Self {
            device,
            pipelines: HashMap::new(),
            layouts: HashMap::new(),
            blend: HashMap::new(),
            depth: HashMap::new(),
            raster: HashMap::new(),
        }
    }

    /// The input layout of `program` on `shader_state`.
    pub fn layout(
        &mut self,
        program: &GfxProgram,
        shader_state: GfxShaderState,
    ) -> Option<GfxInputLayout> {
        if let Some(l) = self.layouts.get(program.name) {
            return Some(l.layout);
        }
        let mut binds = Vec::with_capacity(program.inputs.len());
        for (i, input) in program.inputs.iter().enumerate() {
            let format = input.attribute.format;
            binds.push(ffi::GfxInputLayoutBind {
                buffer: i as u32,
                format: input.read_as.or_else(|| vertex_format(format))?,
                stride: format.size() as u32,
                offset: 0,
                step_mode: GfxStepMode::Vertex,
            });
        }
        let layout = create_layout(self.device, shader_state, &binds)?;
        self.layouts.insert(program.name, Layout { layout });
        Some(layout)
    }

    /// The pipeline of `key`, made against `target` (null: the window), a framebuffer of
    /// `key.target`'s class.
    pub fn get(
        &mut self,
        key: PipelineKey,
        shader_state: GfxShaderState,
        layout: GfxInputLayout,
        target: GfxFramebuffer,
    ) -> Option<GfxPipeline> {
        if let Some(p) = self.pipelines.get(&key) {
            return Some(*p);
        }
        let device = self.device;
        let blend = *self
            .blend
            .entry((key.blend, key.color_write))
            .or_insert_with(|| blend_state(device, key.blend, key.color_write));
        let compare = match (key.depth_always, key.depth_strict) {
            (true, _) => GfxCompareFunction::Always,
            (false, true) => GfxCompareFunction::Greater,
            (false, false) => GfxCompareFunction::GEqual,
        };
        let depth = *self
            .depth
            .entry((key.depth_test, key.depth_write, compare))
            .or_insert_with(|| depth_state(device, key.depth_test, key.depth_write, compare));
        let raster = *self
            .raster
            .entry((key.cull, key.depth_bias))
            .or_insert_with(|| {
                let (constant, slope) = key.depth_bias;
                raster_state_biased(device, key.cull, constant, f32::from_bits(slope))
            });
        if blend.is_null() || depth.is_null() || raster.is_null() {
            return None;
        }
        let info = ffi::GfxPipelineCreateInfo {
            shader_state,
            rasterizer_state: raster,
            depth_stencil_state: depth,
            blend_state: blend,
            input_layout: layout,
            primitive: key.primitive,
            target_framebuffer: target,
        };
        let mut pipeline: GfxPipeline = ptr::null_mut();
        // SAFETY: every state in `info` is live and made on this device.
        if !unsafe { ffi::gfx_dll_create_pipeline(device, &info, &mut pipeline) } {
            return None;
        }
        self.pipelines.insert(key, pipeline);
        Some(pipeline)
    }

    /// Deletes every pipeline, for a re-made target; states and layouts stay.
    pub fn clear_pipelines(&mut self) {
        for (_, p) in self.pipelines.drain() {
            // SAFETY: made on this device, dropped from the cache here.
            unsafe { ffi::gfx_dll_delete_pipeline(self.device, p) };
        }
    }

    /// Deletes everything; the device must still be alive.
    pub fn clear(&mut self) {
        self.clear_pipelines();
        let device = self.device;
        // SAFETY: each handle was made on `device` and is dropped from its cache here.
        unsafe {
            for (_, l) in self.layouts.drain() {
                ffi::gfx_dll_delete_input_layout(device, l.layout);
            }
            for (_, s) in self.blend.drain() {
                ffi::gfx_dll_delete_blend_state(device, s);
            }
            for (_, s) in self.depth.drain() {
                ffi::gfx_dll_delete_depth_stencil_state(device, s);
            }
            for (_, s) in self.raster.drain() {
                ffi::gfx_dll_delete_rasterizer_state(device, s);
            }
        }
    }
}

impl Drop for Pipelines {
    fn drop(&mut self) {
        self.clear();
    }
}

pub(crate) fn create_layout(
    device: GfxDevice,
    shader_state: GfxShaderState,
    binds: &[ffi::GfxInputLayoutBind],
) -> Option<GfxInputLayout> {
    let info = ffi::GfxInputLayoutCreateInfo {
        shader_state,
        binds: binds.as_ptr(),
        count: binds.len() as u32,
    };
    let mut layout: GfxInputLayout = ptr::null_mut();
    // SAFETY: `info` and `binds` are live for the call.
    unsafe { ffi::gfx_dll_create_input_layout(device, &info, &mut layout) }.then_some(layout)
}

pub(crate) fn blend_state(
    device: GfxDevice,
    blend: Blend,
    color_write: bool,
) -> ffi::GfxBlendState {
    use GfxBlendFunction as F;
    let (enabled, src_color, dst_color, src_alpha, dst_alpha) = match blend {
        Blend::Replace => (false, F::One, F::Zero, F::One, F::Zero),
        Blend::Alpha => (
            true,
            F::SrcAlpha,
            F::OneMinusSrcAlpha,
            F::One,
            F::OneMinusSrcAlpha,
        ),
        Blend::Premultiplied => (
            true,
            F::One,
            F::OneMinusSrcAlpha,
            F::One,
            F::OneMinusSrcAlpha,
        ),
        Blend::Multiply => (
            true,
            F::DstColor,
            F::OneMinusSrcAlpha,
            F::One,
            F::OneMinusSrcAlpha,
        ),
        Blend::Add => (true, F::One, F::One, F::Zero, F::One),
        Blend::AddAlpha => (true, F::SrcAlpha, F::One, F::Zero, F::One),
        Blend::Modulate => (true, F::DstColor, F::Zero, F::Zero, F::One),
        Blend::Modulate2x => (true, F::DstColor, F::SrcColor, F::Zero, F::One),
    };
    let info = ffi::GfxBlendStateCreateInfo {
        enabled,
        src_color,
        src_alpha,
        dst_color,
        dst_alpha,
        equation_color: GfxBlendEquation::Add,
        equation_alpha: GfxBlendEquation::Add,
        color_mask: if color_write {
            color_mask::RGBA
        } else {
            color_mask::NONE
        },
        constant_color: [0.0; 4],
    };
    let mut state = ptr::null_mut();
    // SAFETY: `info` is live for the call.
    unsafe { ffi::gfx_dll_create_blend_state(device, &info, &mut state) };
    state
}

pub(crate) fn depth_state(
    device: GfxDevice,
    test: bool,
    write: bool,
    compare: GfxCompareFunction,
) -> ffi::GfxDepthStencilState {
    let stencil = GfxStencilState {
        fail: GfxStencilOperation::Keep,
        pass: GfxStencilOperation::Keep,
        zfail: GfxStencilOperation::Keep,
        compare_func: GfxCompareFunction::Always,
        compare_mask: 0,
        write_mask: 0,
        reference: 0,
    };
    let info = ffi::GfxDepthStencilStateCreateInfo {
        depth_compare: compare,
        depth_write: write,
        depth_test: test,
        stencil_enabled: false,
        front: stencil,
        back: stencil,
    };
    let mut state = ptr::null_mut();
    // SAFETY: `info` is live for the call.
    unsafe { ffi::gfx_dll_create_depth_stencil_state(device, &info, &mut state) };
    state
}

pub(crate) fn raster_state(device: GfxDevice, cull: Option<Face>) -> ffi::GfxRasterizerState {
    raster_state_biased(device, cull, 0, 0.0)
}

/// No culling, the scissor test on: a pass clipped to a rect on every device.
pub(crate) fn raster_state_scissored(device: GfxDevice) -> ffi::GfxRasterizerState {
    let info = ffi::GfxRasterizerStateCreateInfo {
        fill_mode: GfxFillMode::Solid,
        cull_mode: GfxCullMode::None,
        front_face: GfxFrontFace::Ccw,
        scissor: true,
        depth_clamp: false,
        multisample: false,
    };
    let mut state = ptr::null_mut();
    // SAFETY: `info` is live for the call.
    unsafe { ffi::gfx_dll_create_rasterizer_state(device, &info, &mut state) };
    state
}

/// A rasterizer state with wgpu's depth bias (`DepthBiasState`, unclamped): `constant` in the
/// depth format's minimal resolvable difference, as wgpu passes it to every backend.
pub(crate) fn raster_state_biased(
    device: GfxDevice,
    cull: Option<Face>,
    constant: i32,
    slope: f32,
) -> ffi::GfxRasterizerState {
    let info = ffi::GfxRasterizerStateCreateInfo {
        fill_mode: GfxFillMode::Solid,
        cull_mode: match cull {
            None => GfxCullMode::None,
            Some(Face::Front) => GfxCullMode::Front,
            Some(Face::Back) => GfxCullMode::Back,
        },
        front_face: GfxFrontFace::Ccw,
        scissor: false,
        depth_clamp: false,
        multisample: false,
    };
    let mut state = ptr::null_mut();
    // SAFETY: `info` is live for the call.
    unsafe {
        if constant == 0 && slope == 0.0 {
            ffi::gfx_dll_create_rasterizer_state(device, &info, &mut state);
        } else {
            ffi::gfx_dll_create_rasterizer_state_biased(
                device,
                &info,
                constant as f32,
                slope,
                0.0,
                &mut state,
            );
        }
    }
    state
}
