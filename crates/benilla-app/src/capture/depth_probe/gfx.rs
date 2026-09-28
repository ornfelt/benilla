//! The depth probe under gfx: no render graph, so each frame in the window asks the gfx draw for
//! the world camera's depth at the node's point of its phases ([`benilla_gfx::GfxDepthProbe`]);
//! the copy comes back two frames later, reported against the quads and projection of the frame
//! it was drawn in.

use std::collections::BTreeMap;

use benilla_gfx::{GfxDepthProbe, GfxDepthRequest};
use bevy::prelude::*;

use super::{collect_quads, report_frame, DepthProbeView, DepthWatch, QuadProbe, QuadProbes};

pub(super) fn build(app: &mut App) {
    app.init_resource::<GfxDepthFrames>()
        .add_systems(PostUpdate, request.after(collect_quads));
}

/// A copy asked for and not yet read back: what its frame drew with.
struct Pending {
    quads: Vec<QuadProbe>,
    clip_from_view: Mat4,
}

/// The frames reported, and the copies in flight by tag.
#[derive(Resource, Default)]
struct GfxDepthFrames {
    read: u32,
    next_tag: u32,
    pending: BTreeMap<u32, Pending>,
}

/// How many frames a copy may stay unanswered (its camera did not draw) before it is dropped.
const STALE: u32 = 4;

/// Report the copy the draw handed back, then ask for this frame's, as the graph node copies
/// once per frame until the burst is read.
fn request(
    watch: Res<DepthWatch>,
    quads: Res<QuadProbes>,
    mut frames: ResMut<GfxDepthFrames>,
    mut probe: ResMut<GfxDepthProbe>,
    // `arm` marks the world camera alone.
    cam: Query<(Entity, &Camera), With<DepthProbeView>>,
) {
    if let Some(back) = probe.result.take() {
        // Older copies were never drawn: the draw answers in order.
        frames.pending.retain(|&tag, _| tag >= back.tag);
        if let Some(p) = frames.pending.remove(&back.tag) {
            if frames.read < watch.count {
                let frame = frames.read;
                frames.read += 1;
                let width = back.size.x as usize;
                report_frame(
                    frame,
                    back.size,
                    &p.clip_from_view,
                    &watch.pixels,
                    &p.quads,
                    |x, y| back.depth[y as usize * width + x as usize],
                );
            }
        }
    }
    let next = frames.next_tag;
    frames
        .pending
        .retain(|&tag, _| next.wrapping_sub(tag) <= STALE);
    if !watch.armed || frames.read + frames.pending.len() as u32 >= watch.count {
        return;
    }
    // Quad mode with nothing live this frame: no copy, and the frame is not counted.
    if watch.pixels.is_empty() && quads.0.is_empty() {
        return;
    }
    let Ok((camera, cam)) = cam.single() else {
        return;
    };
    frames.pending.insert(
        next,
        Pending {
            quads: quads.0.clone(),
            clip_from_view: cam.clip_from_view(),
        },
    );
    frames.next_tag = next.wrapping_add(1);
    probe.request = Some(GfxDepthRequest {
        camera,
        after_transparent: std::env::var_os("WOW_DEPTH_AFTER").is_some(),
        tag: next,
    });
}
