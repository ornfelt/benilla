use bevy_app::prelude::*;
use bevy_asset::{Assets, Handle, RenderAssetUsages};
use bevy_camera::Camera;
use bevy_ecs::prelude::*;
use bevy_image::{CompressedImageFormats, Image, ImageSampler, ImageType};
use bevy_render::{
    extract_component::{ExtractComponent, ExtractComponentPlugin},
    extract_resource::{ExtractResource, ExtractResourcePlugin},
};
use bevy_utils::default;

/// 3D LUT (look up table) textures used for tonemapping
#[derive(Resource, Clone, ExtractResource)]
pub struct TonemappingLuts {
    pub blender_filmic: Handle<Image>,
    pub agx: Handle<Image>,
    pub tony_mc_mapface: Handle<Image>,
}

pub struct TonemappingPlugin;

impl Plugin for TonemappingPlugin {
    fn build(&self, app: &mut App) {
        if !app.world().is_resource_added::<TonemappingLuts>() {
            let mut images = app.world_mut().resource_mut::<Assets<Image>>();

            let tonemapping_luts = {
                TonemappingLuts {
                    blender_filmic: images.add(setup_tonemapping_lut_image(
                        include_bytes!("luts/Blender_-11_12.ktx2"),
                        ImageType::Extension("ktx2"),
                    )),
                    agx: images.add(setup_tonemapping_lut_image(
                        include_bytes!("luts/AgX-default_contrast.ktx2"),
                        ImageType::Extension("ktx2"),
                    )),
                    tony_mc_mapface: images.add(setup_tonemapping_lut_image(
                        include_bytes!("luts/tony_mc_mapface.ktx2"),
                        ImageType::Extension("ktx2"),
                    )),
                }
            };

            app.insert_resource(tonemapping_luts);
        }

        app.add_plugins(ExtractResourcePlugin::<TonemappingLuts>::default());

        app.add_plugins((
            ExtractComponentPlugin::<Tonemapping>::default(),
            ExtractComponentPlugin::<DebandDither>::default(),
        ));
    }
}

/// Optionally enables a tonemapping shader that attempts to map linear input stimulus into a perceptually uniform image for a given [`Camera`] entity.
#[derive(Component, Debug, Hash, Clone, Copy, Default, ExtractComponent, PartialEq, Eq)]
#[extract_component_filter(With<Camera>)]
pub enum Tonemapping {
    /// Bypass tonemapping.
    None,
    /// Suffers from lots hue shifting, brights don't desaturate naturally.
    /// Bright primaries and secondaries don't desaturate at all.
    Reinhard,
    /// Suffers from hue shifting. Brights don't desaturate much at all across the spectrum.
    ReinhardLuminance,
    /// Same base implementation that Godot 4.0 uses for Tonemap ACES.
    /// <https://github.com/TheRealMJP/BakingLab/blob/master/BakingLab/ACES.hlsl>
    /// Not neutral, has a very specific aesthetic, intentional and dramatic hue shifting.
    /// Bright greens and reds turn orange. Bright blues turn magenta.
    /// Significantly increased contrast. Brights desaturate across the spectrum.
    AcesFitted,
    /// By Troy Sobotka
    /// <https://github.com/sobotka/AgX>
    /// Very neutral. Image is somewhat desaturated when compared to other tonemappers.
    /// Little to no hue shifting. Subtle [Abney shifting](https://en.wikipedia.org/wiki/Abney_effect).
    /// NOTE: Requires the `tonemapping_luts` cargo feature.
    AgX,
    /// By Tomasz Stachowiak
    /// Has little hue shifting in the darks and mids, but lots in the brights. Brights desaturate across the spectrum.
    /// Is sort of between Reinhard and `ReinhardLuminance`. Conceptually similar to reinhard-jodie.
    /// Designed as a compromise if you want e.g. decent skin tones in low light, but can't afford to re-do your
    /// VFX to look good without hue shifting.
    SomewhatBoringDisplayTransform,
    /// Current Bevy default.
    /// By Tomasz Stachowiak
    /// <https://github.com/h3r2tic/tony-mc-mapface>
    /// Very neutral. Subtle but intentional hue shifting. Brights desaturate across the spectrum.
    /// Comment from author:
    /// Tony is a display transform intended for real-time applications such as games.
    /// It is intentionally boring, does not increase contrast or saturation, and stays close to the
    /// input stimulus where compression isn't necessary.
    /// Brightness-equivalent luminance of the input stimulus is compressed. The non-linearity resembles Reinhard.
    /// Color hues are preserved during compression, except for a deliberate [Bezold–Brücke shift](https://en.wikipedia.org/wiki/Bezold%E2%80%93Br%C3%BCcke_shift).
    /// To avoid posterization, selective desaturation is employed, with care to avoid the [Abney effect](https://en.wikipedia.org/wiki/Abney_effect).
    /// NOTE: Requires the `tonemapping_luts` cargo feature.
    #[default]
    TonyMcMapface,
    /// Default Filmic Display Transform from blender.
    /// Somewhat neutral. Suffers from hue shifting. Brights desaturate across the spectrum.
    /// NOTE: Requires the `tonemapping_luts` cargo feature.
    BlenderFilmic,
}

/// Enables a debanding shader that applies dithering to mitigate color banding in the final image for a given [`Camera`] entity.
#[derive(Component, Debug, Hash, Clone, Copy, Default, ExtractComponent, PartialEq, Eq)]
#[extract_component_filter(With<Camera>)]
pub enum DebandDither {
    #[default]
    Disabled,
    Enabled,
}

fn setup_tonemapping_lut_image(bytes: &[u8], image_type: ImageType) -> Image {
    let image_sampler = ImageSampler::Descriptor(bevy_image::ImageSamplerDescriptor {
        label: Some("Tonemapping LUT sampler".to_string()),
        address_mode_u: bevy_image::ImageAddressMode::ClampToEdge,
        address_mode_v: bevy_image::ImageAddressMode::ClampToEdge,
        address_mode_w: bevy_image::ImageAddressMode::ClampToEdge,
        mag_filter: bevy_image::ImageFilterMode::Linear,
        min_filter: bevy_image::ImageFilterMode::Linear,
        mipmap_filter: bevy_image::ImageFilterMode::Linear,
        ..default()
    });
    Image::from_buffer(
        bytes,
        image_type,
        CompressedImageFormats::NONE,
        false,
        image_sampler,
        RenderAssetUsages::RENDER_WORLD,
    )
    .unwrap()
}
