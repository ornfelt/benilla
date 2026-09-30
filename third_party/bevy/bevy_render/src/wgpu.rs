//! The part of wgpu 27's API that `bevy_render` names, without wgpu.
//!
//! No `RenderApp` exists and no device is ever opened (benilla's gfx backend draws; see
//! `third_party/bevy/BENILLA.md`), so no GPU handle is ever created: each handle type is an
//! uninhabited enum and code holding one is unreachable. The plain-data types are wgpu's own, from
//! wgpu-types, and the descriptor aliases and attachment structs are wgpu's definitions.

pub use wgpu_types::*;

/// wgpu's object label (`wgpu::Label`).
pub type Label<'a> = Option<&'a str>;

macro_rules! handles {
    ($($name:ident),* $(,)?) => {
        $(
            #[doc = concat!("wgpu's `", stringify!($name), "`: a GPU handle, which only a device creates.")]
            #[derive(Clone, Debug, PartialEq, Eq, Hash)]
            pub enum $name {}
        )*
    };
}

handles!(
    Device,
    Buffer,
    Texture,
    TextureView,
    SurfaceTexture,
    Sampler,
    BindGroup,
    BindGroupLayout,
    RenderPipeline,
    ComputePipeline,
);

pub type BufferDescriptor<'a> = wgpu_types::BufferDescriptor<Label<'a>>;
pub type TextureDescriptor<'a> = wgpu_types::TextureDescriptor<Label<'a>, &'a [TextureFormat]>;
pub type TextureViewDescriptor<'a> = wgpu_types::TextureViewDescriptor<Label<'a>>;
pub type SamplerDescriptor<'a> = wgpu_types::SamplerDescriptor<Label<'a>>;

/// A color attachment of a render pass.
#[derive(Clone, Debug)]
pub struct RenderPassColorAttachment<'tex> {
    /// The view to use as an attachment.
    pub view: &'tex TextureView,
    /// The depth slice index of a 3D view. It must not be provided if the view is not 3D.
    pub depth_slice: Option<u32>,
    /// The view that will receive the resolved output if multisampling is used.
    pub resolve_target: Option<&'tex TextureView>,
    /// What operations will be performed on this color attachment.
    pub ops: Operations<Color>,
}

/// The depth and stencil attachment of a render pass.
#[derive(Clone, Debug)]
pub struct RenderPassDepthStencilAttachment<'tex> {
    /// The view to use as an attachment.
    pub view: &'tex TextureView,
    /// What operations will be performed on the depth part of the attachment.
    pub depth_ops: Option<Operations<f32>>,
    /// What operations will be performed on the stencil part of the attachment.
    pub stencil_ops: Option<Operations<u32>>,
}

pub mod util {
    //! wgpu's `util` types that `bevy_render` names.

    pub use wgpu_types::TextureDataOrder;

    /// Describes a buffer created with initial contents.
    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    pub struct BufferInitDescriptor<'a> {
        /// Debug label of a buffer.
        pub label: super::Label<'a>,
        /// Contents of a buffer on creation.
        pub contents: &'a [u8],
        /// Usages of a buffer.
        pub usage: wgpu_types::BufferUsages,
    }
}
