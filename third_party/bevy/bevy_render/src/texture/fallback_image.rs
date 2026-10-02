use crate::texture::GpuImage;
use bevy_ecs::resource::Resource;

/// A [`RenderApp`](crate::RenderApp) resource that contains the default "fallback image",
/// which can be used in situations where an image was not explicitly defined. The most common
/// use case is [`AsBindGroup`](crate::render_resource::AsBindGroup) implementations (such as materials) that support optional textures.
///
/// Defaults to a 1x1 fully opaque white texture, (1.0, 1.0, 1.0, 1.0) which makes multiplying
/// it with other colors a no-op.
#[derive(Resource)]
pub struct FallbackImage {
    /// Fallback image for [`TextureViewDimension::D1`](crate::render_resource::TextureViewDimension::D1).
    pub d1: GpuImage,
    /// Fallback image for [`TextureViewDimension::D2`](crate::render_resource::TextureViewDimension::D2).
    pub d2: GpuImage,
    /// Fallback image for [`TextureViewDimension::D2Array`](crate::render_resource::TextureViewDimension::D2Array).
    pub d2_array: GpuImage,
    /// Fallback image for [`TextureViewDimension::Cube`](crate::render_resource::TextureViewDimension::Cube).
    pub cube: GpuImage,
    /// Fallback image for [`TextureViewDimension::CubeArray`](crate::render_resource::TextureViewDimension::CubeArray).
    pub cube_array: GpuImage,
    /// Fallback image for [`TextureViewDimension::D3`](crate::render_resource::TextureViewDimension::D3).
    pub d3: GpuImage,
}
