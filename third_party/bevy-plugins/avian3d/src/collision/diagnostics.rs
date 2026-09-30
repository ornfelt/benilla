use bevy::{
    prelude::{ReflectResource, Resource},
    reflect::Reflect,
};
use core::time::Duration;

use crate::diagnostics::PhysicsDiagnostics;

/// Diagnostics for collision detection.
#[derive(Resource, Debug, Default, Reflect)]
#[reflect(Resource, Debug)]
pub struct CollisionDiagnostics {
    /// Time spent finding potential collision pairs in the [broad phase](crate::collision::broad_phase).
    pub broad_phase: Duration,
    /// Time spent updating contacts in the [narrow phase](crate::collision::narrow_phase).
    pub narrow_phase: Duration,
    /// The number of contacts.
    pub contact_count: u32,
}

impl PhysicsDiagnostics for CollisionDiagnostics {}
