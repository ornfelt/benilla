//! Screen space reflections implemented via raymarching.

use bevy_app::{App, Plugin};
use bevy_core_pipeline::prepass::{DeferredPrepass, DepthPrepass};
use bevy_ecs::component::Component;
use bevy_render::sync_component::SyncComponentPlugin;

/// Enables screen-space reflections for a camera.
///
/// Screen-space reflections are currently only supported with deferred rendering.
pub struct ScreenSpaceReflectionsPlugin;

/// Add this component to a camera to enable *screen-space reflections* (SSR).
///
/// Screen-space reflections currently require deferred rendering in order to
/// appear. Therefore, they also need the [`DepthPrepass`] and [`DeferredPrepass`]
/// components, which are inserted automatically,
/// but deferred rendering itself is not automatically enabled.
///
/// SSR currently performs no roughness filtering for glossy reflections, so
/// only very smooth surfaces will reflect objects in screen space.
///
/// As with all screen-space techniques, SSR can only reflect objects on screen.
/// When objects leave the camera, they will disappear from reflections.
/// An alternative that doesn't suffer from this problem is the combination of
/// a [`LightProbe`](bevy_light::LightProbe) and [`EnvironmentMapLight`]. The advantage of SSR is
/// that it can reflect all objects, not just static ones.
///
/// SSR is an approximation technique and produces artifacts in some situations.
///
/// Screen-space reflections are presently unsupported on WebGL 2 because of a
/// bug whereby Naga doesn't generate correct GLSL when sampling depth buffers,
/// which is required for screen-space raymarching.
#[derive(Clone, Copy, Component, Default)]
#[require(DepthPrepass, DeferredPrepass)]
#[doc(alias = "Ssr")]
pub struct ScreenSpaceReflections;

impl Plugin for ScreenSpaceReflectionsPlugin {
    fn build(&self, app: &mut App) {
        // `ExtractComponentPlugin`'s main-world half: no RenderApp exists under gfx.
        app.add_plugins(SyncComponentPlugin::<ScreenSpaceReflections>::default());
    }
}
