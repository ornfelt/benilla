//! The whole-frame GPU meter on gfx (`WOW_GPU_MS=1`, `benilla-app`'s `perf::gpu`): a timestamp
//! as the first camera starts drawing ([`GfxRenderSystems::Draw`]) and one once the window's
//! present pass is recorded, as the wgpu meter's two sentinel passes bracket bevy's camera
//! driver. Each frame's pair goes into a ring of [`FRAMES`] pairs of gfx_benilla's timestamp
//! slots and is read back [`FRAMES`] - 1 frames later, when the GPU is long past it, so the meter
//! never waits; the freshest difference is published in nanoseconds.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use bevy::prelude::*;

use crate::context::GfxContext;
use crate::ffi;
use crate::render::GfxRenderSystems;

/// Frames in the ring: deeper than any device's frames in flight, so a slot's last writer has
/// finished before it is written again.
const FRAMES: u32 = 8;

/// Arms the meter: the freshest whole-frame GPU time in nanoseconds is stored here, 0 until the
/// first reading. Inserted by the app that wants it.
#[derive(Resource, Clone)]
pub struct GfxGpuMeter(pub Arc<AtomicU64>);

/// The frame's index in the ring, and whether the device has timestamps (known after the first
/// write).
#[derive(Resource, Default)]
struct Ring {
    frame: u32,
    unsupported: bool,
}

fn slots(frame: u32) -> (u32, u32) {
    let pair = frame % FRAMES;
    (2 * pair, 2 * pair + 1)
}

fn armed(meter: Option<Res<GfxGpuMeter>>, ring: Res<Ring>) -> bool {
    meter.is_some() && !ring.unsupported
}

/// Prepare: the pair this frame is about to overwrite, written [`FRAMES`] frames ago.
fn read(ctx: NonSend<GfxContext>, meter: Res<GfxGpuMeter>, ring: Res<Ring>) {
    if ring.frame < FRAMES {
        return;
    }
    let (begin, end) = slots(ring.frame);
    let (mut a, mut b) = (0u64, 0u64);
    // SAFETY: the live device and two out-pointers.
    let read = unsafe {
        ffi::gfx_dll_read_timestamp(ctx.device, begin, &mut a)
            && ffi::gfx_dll_read_timestamp(ctx.device, end, &mut b)
    };
    if read && b >= a {
        meter.0.store(b - a, Ordering::Relaxed);
    }
}

/// The start of Draw.
fn begin(ctx: NonSend<GfxContext>, mut ring: ResMut<Ring>) {
    // SAFETY: the live device, in the frame being recorded.
    if !unsafe { ffi::gfx_dll_write_timestamp(ctx.device, slots(ring.frame).0) } {
        ring.unsupported = true;
        warn!("gfx: this device has no GPU timestamps; WOW_GPU_MS reads 0");
    }
}

/// After Present: the frame's work is all recorded.
fn end(ctx: NonSend<GfxContext>, mut ring: ResMut<Ring>) {
    // SAFETY: the live device, in the frame being recorded.
    unsafe { ffi::gfx_dll_write_timestamp(ctx.device, slots(ring.frame).1) };
    ring.frame = ring.frame.wrapping_add(1);
}

pub(crate) fn build(app: &mut App) {
    app.init_resource::<Ring>().add_systems(
        crate::render::GfxRender,
        (
            read.in_set(GfxRenderSystems::Prepare),
            begin
                .in_set(GfxRenderSystems::Draw)
                .before(crate::draw::draw_views),
            end.after(GfxRenderSystems::Present),
        )
            .run_if(armed),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The pair read before a frame's writes is the one written `FRAMES` frames before it, and no
    /// slot leaves the device's ring.
    #[test]
    fn a_frame_reads_the_pair_it_is_about_to_write() {
        for frame in FRAMES..FRAMES * 4 {
            assert_eq!(slots(frame), slots(frame - FRAMES));
            let (b, e) = slots(frame);
            assert!(b < ffi::TIMESTAMP_SLOTS && e < ffi::TIMESTAMP_SLOTS && e == b + 1);
        }
    }
}
