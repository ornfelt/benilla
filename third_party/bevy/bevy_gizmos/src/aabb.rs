//! A module adding debug visualization of `Aabb`s.

use bevy_app::{Plugin, PostUpdate};
use bevy_ecs::{schedule::IntoScheduleConfigs, system::Res};
use bevy_reflect::TypePath;
use bevy_transform::TransformSystems;

use crate::{
    config::{GizmoConfigGroup, GizmoConfigStore},
    gizmos::Gizmos,
    AppGizmoBuilder,
};

/// A [`Plugin`] that provides visualization of `Aabb`s for debugging.
pub struct AabbGizmoPlugin;

impl Plugin for AabbGizmoPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.init_gizmo_group::<AabbGizmoConfigGroup>().add_systems(
            PostUpdate,
            (
                draw_aabbs,
                draw_all_aabbs.run_if(|config: Res<GizmoConfigStore>| {
                    config.config::<AabbGizmoConfigGroup>().1.draw_all
                }),
            )
                .after(bevy_camera::visibility::VisibilitySystems::MarkNewlyHiddenEntitiesInvisible)
                .after(TransformSystems::Propagate),
        );
    }
}
/// The [`GizmoConfigGroup`] used for debug visualizations of `Aabb` components on entities
#[derive(Clone, Default, TypePath, GizmoConfigGroup)]
pub struct AabbGizmoConfigGroup {
    /// Draws all bounding boxes in the scene when set to `true`.
    ///
    /// Defaults to `false`.
    pub draw_all: bool,
}

// Stand-in for `draw_aabbs`, which drew the `Aabb` of each entity with a `ShowAabbGizmo`.
fn draw_aabbs(_gizmos: Gizmos<AabbGizmoConfigGroup>) {}

// Stand-in for `draw_all_aabbs`, which drew every visible entity's `Aabb`.
fn draw_all_aabbs(_gizmos: Gizmos<AabbGizmoConfigGroup>) {}
