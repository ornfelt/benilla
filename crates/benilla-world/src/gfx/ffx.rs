//! FFXGlow on the gfx renderer: each camera's [`FfxGlow`] becomes the [`GfxFfxGlow`] combine
//! uniform `benilla-gfx`'s post pass reads, from the same live state the render graph's node reads
//! (`ffx_glow::live_combine`); an armed underwater warp adds the GlowWave LUT, as an image. Not
//! drawn yet: the UI camera's backdrop claim (the world view always runs its own combine).

use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use benilla_gfx::GfxFfxGlow;

use crate::ffx_glow::{wave_lut_texels, FfxGlow, WAVE_LUT_EDGE};

/// The GlowWave LUT as an image: `Rg8Unorm`, linear and repeating, as the render graph's.
#[derive(Resource)]
struct WaveLut(Handle<Image>);

fn wave_lut(images: &mut Assets<Image>) -> Handle<Image> {
    let mut image = Image::new(
        Extent3d {
            width: WAVE_LUT_EDGE,
            height: WAVE_LUT_EDGE,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        wave_lut_texels(),
        TextureFormat::Rg8Unorm,
        RenderAssetUsages::default(),
    );
    // Linear, so the 128-texel sine samples as a smooth wave; repeat, past 1.0.
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        ..default()
    });
    images.add(image)
}

/// `GfxRenderSystems::Pack`: every glowing camera's uniform for this frame.
pub(crate) fn sync(world: &mut World) {
    let mut cameras = world.query::<(Entity, &FfxGlow)>();
    let views: Vec<(Entity, FfxGlow)> = cameras.iter(world).map(|(e, g)| (e, *g)).collect();
    for (entity, glow) in views {
        let (u, wave) = crate::ffx_glow::gfx_live_combine(world, &glow);
        let wave_lut = wave.then(|| {
            if let Some(lut) = world.get_resource::<WaveLut>() {
                return lut.0.id();
            }
            let handle = wave_lut(&mut world.resource_mut::<Assets<Image>>());
            let id = handle.id();
            world.insert_resource(WaveLut(handle));
            id
        });
        let uniform = GfxFfxGlow {
            lane: [u[0], u[1], u[2], u[3]],
            wave: [u[4], u[5], u[6], u[7]],
            wave_lut,
        };
        let mut e = world.entity_mut(entity);
        if e.get::<GfxFfxGlow>() != Some(&uniform) {
            e.insert(uniform);
        }
    }
}
