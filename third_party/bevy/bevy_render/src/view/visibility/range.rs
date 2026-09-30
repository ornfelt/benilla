//! Specific distances from the camera in which entities are visible, also known
//! as *hierarchical levels of detail* or *HLOD*s.

use bevy_app::{App, Plugin};

/// A plugin that enables render-world visibility ranges, which allow entities to be
/// hidden or shown based on distance to the camera.
pub struct RenderVisibilityRangePlugin;

impl Plugin for RenderVisibilityRangePlugin {
    fn build(&self, _app: &mut App) {
        // The visibility-range buffer and its extraction only reached the RenderApp.
    }
}
