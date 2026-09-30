mod fallback_image;
mod gpu_image;
mod manual_texture_view;
mod texture_attachment;
mod texture_cache;

use bevy_image::{CompressedImageFormatSupport, CompressedImageFormats, ImageLoader};
pub use fallback_image::*;
pub use gpu_image::*;
pub use manual_texture_view::*;
pub use texture_attachment::*;
pub use texture_cache::*;

use crate::extract_resource::ExtractResourcePlugin;
use bevy_app::{App, Plugin};
use bevy_asset::AssetApp;
use tracing::warn;

#[derive(Default)]
pub struct TexturePlugin;

impl Plugin for TexturePlugin {
    fn build(&self, app: &mut App) {
        // `RenderAssetPlugin::<GpuImage>` and the texture cache only reached the RenderApp.
        app.add_plugins(ExtractResourcePlugin::<ManualTextureViews>::default())
            .init_resource::<ManualTextureViews>();
    }

    fn finish(&self, app: &mut App) {
        if !ImageLoader::SUPPORTED_FORMATS.is_empty() {
            let supported_compressed_formats = if let Some(resource) =
                app.world().get_resource::<CompressedImageFormatSupport>()
            {
                resource.0
            } else {
                warn!("CompressedImageFormatSupport resource not found. It should either be initialized in finish() of \
                       RenderPlugin, or manually if not using the RenderPlugin or the WGPU backend.");
                CompressedImageFormats::NONE
            };

            app.register_asset_loader(ImageLoader::new(supported_compressed_formats));
        }
    }
}
