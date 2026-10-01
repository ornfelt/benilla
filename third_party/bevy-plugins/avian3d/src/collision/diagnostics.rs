use bevy::{
    prelude::{ReflectResource, Resource},
    reflect::Reflect,
};

use crate::diagnostics::PhysicsDiagnostics;

/// Diagnostics for collision detection. Its broad-phase and narrow-phase timers and the contact
/// count went, nothing read them; the resource and its reset system stay.
#[derive(Resource, Debug, Default, Reflect)]
#[reflect(Resource, Debug)]
pub struct CollisionDiagnostics;

impl PhysicsDiagnostics for CollisionDiagnostics {}
