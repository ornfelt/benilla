#![cfg_attr(docsrs, feature(doc_cfg))]
#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://bevy.org/assets/icon.png",
    html_favicon_url = "https://bevy.org/assets/icon.png"
)]

//! `bevy_winit` provides utilities to handle window creation and the eventloop through [`winit`]
//!
//! Trimmed for benilla: the gfx library owns the window and nothing adds `WinitPlugin`, so what
//! is left is what bevy_egui names: the window map (always empty) and the event-loop proxy
//! resource (never inserted).

use bevy_derive::Deref;
use core::cell::RefCell;
use winit::event_loop::EventLoopProxy;

use bevy_ecs::prelude::*;
pub use winit_windows::*;

mod winit_windows;

thread_local! {
    /// Temporary storage of WinitWindows data to replace usage of `!Send` resources. This will be replaced with proper
    /// storage of `!Send` data after issue #17667 is complete.
    pub static WINIT_WINDOWS: RefCell<WinitWindows> = const { RefCell::new(WinitWindows::new()) };
}

/// Events that can be sent to perform actions inside the winit event loop.
///
/// Sent via the [`EventLoopProxyWrapper`] resource.
///
/// # Example
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_winit::{EventLoopProxyWrapper, WinitUserEvent};
/// fn wakeup_system(event_loop_proxy: Res<EventLoopProxyWrapper>) -> Result {
///     event_loop_proxy.send_event(WinitUserEvent::WakeUp)?;
///
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone, Copy)]
pub enum WinitUserEvent {
    /// Dummy event that just wakes up the winit event loop
    WakeUp,
    /// Tell winit that a window needs to be created
    WindowAdded,
}

/// A wrapper type around [`winit::event_loop::EventLoopProxy`] with the specific
/// [`winit::event::Event::UserEvent`] used in the `WinitPlugin`.
///
/// The `EventLoopProxy` can be used to request a redraw from outside bevy.
///
/// Use `Res<EventLoopProxyWrapper>` to retrieve this resource.
#[derive(Resource, Deref)]
pub struct EventLoopProxyWrapper(EventLoopProxy<WinitUserEvent>);
