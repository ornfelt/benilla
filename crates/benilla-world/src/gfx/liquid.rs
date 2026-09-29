//! `LiquidMaterial` (ADT, WMO, magma and slime liquid) on the gfx renderer: the program
//! `liquid.{vs,fs}.gfxs`, a port of `liquid.wgsl`.

use bevy::prelude::*;

use benilla_assets::materials::LiquidMaterial;
use benilla_gfx::material::{standard_rows, MAX_PARAMS};
use benilla_gfx::meshes::VertexInput;
use benilla_gfx::{GfxMaterialDesc, GfxProgram, GfxTextureSlot};

/// The program's inputs: bevy_pbr's forward vertex as the WGSL reads it; UV1.x carries the depth
/// coordinate, and a mesh without colours reads white (the WGSL's `VERTEX_COLORS` arm).
pub const LIQUID: GfxProgram = GfxProgram {
    name: "liquid",
    inputs: &[
        VertexInput::new(Mesh::ATTRIBUTE_POSITION, [0.0; 4]),
        VertexInput::new(Mesh::ATTRIBUTE_NORMAL, [0.0; 4]),
        VertexInput::new(Mesh::ATTRIBUTE_UV_0, [0.0; 4]),
        VertexInput::new(Mesh::ATTRIBUTE_UV_1, [0.0; 4]),
        VertexInput::new(Mesh::ATTRIBUTE_COLOR, [1.0; 4]),
    ],
    params: 3,
    samplers: 2,
};

/// One liquid: rows 0-2 the extension's `kind`, `path` and `anim`; the frames array through its
/// own sampler (binding 101), then the light buffer's data texture. Alpha, cull and the sort rung
/// are the base `StandardMaterial`'s; `LiquidExt::specialize` zeroes its raster bias, keeping the
/// rung a sort key.
pub fn describe(m: &LiquidMaterial) -> GfxMaterialDesc {
    let (_, _, alpha, mut state) = standard_rows(&m.base);
    state.raster_bias = 0;
    let e = &m.extension;
    let mut params = [[0.0; 4]; MAX_PARAMS];
    params[0] = e.kind.to_array();
    params[1] = e.path.to_array();
    params[2] = e.anim.to_array();
    GfxMaterialDesc {
        program: LIQUID,
        textures: [
            GfxTextureSlot::Image(e.frames.id()),
            GfxTextureSlot::Data(e.light_buf),
            GfxTextureSlot::White,
            GfxTextureSlot::White,
        ],
        params,
        alpha,
        cull: m.base.cull_mode,
        state,
    }
}
