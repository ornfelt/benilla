//! FFXGlow on the gfx renderer: each camera's [`FfxGlow`] becomes the [`GfxFfxGlow`] combine
//! uniform `benilla-gfx`'s post pass reads, from the same live state the render graph's node reads
//! (`ffx_glow::live_combine`). Not drawn yet: the underwater GlowWave warp (the plain combine runs
//! in its place) and the UI camera's backdrop claim (the world view always runs its own combine).

use bevy::prelude::*;

use benilla_gfx::GfxFfxGlow;

use crate::ffx_glow::FfxGlow;

/// `GfxRenderSystems::Pack`: every glowing camera's uniform for this frame.
pub(crate) fn sync(world: &mut World) {
    let mut cameras = world.query::<(Entity, &FfxGlow)>();
    let views: Vec<(Entity, FfxGlow)> = cameras.iter(world).map(|(e, g)| (e, *g)).collect();
    for (entity, glow) in views {
        let (u, wave) = crate::ffx_glow::gfx_live_combine(world, &glow);
        if wave {
            warn_once!(
                "gfx: the underwater GlowWave warp is not drawn yet; the plain combine runs"
            );
        }
        let uniform = GfxFfxGlow {
            lane: [u[0], u[1], u[2], u[3]],
            wave: [u[4], u[5], u[6], u[7]],
        };
        let mut e = world.entity_mut(entity);
        if e.get::<GfxFfxGlow>() != Some(&uniform) {
            e.insert(uniform);
        }
    }
}
