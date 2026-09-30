use bevy_asset::RenderAssetUsages;
use bevy_image::{Image, ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use egui::{TextureFilter, TextureOptions};
use wgpu_types::{Extent3d, TextureDimension, TextureFormat};

/// Plugin systems for the render app.
#[cfg(feature = "render")]
pub mod systems;

pub(crate) fn as_color_image(image: &egui::ImageData) -> egui::ColorImage {
    match image {
        egui::ImageData::Color(image) => (**image).clone(),
    }
}

pub(crate) fn color_image_as_bevy_image(
    egui_image: &egui::ColorImage,
    sampler_descriptor: ImageSampler,
) -> Image {
    let pixels = egui_image
        .pixels
        .iter()
        // We unmultiply Egui textures to premultiply them later in the fragment shader.
        // As user textures loaded as Bevy assets are not premultiplied (and there seems to be no
        // convenient way to convert them to premultiplied ones), we do this with Egui ones.
        .flat_map(|color| color.to_array())
        .collect();

    Image {
        sampler: sampler_descriptor,
        ..Image::new(
            Extent3d {
                width: egui_image.width() as u32,
                height: egui_image.height() as u32,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            pixels,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        )
    }
}

pub(crate) fn texture_options_as_sampler_descriptor(
    options: &TextureOptions,
) -> ImageSamplerDescriptor {
    fn convert_filter(filter: &TextureFilter) -> ImageFilterMode {
        match filter {
            TextureFilter::Nearest => ImageFilterMode::Nearest,
            TextureFilter::Linear => ImageFilterMode::Linear,
        }
    }
    let address_mode = match options.wrap_mode {
        egui::TextureWrapMode::ClampToEdge => ImageAddressMode::ClampToEdge,
        egui::TextureWrapMode::Repeat => ImageAddressMode::Repeat,
        egui::TextureWrapMode::MirroredRepeat => ImageAddressMode::MirrorRepeat,
    };
    ImageSamplerDescriptor {
        mag_filter: convert_filter(&options.magnification),
        min_filter: convert_filter(&options.minification),
        address_mode_u: address_mode,
        address_mode_v: address_mode,
        ..Default::default()
    }
}
