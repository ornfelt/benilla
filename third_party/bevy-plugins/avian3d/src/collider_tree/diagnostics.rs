use bevy::{
    prelude::{ReflectResource, Resource},
    reflect::Reflect,
};

use crate::diagnostics::PhysicsDiagnostics;

/// Diagnostics for [collider trees](crate::collider_tree). Its optimize and update timers went,
/// nothing read them; the resource and its reset system stay.
#[derive(Resource, Debug, Default, Reflect)]
#[reflect(Resource, Debug)]
pub struct ColliderTreeDiagnostics;

impl PhysicsDiagnostics for ColliderTreeDiagnostics {}
