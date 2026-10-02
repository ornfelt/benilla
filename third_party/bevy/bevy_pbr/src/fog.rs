use bevy_camera::Camera;
use bevy_ecs::prelude::*;
use bevy_render::extract_component::ExtractComponent;

/// Configures the “classic” computer graphics [distance fog](https://en.wikipedia.org/wiki/Distance_fog) effect,
/// in which objects appear progressively more covered in atmospheric haze the further away they are from the camera.
/// Affects meshes rendered via the PBR [`StandardMaterial`](crate::StandardMaterial).
///
/// ## Material Override
///
/// Once enabled for a specific camera, the fog effect can also be disabled for individual
/// [`StandardMaterial`](crate::StandardMaterial) instances via the `fog_enabled` flag.
#[derive(Debug, Clone, Default, Component, ExtractComponent)]
#[extract_component_filter(With<Camera>)]
pub struct DistanceFog;
