//! The window's event handler queues each event for the runner, which drains the queue into
//! Bevy's messages after the poll. The library calls the handler on the main thread, inside
//! `gfx_dll_window_poll_events`, so the queue is thread-local.

use std::cell::RefCell;

use crate::ffi::GfxEvent;

thread_local! {
    static QUEUE: RefCell<Vec<GfxEvent>> = const { RefCell::new(Vec::new()) };
}

/// The handler installed on the window. It must not unwind into C, so a re-entrant call (which
/// the library never makes) drops its event instead of panicking.
pub(crate) unsafe extern "C" fn on_event(evt: *mut GfxEvent) {
    if evt.is_null() {
        return;
    }
    // SAFETY: the library passes a live event for the duration of the call.
    let evt = unsafe { *evt };
    QUEUE.with(|q| {
        if let Ok(mut q) = q.try_borrow_mut() {
            q.push(evt);
        }
    });
}

/// Moves the queued events into `out`, in arrival order.
pub fn drain_into(out: &mut Vec<GfxEvent>) {
    QUEUE.with(|q| out.append(&mut q.borrow_mut()));
}
