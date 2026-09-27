//! The sky family on the gfx renderer: the dome (`SkyMaterial`), the celestial discs and glares
//! (`CelestialMaterial`), the stars (`StarMaterial`) and the cloud dome (`CloudMaterial`), each a
//! program `<name>.{vs,fs}.gfxs` porting its WGSL over the shared `sky_vertex.wgsl`, whose far-depth
//! pin every vertex stage repeats. Every `specialize` zeroes the raster bias
//! (`sky_pipeline_state`), so the base's `depth_bias` stays a sort rung.

use bevy::prelude::*;

use benilla_gfx::material::{standard_rows, MAX_PARAMS};
use benilla_gfx::meshes::VertexInput;
use benilla_gfx::{GfxMaterialDesc, GfxProgram, GfxTextureSlot};

use crate::clouds::CloudMaterial;
use crate::sky::SkyMaterial;
use crate::sun::{CelestialMaterial, StarMaterial};

const POSITION: VertexInput = VertexInput::new(Mesh::ATTRIBUTE_POSITION, [0.0; 4]);
const UV: VertexInput = VertexInput::new(Mesh::ATTRIBUTE_UV_0, [0.0; 4]);
const COLOR: VertexInput = VertexInput::new(Mesh::ATTRIBUTE_COLOR, [1.0; 4]);

pub const SKY: GfxProgram = GfxProgram {
    name: "sky",
    inputs: &[POSITION],
    params: 7,
    samplers: 0,
};

pub const CELESTIAL: GfxProgram = GfxProgram {
    name: "celestial",
    inputs: &[POSITION, UV, COLOR],
    params: 5,
    samplers: 1,
};

pub const STAR: GfxProgram = GfxProgram {
    name: "star",
    inputs: &[POSITION, UV, COLOR],
    params: 3,
    samplers: 1,
};

pub const CLOUD: GfxProgram = GfxProgram {
    name: "cloud",
    inputs: &[POSITION, UV, COLOR],
    params: 0,
    samplers: 1,
};

const WHITE: [GfxTextureSlot; 4] = [GfxTextureSlot::White; 4];

/// The dome: rows 0-6 the extension's stops, fog and warp in field order; opaque without depth
/// writes, as `SkyExt::specialize` sets.
pub fn describe_sky(m: &SkyMaterial) -> GfxMaterialDesc {
    let (_, _, alpha, mut state) = standard_rows(&m.base);
    state.depth_write = Some(false);
    state.raster_bias = 0;
    let e = &m.extension;
    let mut params = [[0.0; 4]; MAX_PARAMS];
    for (row, v) in params
        .iter_mut()
        .zip([e.sky0, e.sky1, e.sky2, e.sky3, e.sky4, e.fog, e.warp])
    {
        *row = v.to_array();
    }
    GfxMaterialDesc {
        program: SKY,
        textures: WHITE,
        params,
        alpha,
        cull: m.base.cull_mode,
        state,
    }
}

/// A disc or glare: the base's rows 0-2 and texture, rows 3-4 the extension's `fade` and `span`.
pub fn describe_celestial(m: &CelestialMaterial) -> GfxMaterialDesc {
    let (rows, texture, alpha, mut state) = standard_rows(&m.base);
    state.raster_bias = 0;
    let mut params = [[0.0; 4]; MAX_PARAMS];
    params[..3].copy_from_slice(&rows);
    params[3] = m.extension.fade.to_array();
    params[4] = m.extension.span.to_array();
    GfxMaterialDesc {
        program: CELESTIAL,
        textures: [texture, WHITE[1], WHITE[2], WHITE[3]],
        params,
        alpha,
        cull: m.base.cull_mode,
        state,
    }
}

/// The star patches: the base's rows 0-2 and texture.
pub fn describe_star(m: &StarMaterial) -> GfxMaterialDesc {
    let (rows, texture, alpha, mut state) = standard_rows(&m.base);
    state.raster_bias = 0;
    let mut params = [[0.0; 4]; MAX_PARAMS];
    params[..3].copy_from_slice(&rows);
    GfxMaterialDesc {
        program: STAR,
        textures: [texture, WHITE[1], WHITE[2], WHITE[3]],
        params,
        alpha,
        cull: m.base.cull_mode,
        state,
    }
}

/// The cloud dome: the CPU-built texels through their own sampler (binding 101).
pub fn describe_cloud(m: &CloudMaterial) -> GfxMaterialDesc {
    let (_, _, alpha, mut state) = standard_rows(&m.base);
    state.raster_bias = 0;
    GfxMaterialDesc {
        program: CLOUD,
        textures: [
            GfxTextureSlot::Image(m.extension.texels.id()),
            WHITE[1],
            WHITE[2],
            WHITE[3],
        ],
        params: [[0.0; 4]; MAX_PARAMS],
        alpha,
        cull: m.base.cull_mode,
        state,
    }
}
