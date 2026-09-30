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
pub use wgpu::{
    util::{
        BufferInitDescriptor, DispatchIndirectArgs, DrawIndexedIndirectArgs, DrawIndirectArgs,
        TextureDataOrder,
    },
    AccelerationStructureFlags, AccelerationStructureGeometryFlags,
    AccelerationStructureUpdateMode, AdapterInfo as WgpuAdapterInfo, AddressMode, AstcBlock,
    AstcChannel, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutEntry, BindingResource,
    BindingType, Blas, BlasBuildEntry, BlasGeometries, BlasGeometrySizeDescriptors,
    BlasTriangleGeometry, BlasTriangleGeometrySizeDescriptor, BlendComponent, BlendFactor,
    BlendOperation, BlendState, BufferAddress, BufferAsyncError, BufferBinding, BufferBindingType,
    BufferDescriptor, BufferSize, BufferUsages, ColorTargetState, ColorWrites, CommandEncoder,
    CommandEncoderDescriptor, CompareFunction, ComputePass, ComputePassDescriptor,
    ComputePipelineDescriptor as RawComputePipelineDescriptor, CreateBlasDescriptor,
    CreateTlasDescriptor, DepthBiasState, DepthStencilState, DownlevelFlags, Extent3d, Face,
    Features as WgpuFeatures, FilterMode, FragmentState as RawFragmentState, FrontFace,
    ImageSubresourceRange, IndexFormat, Limits as WgpuLimits, LoadOp, MapMode, MultisampleState,
    Operations, Origin3d, PipelineCompilationOptions, PipelineLayout, PipelineLayoutDescriptor,
    PollType, PolygonMode, PrimitiveState, PrimitiveTopology, PushConstantRange,
    RenderPassColorAttachment, RenderPassDepthStencilAttachment, RenderPassDescriptor,
    RenderPipelineDescriptor as RawRenderPipelineDescriptor, Sampler as WgpuSampler,
    SamplerBindingType, SamplerDescriptor, ShaderModule, ShaderModuleDescriptor, ShaderSource,
    ShaderStages, StencilFaceState, StencilOperation, StencilState, StorageTextureAccess, StoreOp,
    TexelCopyBufferInfo, TexelCopyBufferLayout, TexelCopyTextureInfo, TextureAspect,
    TextureDescriptor, TextureDimension, TextureFormat, TextureFormatFeatureFlags,
    TextureFormatFeatures, TextureSampleType, TextureUsages, TextureView as WgpuTextureView,
    TextureViewDescriptor, TextureViewDimension, Tlas, TlasInstance, VertexAttribute,
    VertexBufferLayout as RawVertexBufferLayout, VertexFormat, VertexState as RawVertexState,
    VertexStepMode, COPY_BUFFER_ALIGNMENT,
};

pub mod encase {
    pub use bevy_encase_derive::ShaderType;
    pub use encase::*;
}

pub use self::encase::{ShaderSize, ShaderType};

pub use naga::ShaderStage;
