use bevy_app::{plugin_group, Plugin};

use crate::anti_alias;
use crate::gilrs;
use crate::gltf;

plugin_group! {
    /// This plugin group will add all the default plugins for a *Bevy* application:
    pub struct DefaultPlugins {
        bevy_app:::PanicHandlerPlugin,
        bevy_log:::LogPlugin,
        bevy_app:::TaskPoolPlugin,
        bevy_diagnostic:::FrameCountPlugin,
        bevy_time:::TimePlugin,
        bevy_transform:::TransformPlugin,
        bevy_diagnostic:::DiagnosticsPlugin,
        bevy_input:::InputPlugin,
        bevy_window:::WindowPlugin,
        #[custom(cfg(any(all(unix, not(target_os = "horizon")), windows)))]
        bevy_app:::TerminalCtrlCHandlerPlugin,
        bevy_asset:::AssetPlugin,
        bevy_scene:::ScenePlugin,
        bevy_render:::RenderPlugin,
        // NOTE: Load this after renderer initialization so that it knows about the supported
        // compressed texture formats.
        bevy_image:::ImagePlugin,
        bevy_mesh:::MeshPlugin,
        bevy_camera:::CameraPlugin,
        bevy_light:::LightPlugin,
        bevy_render::pipelined_rendering:::PipelinedRenderingPlugin,
        bevy_core_pipeline:::CorePipelinePlugin,
        anti_alias:::AntiAliasPlugin,
        bevy_sprite:::SpritePlugin,
        bevy_sprite_render:::SpriteRenderPlugin,
        bevy_text:::TextPlugin,
        bevy_ui:::UiPlugin,
        bevy_ui_render:::UiRenderPlugin,
        bevy_pbr:::PbrPlugin,
        // A stand-in, as are `AntiAliasPlugin` and `GilrsPlugin` (`crate::cut`).
        gltf:::GltfPlugin,
        gilrs:::GilrsPlugin,
        bevy_animation:::AnimationPlugin,
        bevy_gizmos:::GizmoPlugin,
        bevy_state::app:::StatesPlugin,
        #[doc(hidden)]
        :IgnoreAmbiguitiesPlugin,
    }
    /// Every crate it names is in benilla's build (its features stay declared and always on), so
    /// no slot is feature-gated.
    ///
    /// [`DefaultPlugins`] contains all the plugins typically required to build
    /// a *Bevy* application which includes a *window* and presentation components.
    /// For the absolute minimum number of plugins needed to run a Bevy application, see [`MinimalPlugins`].
}

#[derive(Default)]
struct IgnoreAmbiguitiesPlugin;

impl Plugin for IgnoreAmbiguitiesPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        // bevy_ui owns the Transform and cannot be animated
        if app.is_plugin_added::<bevy_animation::AnimationPlugin>()
            && app.is_plugin_added::<bevy_ui::UiPlugin>()
        {
            app.ignore_ambiguity(
                bevy_app::PostUpdate,
                bevy_animation::advance_animations,
                bevy_ui::ui_layout_system,
            );
            app.ignore_ambiguity(
                bevy_app::PostUpdate,
                bevy_animation::animate_targets,
                bevy_ui::ui_layout_system,
            );
        }
    }
}

plugin_group! {
    /// This plugin group will add the minimal plugins for a *Bevy* application:
    pub struct MinimalPlugins {
        bevy_app:::TaskPoolPlugin,
        bevy_diagnostic:::FrameCountPlugin,
        bevy_time:::TimePlugin,
        bevy_app:::ScheduleRunnerPlugin,
    }
    /// This plugin group represents the absolute minimum, bare-bones, bevy application.
    /// Use this if you want to have absolute control over the plugins used.
    ///
    /// It includes a [schedule runner (`ScheduleRunnerPlugin`)](crate::app::ScheduleRunnerPlugin)
    /// to provide functionality that would otherwise be driven by a windowed application's
    /// *event loop* or *message loop*.
    ///
    /// By default, this loop will run as fast as possible, which can result in high CPU usage.
    /// You can add a delay using [`run_loop`](crate::app::ScheduleRunnerPlugin::run_loop),
    /// or remove the loop using [`run_once`](crate::app::ScheduleRunnerPlugin::run_once).
    /// # Example:
    /// ```rust, no_run
    /// # use std::time::Duration;
    /// # use bevy_app::{App, PluginGroup, ScheduleRunnerPlugin};
    /// # use bevy_internal::MinimalPlugins;
    /// App::new().add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(
    ///     // Run 60 times per second.
    ///     Duration::from_secs_f64(1.0 / 60.0),
    /// ))).run();
}
