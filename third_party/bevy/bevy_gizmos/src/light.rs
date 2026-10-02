//! A module adding debug visualization of `PointLight`s, `SpotLight`s and `DirectionalLight`s.

use bevy_app::{Plugin, PostUpdate};
use bevy_ecs::{schedule::IntoScheduleConfigs, system::Res};
use bevy_reflect::TypePath;
use bevy_transform::TransformSystems;

use crate::{
    config::{GizmoConfigGroup, GizmoConfigStore},
    gizmos::Gizmos,
    AppGizmoBuilder,
};

/// A [`Plugin`] that provides visualization of `PointLight`s, `SpotLight`s
/// and `DirectionalLight`s for debugging.
pub struct LightGizmoPlugin;

impl Plugin for LightGizmoPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.init_gizmo_group::<LightGizmoConfigGroup>().add_systems(
            PostUpdate,
            (
                draw_lights,
                draw_all_lights.run_if(|config: Res<GizmoConfigStore>| {
                    config.config::<LightGizmoConfigGroup>().1.draw_all
                }),
            )
                .after(TransformSystems::Propagate),
        );
    }
}

/// The [`GizmoConfigGroup`] used to configure the visualization of lights.
#[derive(Clone, Default, TypePath, GizmoConfigGroup)]
pub struct LightGizmoConfigGroup {
    /// Draw a gizmo for all lights if true.
    ///
    /// Defaults to `false`.
    pub draw_all: bool,
}

// Stand-in for `draw_lights`, which drew each light with a `ShowLightGizmo`.
fn draw_lights(_gizmos: Gizmos<LightGizmoConfigGroup>) {}

// Stand-in for `draw_all_lights`, which drew every light without a `ShowLightGizmo`.
fn draw_all_lights(_gizmos: Gizmos<LightGizmoConfigGroup>) {}
