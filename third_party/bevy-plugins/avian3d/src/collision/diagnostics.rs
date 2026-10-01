use bevy::prelude::Resource;

use crate::diagnostics::PhysicsDiagnostics;

/// Diagnostics for collision detection. Its broad-phase and narrow-phase timers and the contact
/// count went, nothing read them; the resource and its reset system stay.
#[derive(Resource, Debug, Default)]
pub struct CollisionDiagnostics;

impl PhysicsDiagnostics for CollisionDiagnostics {}
