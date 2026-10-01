use bevy::prelude::Resource;

use crate::diagnostics::PhysicsDiagnostics;

/// Diagnostics for [collider trees](crate::collider_tree). Its optimize and update timers went,
/// nothing read them; the resource and its reset system stay.
#[derive(Resource, Debug, Default)]
pub struct ColliderTreeDiagnostics;

impl PhysicsDiagnostics for ColliderTreeDiagnostics {}
