use core::time::Duration;

use bevy::{
    prelude::{ReflectResource, Resource},
    reflect::Reflect,
};

use crate::diagnostics::PhysicsDiagnostics;

/// Diagnostics for spatial queries.
#[derive(Resource, Debug, Default, Reflect)]
#[reflect(Resource, Debug)]
pub struct SpatialQueryDiagnostics {
    /// Time spent updating [`RayCaster`](super::RayCaster) hits.
    pub update_ray_casters: Duration,
    /// Time spent updating [`ShapeCaster`](super::ShapeCaster) hits.
    pub update_shape_casters: Duration,
}

impl PhysicsDiagnostics for SpatialQueryDiagnostics {}
