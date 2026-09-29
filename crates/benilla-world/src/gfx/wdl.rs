//! `WdlMaterial` (the WDL horizon band) on the gfx renderer: the program `wdl.{vs,fs}.gfxs`, a port
//! of `wdl.wgsl` with its fragment depth clamp taken per vertex (see the vertex stage).

use bevy::prelude::*;

use benilla_assets::materials::WdlMaterial;
use benilla_gfx::material::{standard_rows, MAX_PARAMS};
use benilla_gfx::meshes::VertexInput;
use benilla_gfx::{GfxMaterialDesc, GfxProgram, GfxTextureSlot};

pub const WDL: GfxProgram = GfxProgram {
    name: "wdl",
    inputs: &[VertexInput::new(Mesh::ATTRIBUTE_POSITION, [0.0; 4])],
    params: 0,
    samplers: 1,
};

/// The band: no parameter rows, the light buffer's data texture; alpha, cull and states are the
/// base `StandardMaterial`'s (opaque, both faces).
pub fn describe(m: &WdlMaterial) -> GfxMaterialDesc {
    let (_, _, alpha, state) = standard_rows(&m.base);
    GfxMaterialDesc {
        program: WDL,
        textures: [
            GfxTextureSlot::Data(m.extension.light_buf),
            GfxTextureSlot::White,
            GfxTextureSlot::White,
            GfxTextureSlot::White,
        ],
        params: [[0.0; 4]; MAX_PARAMS],
        alpha,
        cull: m.base.cull_mode,
        state,
    }
}
