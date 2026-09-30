use bevy::{
    prelude::{ReflectResource, Resource},
    reflect::Reflect,
};

use crate::diagnostics::PhysicsDiagnostics;

/// Diagnostics for spatial queries. Its two caster timers went with `RayCaster` and `ShapeCaster`;
/// the resource and its reset system stay.
#[derive(Resource, Debug, Default, Reflect)]
#[reflect(Resource, Debug)]
pub struct SpatialQueryDiagnostics;

impl PhysicsDiagnostics for SpatialQueryDiagnostics {}
