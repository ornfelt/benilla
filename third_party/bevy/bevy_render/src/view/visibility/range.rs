//! Specific distances from the camera in which entities are visible, also known
//! as *hierarchical levels of detail* or *HLOD*s.

use bevy_app::{App, Plugin};

/// We need at least 4 storage buffer bindings available to enable the
/// visibility range buffer.
///
/// Even though we only use one storage buffer, the first 3 available storage
/// buffers will go to various light-related buffers. We will grab the fourth
/// buffer slot.
pub const VISIBILITY_RANGES_STORAGE_BUFFER_COUNT: u32 = 4;

/// A plugin that enables render-world visibility ranges, which allow entities to be
/// hidden or shown based on distance to the camera.
pub struct RenderVisibilityRangePlugin;

impl Plugin for RenderVisibilityRangePlugin {
    fn build(&self, _app: &mut App) {
        // The visibility-range buffer and its extraction only reached the RenderApp.
    }
}
