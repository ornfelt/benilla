mod bind_group;
mod bind_group_layout;
mod bind_group_layout_entries;
mod bindless;
mod buffer;
mod pipeline;
mod pipeline_specializer;
pub mod resource_macros;
mod texture;

pub use bind_group::*;
pub use bind_group_layout::*;
pub use bind_group_layout_entries::*;
pub use bindless::*;
pub use buffer::*;
pub use pipeline::*;
pub use pipeline_specializer::*;
pub use texture::*;

// TODO: decide where re-exports should go
pub use crate::wgpu::{
    util::{BufferInitDescriptor, TextureDataOrder},
    BindGroupLayoutEntry, BindingType, BlendComponent, BlendFactor, BlendOperation, BlendState,
    BufferBindingType, BufferDescriptor, BufferUsages, ColorWrites, CompareFunction,
    DepthBiasState, Extent3d, Face, FilterMode, LoadOp, MultisampleState, Operations,
    PrimitiveTopology, RenderPassColorAttachment, RenderPassDepthStencilAttachment,
    SamplerBindingType, ShaderStages, StorageTextureAccess, StoreOp, TextureDescriptor,
    TextureDimension, TextureFormat, TextureSampleType, TextureUsages, TextureViewDescriptor,
    TextureViewDimension, VertexFormat,
};

pub mod encase {
    pub use bevy_encase_derive::ShaderType;
    pub use encase::*;
}

pub use self::encase::{ShaderSize, ShaderType};
