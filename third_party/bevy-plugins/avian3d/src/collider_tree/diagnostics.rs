use bevy::{
    prelude::{ReflectResource, Resource},
    reflect::Reflect,
};
use core::time::Duration;

use crate::diagnostics::PhysicsDiagnostics;

/// Diagnostics for [collider trees](crate::collider_tree).
#[derive(Resource, Debug, Default, Reflect)]
#[reflect(Resource, Debug)]
pub struct ColliderTreeDiagnostics {
    /// Time spent optimizing [collider trees](crate::collider_tree).
    pub optimize: Duration,
    /// Time spent updating AABBs and BVH nodes.
    pub update: Duration,
}

impl PhysicsDiagnostics for ColliderTreeDiagnostics {}
