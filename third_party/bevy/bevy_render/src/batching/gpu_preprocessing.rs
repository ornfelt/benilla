//! Batching functionality when GPU preprocessing is in use.

use bevy_app::{App, Plugin};

use crate::RenderDebugFlags;

#[derive(Default)]
pub struct BatchingPlugin {
    /// Debugging flags that can optionally be set when constructing the renderer.
    pub debug_flags: RenderDebugFlags,
}

impl Plugin for BatchingPlugin {
    fn build(&self, _app: &mut App) {
        // The indirect-parameter buffers and GPU preprocessing support only reached the RenderApp.
    }
}
