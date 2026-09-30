use crate::{
    render_asset::RenderAsset,
    render_resource::{Sampler, Texture, TextureView},
    wgpu::{Extent3d, TextureFormat},
};
use bevy_image::Image;

/// The GPU-representation of an [`Image`].
/// Consists of the [`Texture`], its [`TextureView`] and the corresponding [`Sampler`], and the texture's size.
#[derive(Debug, Clone)]
pub struct GpuImage {
    pub texture: Texture,
    pub texture_view: TextureView,
    pub texture_format: TextureFormat,
    pub texture_view_format: Option<TextureFormat>,
    pub sampler: Sampler,
    pub size: Extent3d,
    pub mip_level_count: u32,
    pub had_data: bool,
}

impl RenderAsset for GpuImage {
    type SourceAsset = Image;
}
