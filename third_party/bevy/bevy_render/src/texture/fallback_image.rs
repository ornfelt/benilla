use crate::{render_resource::*, texture::GpuImage};
use bevy_ecs::resource::Resource;

/// A [`RenderApp`](crate::RenderApp) resource that contains the default "fallback image",
/// which can be used in situations where an image was not explicitly defined. The most common
/// use case is [`AsBindGroup`] implementations (such as materials) that support optional textures.
///
/// Defaults to a 1x1 fully opaque white texture, (1.0, 1.0, 1.0, 1.0) which makes multiplying
/// it with other colors a no-op.
#[derive(Resource)]
pub struct FallbackImage {
    /// Fallback image for [`TextureViewDimension::D1`].
    pub d1: GpuImage,
    /// Fallback image for [`TextureViewDimension::D2`].
    pub d2: GpuImage,
    /// Fallback image for [`TextureViewDimension::D2Array`].
    pub d2_array: GpuImage,
    /// Fallback image for [`TextureViewDimension::Cube`].
    pub cube: GpuImage,
    /// Fallback image for [`TextureViewDimension::CubeArray`].
    pub cube_array: GpuImage,
    /// Fallback image for [`TextureViewDimension::D3`].
    pub d3: GpuImage,
}

impl FallbackImage {
    /// Returns the appropriate fallback image for the given texture dimension.
    pub fn get(&self, texture_dimension: TextureViewDimension) -> &GpuImage {
        match texture_dimension {
            TextureViewDimension::D1 => &self.d1,
            TextureViewDimension::D2 => &self.d2,
            TextureViewDimension::D2Array => &self.d2_array,
            TextureViewDimension::Cube => &self.cube,
            TextureViewDimension::CubeArray => &self.cube_array,
            TextureViewDimension::D3 => &self.d3,
        }
    }
}
