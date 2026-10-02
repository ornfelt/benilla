//! Procedural Atmospheric Scattering.
//!
//! This plugin implements [Hillaire's 2020 paper](https://sebh.github.io/publications/egsr2020.pdf)
//! on real-time atmospheric scattering. While it *will* work simply as a
//! procedural skybox, it also does much more. It supports dynamic time-of-
//! -day, multiple directional lights, and since it's applied as a post-processing
//! effect *on top* of the existing skybox, a starry skybox would automatically
//! show based on the time of day. Scattering in front of terrain (similar
//! to distance fog, but more complex) is handled as well, and takes into
//! account the directional light color and direction.
//!
//! Adding the [`Atmosphere`] component to a 3d camera will enable the effect,
//! which by default is set to look similar to Earth's atmosphere. See the
//! documentation on the component itself for information regarding its fields.
//!
//! Performance-wise, the effect should be fairly cheap since the LUTs (Look
//! Up Tables) that encode most of the data are small, and take advantage of the
//! fact that the atmosphere is symmetric. Performance is also proportional to
//! the number of directional lights in the scene. In order to tune
//! performance more finely, the [`AtmosphereSettings`] camera component
//! manages the size of each LUT and the sample count for each ray.
//!
//! Given how similar it is to [`crate::volumetric_fog`], it might be expected
//! that these two modules would work together well. However for now using both
//! at once is untested, and might not be physically accurate. These may be
//! integrated into a single module in the future.
//!
//! On web platforms, atmosphere rendering will look slightly different. Specifically, when calculating how light travels
//! through the atmosphere, we use a simpler averaging technique instead of the more
//! complex blending operations. This difference will be resolved for WebGPU in a future release.
//!
//! [Shadertoy]: https://www.shadertoy.com/view/slSXRW
//!
//! [Unreal Engine Implementation]: https://github.com/sebh/UnrealEngineSkyAtmosphere

mod environment;

use bevy_app::{App, Plugin, Update};
use bevy_ecs::component::Component;
use bevy_render::{sync_component::SyncComponentPlugin, view::Hdr};

use environment::{prepare_atmosphere_probe_components, AtmosphereEnvironmentMap};

#[doc(hidden)]
pub struct AtmospherePlugin;

impl Plugin for AtmospherePlugin {
    fn build(&self, app: &mut App) {
        // The main-world halves of the three `ExtractComponentPlugin`s; the uniform plugins and
        // `finish` only reached the RenderApp, which gfx does not have.
        app.add_plugins((
            SyncComponentPlugin::<Atmosphere>::default(),
            SyncComponentPlugin::<GpuAtmosphereSettings>::default(),
            SyncComponentPlugin::<AtmosphereEnvironmentMap>::default(),
        ))
        .add_systems(Update, prepare_atmosphere_probe_components);
    }
}

/// Enables atmospheric scattering for an HDR camera.
#[derive(Clone, Component)]
#[require(AtmosphereSettings, Hdr)]
pub struct Atmosphere;

/// This component controls the resolution of the atmosphere LUTs, and
/// how many samples are used when computing them.
#[derive(Clone, Component, Default)]
pub struct AtmosphereSettings;

#[derive(Clone, Component, Default)]
pub struct GpuAtmosphereSettings;
