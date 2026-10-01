#![allow(missing_docs)]

use crate::prelude::*;
use bevy::{
    ecs::query::QueryData,
    prelude::{Entity, Has, Ref},
};

/// A `WorldQuery` to make querying and modifying rigid bodies more convenient.
#[derive(QueryData)]
#[query_data(mutable)]
pub struct RigidBodyQuery {
    pub entity: Entity,
    pub rb: Ref<'static, RigidBody>,
    pub position: &'static mut Position,
    pub rotation: &'static mut Rotation,
    pub linear_velocity: &'static mut LinearVelocity,
    pub angular_velocity: &'static mut AngularVelocity,
    pub mass: &'static mut ComputedMass,
    pub angular_inertia: &'static mut ComputedAngularInertia,
    pub center_of_mass: &'static mut ComputedCenterOfMass,
    pub friction: Option<&'static Friction>,
    pub restitution: Option<&'static Restitution>,
    pub locked_axes: Option<&'static LockedAxes>,
    pub is_sleeping: Has<Sleeping>,
    pub is_sensor: Has<Sensor>,
}
