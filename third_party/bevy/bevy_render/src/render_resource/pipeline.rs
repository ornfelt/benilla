use crate::wgpu::{
    BindGroupLayoutEntry, ColorTargetState, DepthStencilState, MultisampleState, PrimitiveState,
    PushConstantRange,
};
use alloc::borrow::Cow;
use bevy_asset::Handle;
use bevy_mesh::VertexBufferLayout;
use bevy_shader::{Shader, ShaderDefVal};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct BindGroupLayoutDescriptor {
    /// Debug label of the bind group layout descriptor. This will show up in graphics debuggers for easy identification.
    pub label: Cow<'static, str>,
    pub entries: Vec<BindGroupLayoutEntry>,
}

impl BindGroupLayoutDescriptor {
    pub fn new(label: impl Into<Cow<'static, str>>, entries: &[BindGroupLayoutEntry]) -> Self {
        Self {
            label: label.into(),
            entries: entries.into(),
        }
    }
}

/// Describes a render (graphics) pipeline.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct RenderPipelineDescriptor {
    /// Debug label of the pipeline. This will show up in graphics debuggers for easy identification.
    pub label: Option<Cow<'static, str>>,
    /// The layout of bind groups for this pipeline.
    pub layout: Vec<BindGroupLayoutDescriptor>,
    /// The push constant ranges for this pipeline.
    /// Supply an empty vector if the pipeline doesn't use push constants.
    pub push_constant_ranges: Vec<PushConstantRange>,
    /// The compiled vertex stage, its entry point, and the input buffers layout.
    pub vertex: VertexState,
    /// The properties of the pipeline at the primitive assembly and rasterization level.
    pub primitive: PrimitiveState,
    /// The effect of draw calls on the depth and stencil aspects of the output target, if any.
    pub depth_stencil: Option<DepthStencilState>,
    /// The multi-sampling properties of the pipeline.
    pub multisample: MultisampleState,
    /// The compiled fragment stage, its entry point, and the color targets.
    pub fragment: Option<FragmentState>,
    /// Whether to zero-initialize workgroup memory by default. If you're not sure, set this to true.
    /// If this is false, reading from workgroup variables before writing to them will result in garbage values.
    pub zero_initialize_workgroup_memory: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct VertexState {
    /// The compiled shader module for this stage.
    pub shader: Handle<Shader>,
    pub shader_defs: Vec<ShaderDefVal>,
    /// The name of the entry point in the compiled shader, or `None` if the default entry point
    /// is used.
    pub entry_point: Option<Cow<'static, str>>,
    /// The format of any vertex buffers used with this pipeline.
    pub buffers: Vec<VertexBufferLayout>,
}

/// Describes the fragment process in a render pipeline.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct FragmentState {
    /// The compiled shader module for this stage.
    pub shader: Handle<Shader>,
    pub shader_defs: Vec<ShaderDefVal>,
    /// The name of the entry point in the compiled shader, or `None` if the default entry point
    /// is used.
    pub entry_point: Option<Cow<'static, str>>,
    /// The color state of the render targets.
    pub targets: Vec<Option<ColorTargetState>>,
}
