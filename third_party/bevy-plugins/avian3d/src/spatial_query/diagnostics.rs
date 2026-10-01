use bevy::prelude::Resource;

use crate::diagnostics::PhysicsDiagnostics;

/// Diagnostics for spatial queries. Its two caster timers went with `RayCaster` and `ShapeCaster`;
/// the resource and its reset system stay.
#[derive(Resource, Debug, Default)]
pub struct SpatialQueryDiagnostics;

impl PhysicsDiagnostics for SpatialQueryDiagnostics {}
