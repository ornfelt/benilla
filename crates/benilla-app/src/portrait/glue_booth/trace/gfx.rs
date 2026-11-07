//! The `gsel` trace's draw lines under gfx: no render world, so the gfx draw keeps each 3D view's
//! phase lists ([`benilla_gfx::GfxPhaseRecord`]) and they are read after the frame's draw, in the
//! same frame, as [`super::report_glue_draw`] reads bevy's after `PhaseSort`.

use bevy::diagnostic::FrameCount;
use bevy::prelude::*;

use benilla_gfx::{GfxPhaseRecord, GfxRender, GfxRenderSystems, GfxViewPhases};

use super::{ms, DrawState, GlueDrawWatch, TAG};

pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        GfxRender,
        (
            record_phases.in_set(GfxRenderSystems::Pack),
            report_glue_draw.after(GfxRenderSystems::Draw),
        ),
    );
}

/// Keep the draw's phase lists every frame the trace is on (the phase probe turns the record off
/// in `Update` when it is not sampling).
fn record_phases(mut record: ResMut<GfxPhaseRecord>) {
    record.enabled = true;
}

/// `report_glue_draw`, over the phases the gfx draw recorded this frame.
fn report_glue_draw(
    watch: Res<GlueDrawWatch>,
    record: Res<GfxPhaseRecord>,
    frame: Option<Res<FrameCount>>,
    mut state: Local<DrawState>,
) {
    if watch.seq != state.seq {
        *state = DrawState {
            seq: watch.seq,
            ..default()
        };
    }
    if state.whole && state.scene {
        return;
    }
    let submitted = |e: &Entity| record.views.iter().any(|v| in_view(v, *e));
    let body = watch.body.iter().filter(|e| submitted(e)).count();
    let scene = watch.scene.iter().filter(|e| submitted(e)).count();
    let at = ms(watch.since, frame.map_or(0, |f| f.0));
    let counts = format!(
        "body {body}/{} scene {scene}/{}",
        watch.body.len(),
        watch.scene.len()
    );
    let line = |what: &str| {
        benilla_assets::trace::line(TAG, &format!("#{} draw {what} {at} {counts}", watch.seq));
    };
    if !state.first && body > 0 {
        state.first = true;
        line("first");
    }
    if !state.whole && !watch.body.is_empty() && body == watch.body.len() {
        state.whole = true;
        line("whole");
    }
    if !state.scene && !watch.scene.is_empty() && scene == watch.scene.len() {
        state.scene = true;
        line("scene");
    }
}

/// Whether `entity` is in any of the view's phases.
fn in_view(view: &GfxViewPhases, entity: Entity) -> bool {
    view.opaque.contains(&entity)
        || view.mask.contains(&entity)
        || view.transparent.iter().any(|(e, _)| *e == entity)
}
