#![expect(
    unsafe_code,
    reason = "This module acts as a wrapper around the `raw_window_handle` crate, which exposes many unsafe interfaces; thus, we have to use unsafe code here."
)]

use alloc::sync::Arc;
use bevy_ecs::prelude::Component;
use bevy_platform::sync::Mutex;
use core::{any::Any, marker::PhantomData, ops::Deref};
use raw_window_handle::RawDisplayHandle;

/// A wrapper over a window.
///
/// This allows us to extend the lifetime of the window, so it doesn't get eagerly dropped while a
/// pipelined renderer still has frames in flight that need to draw to it.
#[derive(Debug)]
pub struct WindowWrapper<W> {
    reference: Arc<dyn Any + Send + Sync>,
    ty: PhantomData<W>,
}

impl<W: 'static> Deref for WindowWrapper<W> {
    type Target = W;

    fn deref(&self) -> &Self::Target {
        self.reference.downcast_ref::<W>().unwrap()
    }
}

/// A wrapper over [`RawDisplayHandle`] that allows us to safely pass it across threads.
///
/// Depending on the platform, the underlying pointer-containing handle cannot be used on all threads,
/// and so we cannot simply make it (or any type that has a safe operation to get a [`RawDisplayHandle`])
/// thread-safe.
#[derive(Debug, Clone, Component)]
pub struct RawHandleWrapper {
    /// Raw handle to the display server.
    display_handle: RawDisplayHandle,
}

impl RawHandleWrapper {
    /// Gets the stored display handle
    pub fn get_display_handle(&self) -> RawDisplayHandle {
        self.display_handle
    }
}

// SAFETY: [`RawHandleWrapper`] is just a normal "raw pointer", which doesn't impl Send/Sync. The pointer is only
// handed out as a copy of the raw handle, whose use is unsafe on the caller's side.
// A recommendation for this pattern (and more context) is available here:
// https://github.com/rust-windowing/raw-window-handle/issues/59
unsafe impl Send for RawHandleWrapper {}
// SAFETY: This is safe for the same reasons as the Send impl above.
unsafe impl Sync for RawHandleWrapper {}

/// Holder of the [`RawHandleWrapper`] with wrappers, to allow use in asynchronous context
#[derive(Debug, Clone, Component)]
pub struct RawHandleWrapperHolder(pub Arc<Mutex<Option<RawHandleWrapper>>>);
