//! `WOW_GPU_MS=1`: the whole-frame GPU meter. The gfx renderer brackets the frame's draws with
//! timer queries (`benilla_gfx::GfxGpuMeter`) and publishes the freshest delta in nanoseconds
//! through one shared `AtomicU64`.

use std::sync::atomic::AtomicU64;
use std::sync::Arc;

use bevy::prelude::*;

/// Whether the meter is armed, read once.
pub(crate) fn enabled() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var("WOW_GPU_MS").as_deref() == Ok("1"))
}

/// The freshest whole-frame GPU duration in nanoseconds, written by the gfx meter;
/// 0 until the first reading.
#[derive(Resource, Clone)]
pub(crate) struct GpuMsShared(pub Arc<AtomicU64>);

/// The wgpu resource census `FPS_PROBE` prints: live buffers, textures and bind groups, and draw
/// calls per family. Nothing fills it on the gfx renderer, so it reads 0.
#[derive(Resource, Clone)]
pub(crate) struct WgpuCensusShared(pub Arc<WgpuCensus>);

#[derive(Default)]
pub(crate) struct WgpuCensus {
    pub buffers: AtomicU64,
    pub textures: AtomicU64,
    pub bind_groups: AtomicU64,
    /// Draw calls this frame, per family.
    pub draws_opaque: AtomicU64,
    pub draws_transparent: AtomicU64,
    pub draws_shadow: AtomicU64,
    pub draws_ui: AtomicU64,
}

pub(crate) fn plugin(app: &mut App) {
    if !enabled() {
        return;
    }
    let shared = Arc::new(AtomicU64::new(0));
    let counts = Arc::new(WgpuCensus::default());
    app.insert_resource(GpuMsShared(shared.clone()));
    app.insert_resource(WgpuCensusShared(counts));
    // gfx's meter brackets the frame and writes the counter.
    app.insert_resource(benilla_gfx::GfxGpuMeter(shared));
}
