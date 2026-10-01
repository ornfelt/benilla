use bevy::{
    prelude::{ReflectResource, Resource},
    reflect::Reflect,
};

use crate::diagnostics::PhysicsDiagnostics;

/// Diagnostics for the physics solver. Its eleven stage timers and the contact constraint count
/// went, nothing read them; the resource and its reset system stay.
#[derive(Resource, Debug, Default, Reflect)]
#[reflect(Resource, Debug)]
pub struct SolverDiagnostics;

impl PhysicsDiagnostics for SolverDiagnostics {}
