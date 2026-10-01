//! A module adding debug visualization of `PointLight`s, `SpotLight`s and `DirectionalLight`s.

use bevy_app::{Plugin, PostUpdate};
use bevy_color::{
    palettes::basic::{BLUE, GREEN, RED},
    Color,
};
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

/// Configures how a color is attributed to a light gizmo.
#[derive(Debug, Clone, Copy, Default)]
pub enum LightGizmoColor {
    /// User-specified color.
    Manual(Color),
    /// Random color derived from the light's `Entity`.
    Varied,
    /// Take the color of the represented light.
    #[default]
    MatchLightColor,
    /// Take the color provided by [`LightGizmoConfigGroup`] depending on the light kind.
    ByLightType,
}

/// The [`GizmoConfigGroup`] used to configure the visualization of lights.
#[derive(Clone, TypePath, GizmoConfigGroup)]
pub struct LightGizmoConfigGroup {
    /// Draw a gizmo for all lights if true.
    ///
    /// Defaults to `false`.
    pub draw_all: bool,
    /// Default color strategy for all light gizmos.
    ///
    /// Defaults to [`LightGizmoColor::MatchLightColor`].
    pub color: LightGizmoColor,
    /// [`Color`] to use for drawing a `PointLight` gizmo when [`LightGizmoColor::ByLightType`] is used.
    ///
    /// Defaults to `RED`.
    pub point_light_color: Color,
    /// [`Color`] to use for drawing a `SpotLight` gizmo when [`LightGizmoColor::ByLightType`] is used.
    ///
    /// Defaults to `GREEN`.
    pub spot_light_color: Color,
    /// [`Color`] to use for drawing a `DirectionalLight` gizmo when [`LightGizmoColor::ByLightType`] is used.
    ///
    /// Defaults to `BLUE`.
    pub directional_light_color: Color,
}

impl Default for LightGizmoConfigGroup {
    fn default() -> Self {
        Self {
            draw_all: false,
            color: LightGizmoColor::MatchLightColor,
            point_light_color: RED.into(),
            spot_light_color: GREEN.into(),
            directional_light_color: BLUE.into(),
        }
    }
}

// Stand-in for `draw_lights`, which drew each light with a `ShowLightGizmo`.
fn draw_lights(_gizmos: Gizmos<LightGizmoConfigGroup>) {}

// Stand-in for `draw_all_lights`, which drew every light without a `ShowLightGizmo`.
fn draw_all_lights(_gizmos: Gizmos<LightGizmoConfigGroup>) {}
