//! The world's materials on the gfx renderer (`benilla-gfx`, the `gfx` feature). A material with a
//! ported program draws through it ([`model`], [`terrain`], [`wdl`]); every other `ExtendedMaterial` draws through its
//! `StandardMaterial` base until its extension's WGSL is ported. [`light`] keeps the shared light
//! buffer's data texture, which the ported programs read, and [`ffx`] feeds each camera's FFXGlow
//! combine, the frame's gamma decode.

mod ffx;
mod light;
pub mod model;
pub mod terrain;
pub mod wdl;

use bevy::prelude::*;

use benilla_assets::materials::{LiquidMaterial, TerrainMaterial, WdlMaterial, WowModelMaterial};
use benilla_gfx::material::extended_base;
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
            GfxMaterialPlugin::<LiquidMaterial>::new(extended_base),
            GfxMaterialPlugin::<SkyMaterial>::new(extended_base),
            GfxMaterialPlugin::<CelestialMaterial>::new(extended_base),
            GfxMaterialPlugin::<StarMaterial>::new(extended_base),
            GfxMaterialPlugin::<CloudMaterial>::new(extended_base),
        ))
        .add_systems(
            GfxRender,
            (light::pack, ffx::sync).in_set(GfxRenderSystems::Pack),
        );
    }
}
