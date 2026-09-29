//! `TerrainMaterial` (the ADT splat material) on the gfx renderer: the program
//! `terrain.{vs,fs}.gfxs`, a port of `terrain.wgsl`.

use bevy::prelude::*;

use benilla_assets::materials::TerrainMaterial;
use benilla_gfx::material::{standard_rows, MAX_PARAMS};
use benilla_gfx::meshes::VertexInput;
use benilla_gfx::{GfxMaterialDesc, GfxProgram, GfxTextureSlot};

/// The program's inputs: bevy_pbr's forward vertex as the WGSL reads it; COLOR carries the four
/// layer indices and UV1 the alpha and shadow layers.
pub const TERRAIN: GfxProgram = GfxProgram {
    name: "terrain",
    inputs: &[
        VertexInput::new(Mesh::ATTRIBUTE_POSITION, [0.0; 4]),
        VertexInput::new(Mesh::ATTRIBUTE_NORMAL, [0.0; 4]),
        VertexInput::new(Mesh::ATTRIBUTE_UV_0, [0.0; 4]),
        VertexInput::new(Mesh::ATTRIBUTE_UV_1, [0.0, -1.0, 0.0, 0.0]),
        VertexInput::new(Mesh::ATTRIBUTE_COLOR, [0.0; 4]),
    ],
    params: 1,
    samplers: 4,
};

/// One tile: row 0 the extension's `params`; the layer, alpha and shadow arrays through the layer
/// array's sampler (binding 105), then the light buffer's data texture. Alpha, cull and states
/// are the base `StandardMaterial`'s.
pub fn describe(m: &TerrainMaterial) -> GfxMaterialDesc {
    let (_, _, alpha, state) = standard_rows(&m.base);
    let e = &m.extension;
    let layers = e.layer_array.id();
    let mut params = [[0.0; 4]; MAX_PARAMS];
    params[0] = e.params.to_array();
    GfxMaterialDesc {
        program: TERRAIN,
        textures: [
            GfxTextureSlot::ImageSampledLike(layers, layers),
            GfxTextureSlot::ImageSampledLike(e.alpha_array.id(), layers),
            GfxTextureSlot::ImageSampledLike(e.shadow_array.id(), layers),
            GfxTextureSlot::Data(e.light_buf),
        ],
        params,
        alpha,
        cull: m.base.cull_mode,
        state,
    }
}
