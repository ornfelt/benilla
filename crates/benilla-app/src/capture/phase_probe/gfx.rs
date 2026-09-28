//! The phase probe under gfx: no render world, so the gfx draw keeps each 3D view's phase lists
//! ([`benilla_gfx::GfxPhaseRecord`]) while the watch is sampling, and the next frame reports them
//! as `report_phases` reports bevy's after `PhaseSort`: the census, the particle tail, and where
//! each watched batch sits.

use benilla_gfx::draw::GfxRenderer;
use benilla_gfx::{GfxPhaseRecord, GfxViewPhases};
use bevy::prelude::*;

use super::{collect_batches, collect_emitters, PhaseWatch};

pub(super) fn build(app: &mut App) {
    app.add_systems(
        Update,
        report_phases.after(collect_batches).after(collect_emitters),
    );
}

/// Report what the last frame's draw recorded, then keep recording while frames remain.
fn report_phases(
    watch: Res<PhaseWatch>,
    mut record: ResMut<GfxPhaseRecord>,
    renderer: Option<NonSend<GfxRenderer>>,
    images: Res<Assets<Image>>,
    mut seen: Local<u32>,
) {
    if record.enabled && !record.views.is_empty() && *seen < watch.count {
        report(&watch, &record.views, renderer.as_deref(), &images, *seen);
        *seen += 1;
    }
    record.enabled = watch.armed && *seen < watch.count;
}

/// One frame's report, in `report_phases`' words.
fn report(
    watch: &PhaseWatch,
    views: &[GfxViewPhases],
    renderer: Option<&GfxRenderer>,
    images: &Assets<Image>,
    frame: u32,
) {
    // The whole-frame census first, which sees draws outside the watch list: the binned phases
    // of the view holding the most, the transparent ones of every view.
    let transparent_total: usize = views.iter().map(|v| v.transparent.len()).sum();
    info!(
        "phase#{frame} CENSUS opaque {} alphamask {} transparent {}",
        views.iter().map(|v| v.opaque.len()).max().unwrap_or(0),
        views.iter().map(|v| v.mask.len()).max().unwrap_or(0),
        transparent_total,
    );
    // `particles` mode: the last 25 transparent items, drawn over everything else.
    if watch.particles {
        if let Some(view) = views.iter().max_by_key(|v| v.transparent.len()) {
            let n = view.transparent.len();
            for (i, (entity, distance)) in view
                .transparent
                .iter()
                .enumerate()
                .skip(n.saturating_sub(25))
            {
                info!("phase#{frame} tail @{i} {entity:?} dist {distance:.3}");
            }
        }
    }
    let largest = |len: fn(&GfxViewPhases) -> usize| views.iter().max_by_key(|v| len(v));
    let opaque = largest(|v| v.opaque.len());
    let mask = largest(|v| v.mask.len());
    for &(entity, submesh, ref blend, assets) in &watch.batches {
        // What the device holds: an item whose mesh or texture never reached it draws nothing.
        let gpu = assets.map(|(mesh_id, tex_id)| {
            let mesh = match mesh_id {
                // The shared effect lane: no per-emitter mesh asset exists to check.
                None => "shared-lane".to_string(),
                Some(id) => match renderer.and_then(|r| r.meshes.peek(id)) {
                    None => "gpu_mesh MISSING".to_string(),
                    Some(m) => format!("gpu_verts {}", m.vertex_count),
                },
            };
            let tex = match renderer
                .and_then(|r| r.images.peek(tex_id))
                .and(images.get(tex_id))
            {
                None => "gpu_tex MISSING".to_string(),
                Some(img) => format!("gpu_tex {:?}", img.texture_descriptor.format),
            };
            format!("{mesh} {tex}")
        });
        let mut found = Vec::new();
        if let Some(pos) = opaque.and_then(|v| v.opaque.iter().position(|e| *e == entity)) {
            found.push(format!("Opaque3d @{pos}"));
        }
        if let Some(pos) = mask.and_then(|v| v.mask.iter().position(|e| *e == entity)) {
            found.push(format!("AlphaMask3d @{pos}"));
        }
        for view in views {
            if let Some(pos) = view.transparent.iter().position(|(e, _)| *e == entity) {
                // The sort key too: view-space z plus the material's depth bias, ascending =
                // farthest first (`benilla_world::sky_order`).
                found.push(format!(
                    "Transparent3d @{pos} d {:.3}",
                    view.transparent[pos].1
                ));
            }
        }
        info!(
            "phase#{frame} {entity} batch order {submesh:3} {blend:10} {} -> {}",
            gpu.as_deref().unwrap_or(""),
            if found.is_empty() {
                "NOT SUBMITTED".to_string()
            } else {
                found.join(", ")
            }
        );
    }
}
