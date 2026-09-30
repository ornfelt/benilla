//! Visibility ranges (*hierarchical levels of detail*), cut to a stand-in: no entity carries a
//! `VisibilityRange`, so the range check found nothing to cull. The system keeps its path, place
//! and ordering, so `PostUpdate` (single-threaded) runs in the order it did.

use bevy_app::{App, Plugin, PostUpdate};
use bevy_ecs::schedule::IntoScheduleConfigs as _;

use super::{check_visibility, VisibilitySystems};

/// The plugin that added the visibility-range check.
pub struct VisibilityRangePlugin;

impl Plugin for VisibilityRangePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            check_visibility_ranges
                .in_set(VisibilitySystems::CheckVisibility)
                .before(check_visibility),
        );
    }
}

/// Stand-in for `check_visibility_ranges`, which filled `VisibleEntityRanges` with the entities
/// whose `VisibilityRange` held each camera's distance.
pub fn check_visibility_ranges() {}
