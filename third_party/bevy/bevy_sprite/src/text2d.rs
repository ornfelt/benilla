//! 2D text, cut to its frame: no entity is `Text2d`, so its layout and bounds systems stay as
//! stand-ins with their paths, places and ordering, and `PostUpdate` (single-threaded) runs in the
//! order it did.

use bevy_ecs::{component::Component, system::Commands};

/// The top-level 2D text component. It names the root of
/// `bevy_text::detect_text_needs_rerender::<Text2d>`; nothing spawns one.
#[derive(Component, Clone, Debug, Default)]
pub struct Text2d(pub String);

/// Stand-in for `update_text2d_layout`, which laid out each `Text2d` block for the cameras
/// showing it.
pub fn update_text2d_layout() {}

/// Stand-in for `calculate_bounds_text2d`, which gave each laid-out `Text2d` block its `Aabb`. It
/// keeps its `Commands`, the deferred work that places `PostUpdate`'s sync points.
pub fn calculate_bounds_text2d(_commands: Commands) {}
