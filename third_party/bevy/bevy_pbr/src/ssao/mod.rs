use bevy_app::{App, Plugin};
use bevy_core_pipeline::prepass::{DepthPrepass, NormalPrepass};
use bevy_ecs::prelude::Component;
use bevy_render::{extract_component::ExtractComponent, sync_component::SyncComponentPlugin};

/// Plugin for screen space ambient occlusion.
pub struct ScreenSpaceAmbientOcclusionPlugin;

impl Plugin for ScreenSpaceAmbientOcclusionPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(SyncComponentPlugin::<ScreenSpaceAmbientOcclusion>::default());
    }
}

/// Component to apply screen space ambient occlusion to a 3d camera.
///
/// Screen space ambient occlusion (SSAO) approximates small-scale,
/// local occlusion of _indirect_ diffuse light between objects, based on what's visible on-screen.
/// SSAO does not apply to direct lighting, such as point or directional lights.
///
/// This darkens creases, e.g. on staircases, and gives nice contact shadows
/// where objects meet, giving entities a more "grounded" feel.
///
/// # Usage Notes
///
/// Requires that you add [`ScreenSpaceAmbientOcclusionPlugin`] to your app.
///
/// It strongly recommended that you use SSAO in conjunction with
/// TAA (`TemporalAntiAliasing`).
/// Doing so greatly reduces SSAO noise.
///
/// SSAO is not supported on `WebGL2`, and is not currently supported on `WebGPU`.
#[derive(Component, ExtractComponent, PartialEq, Clone, Debug, Default)]
#[require(DepthPrepass, NormalPrepass)]
#[doc(alias = "Ssao")]
pub struct ScreenSpaceAmbientOcclusion;
