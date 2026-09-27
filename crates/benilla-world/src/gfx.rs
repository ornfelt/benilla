//! The world's materials on the gfx renderer (`benilla-gfx`, the `gfx` feature). Each
//! `ExtendedMaterial` draws through its `StandardMaterial` base until its extension's WGSL is
//! ported to a gfx program.

use bevy::prelude::*;

use benilla_assets::materials::{LiquidMaterial, TerrainMaterial, WdlMaterial, WowModelMaterial};
use benilla_gfx::material::extended_base;
use benilla_gfx::GfxMaterialPlugin;

use crate::clouds::CloudMaterial;
use crate::sky::SkyMaterial;
use crate::sun::{CelestialMaterial, StarMaterial};

pub struct GfxWorldMaterials;

impl Plugin for GfxWorldMaterials {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            GfxMaterialPlugin::<TerrainMaterial>::new(extended_base),
            GfxMaterialPlugin::<WowModelMaterial>::new(extended_base),
            GfxMaterialPlugin::<WdlMaterial>::new(extended_base),
            GfxMaterialPlugin::<LiquidMaterial>::new(extended_base),
            GfxMaterialPlugin::<SkyMaterial>::new(extended_base),
            GfxMaterialPlugin::<CelestialMaterial>::new(extended_base),
            GfxMaterialPlugin::<StarMaterial>::new(extended_base),
            GfxMaterialPlugin::<CloudMaterial>::new(extended_base),
        ));
    }
}
