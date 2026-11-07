//! `WOW_MOVE_TRACE_TAGS=gsel`: the glue character on the trace clock, from the look that asks for
//! it to the first frame its batches are submitted to a render phase. Each select is numbered, and
//! every line carries the milliseconds and main frames since that select:
//!
//! - `select` the look changed (a select or a create dial);
//! - `bake` the assembly for it landed ([`GluePreviewBake`]);
//! - `stage` the requested scene file stands, when it was not standing at the select;
//! - `stand` the booth stood a fresh set of batches, `of=look` when they are this look's bake's
//!   and `of=other` when a stage swap stood the old look again;
//! - `draw first` / `draw whole` one, then every, standing batch of the look is in a phase the gfx
//!   draw built this frame ([`GfxPhaseRecord`]); `draw scene` every batch of the requested scene
//!   file.
//!
//! The reference builds and waits for the whole stage in the select's call (`0x473083`
//! `IsReady(1, 1)` on the scene model, `0x710450`), so its character draws in the select's frame.

use std::time::Instant;

use bevy::diagnostic::FrameCount;
use bevy::ecs::entity::EntityHashSet;
use bevy::prelude::*;

use benilla_gfx::{GfxPhaseRecord, GfxRender, GfxRenderSystems, GfxViewPhases};

use super::{CreateScene, GlueLook, GluePreview, GluePreviewBake, GLUE_SLOT};
use crate::portrait::Booths;

/// The trace tag.
const TAG: &str = "gsel";

/// Register the instrument when the trace is on for its tag; otherwise it costs nothing.
pub(in crate::portrait) fn plugin(app: &mut App) {
    if !benilla_assets::trace::enabled_for(TAG) {
        return;
    }
    app.init_resource::<GlueDrawWatch>()
        // The select's stamp, before any work it asks for runs in the frame.
        .add_systems(
            Update,
            stamp_glue_select.after(crate::portrait::GluePreviewFeed),
        )
        .add_systems(Last, watch_glue_select)
        // The draw lines read the phases the gfx draw recorded, after it, in the same frame.
        .add_systems(
            GfxRender,
            (
                record_phases.in_set(GfxRenderSystems::Pack),
                report_glue_draw.after(GfxRenderSystems::Draw),
            ),
        );
}

/// The select being watched.
#[derive(Resource, Clone, Default)]
struct GlueDrawWatch {
    /// The select's number, from 1.
    seq: u64,
    /// When the select was seen, and its main frame.
    since: Option<(Instant, u32)>,
    /// The new look's batches, once they alone stand on the character's root.
    body: Vec<Entity>,
    /// The standing stage's batches.
    scene: Vec<Entity>,
}

/// The main world's memory of the select under watch.
#[derive(Default)]
struct SelectState {
    seq: u64,
    /// The character root's batches at the end of the last frame.
    last_standing: EntityHashSet,
    baked: bool,
    staged: bool,
}

fn ms(since: Option<(Instant, u32)>, frame: u32) -> String {
    since.map_or_else(String::new, |(t, f)| {
        format!(
            "+{:.1}ms frames={}",
            t.elapsed().as_secs_f64() * 1e3,
            frame.wrapping_sub(f)
        )
    })
}

fn look_line(look: &GlueLook) -> String {
    match look {
        GlueLook::Select(l) => format!(
            "select race={} sex={} worn={}",
            l.race,
            l.sex,
            l.equipment.iter().filter(|e| e.display_id != 0).count()
        ),
        GlueLook::Create(l) => format!("create race={} sex={} class={}", l.race, l.sex, l.class),
    }
}

/// Number and stamp each change of the look, in the frame the feed wrote it.
fn stamp_glue_select(
    preview: Res<GluePreview>,
    scene: Option<Res<CreateScene>>,
    frame: Res<FrameCount>,
    mut watch: ResMut<GlueDrawWatch>,
    mut last: Local<Option<GlueLook>>,
) {
    if preview.look == *last {
        return;
    }
    *last = preview.look;
    watch.seq += 1;
    watch.since = Some((Instant::now(), frame.0));
    watch.body.clear();
    watch.scene.clear();
    let line = match preview.look.as_ref() {
        None => "clear".to_string(),
        Some(look) => format!(
            "{} scene={} staged={} frame={}",
            look_line(look),
            preview.scene.as_ref().map_or("-", |s| s.path()),
            staged(scene.as_deref(), &preview),
            frame.0
        ),
    };
    benilla_assets::trace::line(TAG, &format!("#{} {line}", watch.seq));
}

/// Whether the requested scene file stands.
fn staged(scene: Option<&CreateScene>, preview: &GluePreview) -> bool {
    scene.is_some_and(|s| s.spawned && s.file.is_some() && s.file == preview.scene)
}

/// Follow the look from its change to its batches standing.
#[allow(clippy::too_many_arguments)]
fn watch_glue_select(
    preview: Res<GluePreview>,
    bake: Res<GluePreviewBake>,
    booths: Res<Booths>,
    scene: Option<Res<CreateScene>>,
    children: Query<&Children>,
    mesh_of: Query<&Mesh3d>,
    frame: Res<FrameCount>,
    mut watch: ResMut<GlueDrawWatch>,
    mut state: Local<SelectState>,
) {
    let Some(booth) = booths.0.get(GLUE_SLOT) else {
        return;
    };
    let batches = |root: Entity| -> Vec<Entity> {
        children
            .iter_descendants(root)
            .filter(|e| mesh_of.contains(*e))
            .collect()
    };
    let standing = batches(booth.root);
    let last_standing =
        std::mem::replace(&mut state.last_standing, standing.iter().copied().collect());
    let staged = staged(scene.as_deref(), &preview);
    if watch.seq != state.seq {
        *state = SelectState {
            seq: watch.seq,
            last_standing: std::mem::take(&mut state.last_standing),
            staged,
            ..default()
        };
    }
    if preview.look.is_none() {
        return;
    }
    let at = ms(watch.since, frame.0);
    if !state.baked && bake.look == preview.look {
        state.baked = true;
        benilla_assets::trace::line(
            TAG,
            &format!(
                "#{} bake {at} parts={} riders={} cards={}",
                watch.seq,
                bake.parts.len(),
                bake.riders.len(),
                bake.billboards.len()
            ),
        );
    }
    if !state.staged && staged {
        state.staged = true;
        benilla_assets::trace::line(TAG, &format!("#{} stage {at}", watch.seq));
    }
    // The stage counts once the requested file stands; until then the old one is no answer.
    watch.scene = scene
        .as_deref()
        .filter(|_| staged)
        .map(|s| batches(s.root))
        .unwrap_or_default();
    // A fresh stand: every batch new since the last frame. It is this look's when the bake is
    // and every batch draws one of the bake's meshes; a stage swap re-stands the old look too.
    let fresh = !standing.is_empty() && standing.iter().all(|e| !last_standing.contains(e));
    if !fresh {
        return;
    }
    let baked: bevy::platform::collections::HashSet<AssetId<Mesh>> = bake
        .parts
        .iter()
        .flat_map(|p| {
            [
                Some(p.static_mesh.id()),
                p.skinned_mesh.as_ref().map(Handle::id),
            ]
        })
        .flatten()
        .chain(bake.riders.iter().map(|r| r.mesh.id()))
        .chain(bake.billboards.iter().map(|b| b.mesh.id()))
        .collect();
    let ours = bake.look == preview.look
        && standing
            .iter()
            .all(|e| mesh_of.get(*e).is_ok_and(|m| baked.contains(&m.id())));
    benilla_assets::trace::line(
        TAG,
        &format!(
            "#{} stand {at} batches={} of={}",
            watch.seq,
            standing.len(),
            if ours { "look" } else { "other" }
        ),
    );
    if ours {
        watch.body = standing;
    }
}

/// The draw lines' state: how many watched batches are in a phase this frame.
#[derive(Default)]
struct DrawState {
    seq: u64,
    first: bool,
    whole: bool,
    scene: bool,
}

/// Keep the draw's phase lists every frame the trace is on (the phase probe turns the record off
/// in `Update` when it is not sampling).
fn record_phases(mut record: ResMut<GfxPhaseRecord>) {
    record.enabled = true;
}

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
