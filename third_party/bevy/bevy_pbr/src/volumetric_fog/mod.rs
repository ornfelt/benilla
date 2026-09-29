//! Volumetric fog and volumetric lighting, also known as light shafts or god
//! rays.
//!
//! This module implements a more physically-accurate, but slower, form of fog
//! than the [`crate::fog`] module does. Notably, this *volumetric fog* allows
//! for light beams from directional lights to shine through, creating what is
//! known as *light shafts* or *god rays*.
//!
//! To add volumetric fog to a scene, add [`bevy_light::VolumetricFog`] to the
//! camera, and add [`bevy_light::VolumetricLight`] to directional lights that you wish to
//! be volumetric. [`bevy_light::VolumetricFog`] feature numerous settings that
//! allow you to define the accuracy of the simulation, as well as the look of
//! the fog. Currently, only interaction with directional lights that have
//! shadow maps is supported. Note that the overhead of the effect scales
//! directly with the number of directional lights in use, so apply
//! [`bevy_light::VolumetricLight`] sparingly for the best results.
//!
//! The overall algorithm, which is implemented as a postprocessing effect, is a
//! combination of the techniques described in [Scratchapixel] and [this blog
//! post]. It uses raymarching in screen space, transformed into shadow map
//! space for sampling and combined with physically-based modeling of absorption
//! and scattering. Bevy employs the widely-used [Henyey-Greenstein phase
//! function] to model asymmetry; this essentially allows light shafts to fade
//! into and out of existence as the user views them.
//!
//! [Scratchapixel]: https://www.scratchapixel.com/lessons/3d-basic-rendering/volume-rendering-for-developers/intro-volume-rendering.html
//!
//! [this blog post]: https://www.alexandre-pestana.com/volumetric-lights/
//!
//! [Henyey-Greenstein phase function]: https://www.pbr-book.org/4ed/Volume_Scattering/Phase_Functions#TheHenyeyndashGreensteinPhaseFunction

use bevy_app::{App, Plugin};
use bevy_asset::Assets;
use bevy_light::FogVolume;
use bevy_math::{
    primitives::{Cuboid, Plane3d},
    Vec2, Vec3,
};
use bevy_mesh::{Mesh, Meshable};
use bevy_render::sync_component::SyncComponentPlugin;

/// A plugin that implements volumetric fog.
pub struct VolumetricFogPlugin;

impl Plugin for VolumetricFogPlugin {
    fn build(&self, app: &mut App) {
        // The fog meshes were handed to the render world; with no RenderApp under gfx they are
        // added and dropped here, as before.
        let mut meshes = app.world_mut().resource_mut::<Assets<Mesh>>();
        let _plane_mesh = meshes.add(Plane3d::new(Vec3::Z, Vec2::ONE).mesh());
        let _cube_mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0).mesh());

        app.add_plugins(SyncComponentPlugin::<FogVolume>::default());
    }
}
