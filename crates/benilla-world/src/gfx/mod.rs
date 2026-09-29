//! The world's materials on the gfx renderer (`benilla-gfx`): each draws through
//! a port of its WGSL ([`model`], [`terrain`], [`wdl`], [`liquid`], [`sky`]), and the effect lane
//! through [`effect`]. [`light`] keeps the shared light buffer's data texture, which the ported
//! programs read, and [`ffx`] feeds each camera's FFXGlow combine, the frame's gamma decode.

mod effect;
mod ffx;
pub(crate) mod light;
pub mod liquid;
pub mod model;
pub mod sky;
pub mod terrain;
pub mod wdl;

use bevy::prelude::*;

use benilla_assets::materials::{LiquidMaterial, TerrainMaterial, WdlMaterial, WowModelMaterial};
use benilla_gfx::{GfxMaterialPlugin, GfxRender, GfxRenderSystems};

use crate::clouds::CloudMaterial;
use crate::sky::SkyMaterial;
use crate::sun::{CelestialMaterial, StarMaterial};

pub struct GfxWorldMaterials;

impl Plugin for GfxWorldMaterials {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            GfxMaterialPlugin::<TerrainMaterial>::new(terrain::describe),
            GfxMaterialPlugin::<WowModelMaterial>::new(model::describe),
            GfxMaterialPlugin::<WdlMaterial>::new(wdl::describe),
            GfxMaterialPlugin::<LiquidMaterial>::new(liquid::describe),
            GfxMaterialPlugin::<SkyMaterial>::new(sky::describe_sky),
            GfxMaterialPlugin::<CelestialMaterial>::new(sky::describe_celestial),
            GfxMaterialPlugin::<StarMaterial>::new(sky::describe_star),
            GfxMaterialPlugin::<CloudMaterial>::new(sky::describe_cloud),
        ))
        .add_systems(Startup, effect::init)
        .add_systems(
            GfxRender,
            (
                (light::pack, ffx::sync).in_set(GfxRenderSystems::Pack),
                effect::collect.in_set(GfxRenderSystems::Collect),
            ),
        );
    }
}
