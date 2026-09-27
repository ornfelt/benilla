//! The player-UI lane on the gfx renderer (`benilla-gfx`, the `gfx` feature): [`UiQuadMaterial`]
//! draws through a port of `ui_quad.wgsl` (`ui_quad.{vs,fs}.gfxs`), and the lane camera carries
//! [`GfxUiLane`], its display gamma and the world view its backdrop claims.

use bevy::prelude::*;

use benilla_gfx::meshes::VertexInput;
use benilla_gfx::pipelines::Blend;
use benilla_gfx::{
    GfxAlpha, GfxDrawState, GfxMaterial2dPlugin, GfxMaterialDesc, GfxProgram, GfxRender,
    GfxRenderSystems, GfxTextureSlot, GfxUiLane,
};
use benilla_world::ffx_glow::FfxBackdrop;

use super::UiQuadMaterial;
use crate::ui_gamma::UiGammaLane;

/// `ui_quad.{vs,fs}.gfxs`: POSITION, UV 0 and COLOR at the Mesh2d locations; a mesh without UVs
/// reads 0 and one without colour white, as without `VERTEX_UVS` / `VERTEX_COLORS`.
const UI_QUAD: GfxProgram = GfxProgram {
    name: "ui_quad",
    inputs: &[
        VertexInput::new(Mesh::ATTRIBUTE_POSITION, [0.0; 4]),
        VertexInput::new(Mesh::ATTRIBUTE_UV_0, [0.0; 4]),
        VertexInput::new(Mesh::ATTRIBUTE_COLOR, [1.0; 4]),
    ],
    params: 4,
    samplers: 2,
};

fn flag(on: u32) -> f32 {
    if on != 0 {
        1.0
    } else {
        0.0
    }
}

/// A quad material as the program draws it: rows `modes` (additive, circular, desaturate,
/// premultiplied), `test` (the alpha reference, gamma_texel), `mask_rect`, `uv_clamp`; the one
/// premultiplied blend state of [`UiQuadMaterial::specialize`], no depth, no culling.
fn describe(m: &UiQuadMaterial) -> GfxMaterialDesc {
    let mut params = [[0.0; 4]; benilla_gfx::material::MAX_PARAMS];
    params[0] = [
        flag(m.additive),
        flag(m.circular),
        flag(m.desaturate),
        flag(m.premultiplied),
    ];
    params[1] = [m.alpha_ref, flag(m.gamma_texel), 0.0, 0.0];
    params[2] = m.mask_rect.to_array();
    params[3] = m.uv_clamp.to_array();
    // An unset texture binds bevy's fallback, opaque white.
    let slot = |t: &Option<Handle<Image>>| {
        t.as_ref()
            .map_or(GfxTextureSlot::White, |h| GfxTextureSlot::Image(h.id()))
    };
    GfxMaterialDesc {
        program: UI_QUAD,
        textures: [
            slot(&m.texture),
            slot(&m.mask),
            GfxTextureSlot::White,
            GfxTextureSlot::White,
        ],
        params,
        alpha: GfxAlpha::Premultiplied,
        cull: None,
        state: GfxDrawState {
            blend: Some(Blend::Premultiplied),
            depth_test: false,
            ..default()
        },
    }
}

/// `GfxRenderSystems::Pack`: the lane camera's [`GfxUiLane`] from its [`UiGammaLane`] and
/// [`FfxBackdrop`].
fn sync_lane(
    mut commands: Commands,
    lanes: Query<(Entity, &UiGammaLane, &FfxBackdrop, Option<&GfxUiLane>)>,
) {
    for (entity, lane, backdrop, current) in &lanes {
        let next = GfxUiLane {
            gamma: lane.gamma,
            backdrop: backdrop.source,
        };
        if current != Some(&next) {
            commands.entity(entity).insert(next);
        }
    }
}

pub(crate) struct GfxPlayerUi;

impl Plugin for GfxPlayerUi {
    fn build(&self, app: &mut App) {
        app.add_plugins(GfxMaterial2dPlugin::<UiQuadMaterial>::new(describe))
            .add_systems(GfxRender, sync_lane.in_set(GfxRenderSystems::Pack));
    }
}
