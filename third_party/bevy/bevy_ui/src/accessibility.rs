//! `bevy_ui`'s `AccessKit` integration, cut to stand-ins: no AccessKit adapter exists (no
//! `WinitPlugin`), so nothing reads the `AccessibilityNode`s these systems built. Each keeps its
//! path, place and ordering, so `PostUpdate` (single-threaded) runs in the order it did.

use bevy_app::{App, Plugin, PostUpdate};
use bevy_ecs::schedule::IntoScheduleConfigs;

use bevy_camera::CameraUpdateSystems;

/// Stand-in for `calc_bounds`, which set each `AccessibilityNode`'s bounds from its layout.
fn calc_bounds() {}

/// Stand-in for `button_changed`, which gave a changed `Button` its `AccessibilityNode`.
fn button_changed() {}

/// Stand-in for `image_changed`, which gave a changed `ImageNode` its `AccessibilityNode`.
fn image_changed() {}

/// Stand-in for `label_changed`, which gave a changed `Label` its `AccessibilityNode`.
fn label_changed() {}

/// `AccessKit` integration for `bevy_ui`.
pub(crate) struct AccessibilityPlugin;

impl Plugin for AccessibilityPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            (
                calc_bounds
                    .after(bevy_transform::TransformSystems::Propagate)
                    .after(CameraUpdateSystems)
                    // the listed systems do not affect calculated size
                    .ambiguous_with(crate::ui_stack_system),
                button_changed,
                image_changed,
                label_changed,
            ),
        );
    }
}
