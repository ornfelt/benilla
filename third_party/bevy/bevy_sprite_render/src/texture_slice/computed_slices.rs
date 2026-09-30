use bevy_ecs::prelude::*;

/// Stand-in for `compute_slices_on_asset_event`, which recomputed the slices of each sliced or
/// tiled `Sprite` whose image was added or modified. No entity has a `Sprite`. It keeps its
/// `Commands`, the deferred work that places `PostUpdate`'s sync points.
pub(crate) fn compute_slices_on_asset_event(_commands: Commands) {}

/// Stand-in for `compute_slices_on_sprite_change`, which recomputed the slices of each changed
/// sliced or tiled `Sprite`. No entity has a `Sprite`.
pub(crate) fn compute_slices_on_sprite_change() {}
