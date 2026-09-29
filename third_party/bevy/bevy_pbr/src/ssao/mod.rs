use bevy_app::{App, Plugin};
use bevy_core_pipeline::prepass::{DepthPrepass, NormalPrepass};
use bevy_ecs::{prelude::Component, reflect::ReflectComponent};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
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
#[derive(Component, ExtractComponent, Reflect, PartialEq, Clone, Debug)]
#[reflect(Component, Debug, Default, PartialEq, Clone)]
#[require(DepthPrepass, NormalPrepass)]
#[doc(alias = "Ssao")]
pub struct ScreenSpaceAmbientOcclusion {
    /// Quality of the SSAO effect.
    pub quality_level: ScreenSpaceAmbientOcclusionQualityLevel,
    /// A constant estimated thickness of objects.
    ///
    /// This value is used to decide how far behind an object a ray of light needs to be in order
    /// to pass behind it. Any ray closer than that will be occluded.
    pub constant_object_thickness: f32,
}

impl Default for ScreenSpaceAmbientOcclusion {
    fn default() -> Self {
        Self {
            quality_level: ScreenSpaceAmbientOcclusionQualityLevel::default(),
            constant_object_thickness: 0.25,
        }
    }
}

#[derive(Reflect, PartialEq, Eq, Hash, Clone, Copy, Default, Debug)]
#[reflect(PartialEq, Hash, Clone, Default)]
pub enum ScreenSpaceAmbientOcclusionQualityLevel {
    Low,
    Medium,
    #[default]
    High,
    Ultra,
    Custom {
        /// Higher slice count means less noise, but worse performance.
        slice_count: u32,
        /// Samples per slice side is also tweakable, but recommended to be left at 2 or 3.
        samples_per_slice_side: u32,
    },
}
