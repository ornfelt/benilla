//! External forces, impulses, and acceleration for dynamic [rigid bodies](RigidBody).
//!
//! # Overview
//!
//! In addition to [`Gravity`], it is possible to apply your own forces, impulses, and acceleration
//! to dynamic rigid bodies to simulate various effects such as force fields, motors, and thrusters.
//! They have the following relationships:
//!
//! | Type             | Formula      | Relation to Velocity | Unit        |
//! | ---------------- | ------------ | -------------------- | ----------- |
//! | **Force**        | `F = m * a`  | `Δa = F / m`         | kg⋅m/s² (N) |
//! | **Impulse**      | `J = F * Δt` | `Δv = J / m`         | kg⋅m/s (N⋅s) |
//! | **Acceleration** | `a = F / m`  | `Δv = a * Δt`        | m/s²        |
//!
//! A force applies an acceleration to a body, which in turn modifies its velocity over time,
//! while an impulse applies an immediate change in velocity. Both forces and impulses consider [mass properties],
//! while acceleration by itself is independent of mass.
//!
//! The rotational equivalents are torques, angular impulses, and angular acceleration, which work similarly.
//!
//! [mass properties]: crate::dynamics::rigid_body::mass_properties

mod plugin;

pub use plugin::{ForcePlugin, ForceSystems};

use crate::prelude::*;
use bevy::prelude::*;

/// A component with the user-applied local acceleration
/// accumulated for a rigid body before the physics step.
#[derive(Component, Clone, Debug, Default, PartialEq, Reflect)]
#[reflect(Component, Debug, Default, PartialEq)]
pub struct AccumulatedLocalAcceleration {
    /// The accumulated linear acceleration in local space.
    pub linear: Vector,
    /// The accumulated angular acceleration in local space.
    pub angular: Vector,
}
