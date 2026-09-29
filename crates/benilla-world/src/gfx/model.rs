//! `WowModelMaterial` (the M2, WMO and clutter material) on the gfx renderer: the program
//! `wow_model.{vs,fs}.gfxs`, a port of `wow_model.wgsl`, and the states
//! `WowModelExt::specialize` keys, read off the same fields.

use bevy::prelude::*;

use benilla_assets::materials::WowModelMaterial;
use benilla_assets::{
    ATTRIBUTE_WOW_FADE_SPHERE, ATTRIBUTE_WOW_JOINT_INDEX, ATTRIBUTE_WOW_JOINT_WEIGHT,
    ATTRIBUTE_WOW_MERGED_SLOT,
};
use benilla_gfx::ffi::GfxFormat;
use benilla_gfx::material::{standard_rows, MAX_PARAMS};
use benilla_gfx::meshes::VertexInput;
use benilla_gfx::pipelines::Blend;
use benilla_gfx::{GfxAlpha, GfxMaterialDesc, GfxProgram, GfxTextureSlot};

/// The program's inputs; the shader reads its shader defs off the mask of those a mesh has (bit 3
/// colours, 4 the rig joints, 6 the merged fade sphere, 7 the merged probe slot).
pub const WOW_MODEL: GfxProgram = GfxProgram {
    name: "wow_model",
    inputs: &[
        VertexInput::new(Mesh::ATTRIBUTE_POSITION, [0.0; 4]),
        VertexInput::new(Mesh::ATTRIBUTE_NORMAL, [0.0; 4]),
        VertexInput::new(Mesh::ATTRIBUTE_UV_0, [0.0; 4]),
        VertexInput::new(Mesh::ATTRIBUTE_COLOR, [1.0; 4]),
        VertexInput::new(ATTRIBUTE_WOW_JOINT_INDEX, [0.0; 4]),
        VertexInput::new(ATTRIBUTE_WOW_JOINT_WEIGHT, [0.0; 4]),
        VertexInput::new(ATTRIBUTE_WOW_FADE_SPHERE, [0.0; 4]),
        // A `u32` slot, read as the `int` both shader languages have.
        VertexInput::new(ATTRIBUTE_WOW_MERGED_SLOT, [0.0; 4]).read_as(GfxFormat::R32Sint),
    ],
    params: 10,
    samplers: 2,
};

/// One model material: the base's rows 0-2, row 3 `x` = `WOW_WATER_CLIP` (the blend pass, as
/// `specialize` keys it on `BLEND_ALPHA`), rows 4-9 the extension's uniforms in field order; the
/// texture, then the light buffer's data texture.
pub fn describe(m: &WowModelMaterial) -> GfxMaterialDesc {
    let (rows, texture, alpha, mut state) = standard_rows(&m.base);
    let e = &m.extension;
    let mut params = [[0.0; 4]; MAX_PARAMS];
    params[..3].copy_from_slice(&rows);
    params[3][0] = if alpha == GfxAlpha::Blend { 1.0 } else { 0.0 };
    for (row, v) in params[4..10].iter_mut().zip([
        e.clutter_fade,
        e.model_flags,
        e.sun_scale,
        e.tint,
        e.sidn,
        e.anim_slots,
    ]) {
        *row = v.to_array();
    }

    // `WowModelExt::specialize`, in its order: later keys win.
    let markers = e.clutter_fade.z as u32;
    let sky_depth = markers & 0x2000 != 0;
    // Every M2 batch writes depth, transparent ones too, unless render flag 0x10 clears it.
    state.depth_write = Some(markers & 1 == 0);
    state.depth_always = markers & 2 != 0;
    // The far side's rung and the skybox's bias are sort-only: `specialize` zeroes the raster
    // constant the base packs.
    if markers & 0x800 != 0 || sky_depth {
        state.raster_bias = 0;
    }
    if e.model_flags.y > 0.5 && !sky_depth {
        state.depth_write = Some(true);
    }
    if e.clutter_fade.w > 0.5 {
        state.blend = Some(Blend::Alpha);
    }
    if markers & 4 != 0 {
        state.blend = Some(Blend::Add);
    }
    if markers & 0x200 != 0 {
        // The depth-prime twin: depth only.
        state.blend = Some(Blend::Replace);
        state.color_write = false;
        state.depth_write = Some(true);
    }
    if markers & 0x100 != 0 {
        state.blend = Some(Blend::Modulate2x);
    } else if markers & 0x80 != 0 {
        state.blend = Some(Blend::Modulate);
    }

    GfxMaterialDesc {
        program: WOW_MODEL,
        textures: [
            texture,
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
