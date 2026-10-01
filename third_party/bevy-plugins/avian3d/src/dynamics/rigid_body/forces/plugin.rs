use crate::{
    dynamics::{
        integrator::{self, IntegrationSystems},
        solver::solver_body::SolverBody,
    },
    prelude::*,
};
use bevy::prelude::*;

use super::AccumulatedLocalAcceleration;

/// A plugin for managing and applying external forces, torques, and accelerations for [rigid bodies](RigidBody).
///
/// See the [module-level documentation](crate::dynamics::rigid_body::forces) for more general information about forces in Avian.
pub struct ForcePlugin;

impl Plugin for ForcePlugin {
    fn build(&self, app: &mut App) {
        // Set up system sets.
        app.configure_sets(
            PhysicsSchedule,
            (
                ForceSystems::ApplyConstantForces
                    .in_set(IntegrationSystems::UpdateVelocityIncrements)
                    .before(integrator::pre_process_velocity_increments),
                ForceSystems::Clear.in_set(SolverSystems::PostSubstep),
            ),
        );
        app.configure_sets(
            SubstepSchedule,
            ForceSystems::ApplyLocalAcceleration
                .in_set(IntegrationSystems::Velocity)
                .before(integrator::integrate_velocities),
        );

        // Accumulate constant forces, torques, and accelerations.
        app.add_systems(
            PhysicsSchedule,
            (
                apply_constant_forces,
                apply_constant_torques,
                apply_constant_linear_acceleration,
                apply_constant_angular_acceleration,
                apply_constant_local_forces,
                apply_constant_local_torques,
                apply_constant_local_linear_acceleration,
                apply_constant_local_angular_acceleration,
            )
                .chain()
                .in_set(ForceSystems::ApplyConstantForces),
        );

        // Apply local forces and accelerations.
        // This is done in the substepping loop, because the orientations of bodies can change between substeps.
        app.add_systems(
            SubstepSchedule,
            apply_local_acceleration.in_set(ForceSystems::ApplyLocalAcceleration),
        );

        // Clear accumulated forces and accelerations.
        app.add_systems(
            PhysicsSchedule,
            clear_accumulated_local_acceleration.in_set(ForceSystems::Clear),
        );
    }
}

/// System sets for managing and applying forces, torques, and accelerations for [rigid bodies](RigidBody).
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ForceSystems {
    /// Holds the stand-ins of the constant force systems, which accumulated constant forces and
    /// accelerations into [`VelocityIntegrationData`](integrator::VelocityIntegrationData) and
    /// [`AccumulatedLocalAcceleration`].
    ApplyConstantForces,
    /// Applies [`AccumulatedLocalAcceleration`] to the linear and angular velocities of bodies.
    ApplyLocalAcceleration,
    /// Clears [`AccumulatedLocalAcceleration`] for all bodies.
    Clear,
}

// Stand-in for the system that added each `ConstantForce` to its body's linear velocity increment.
fn apply_constant_forces() {}

// Stand-in for the system that added each `ConstantTorque` to its body's angular velocity increment.
fn apply_constant_torques() {}

// Stand-in for the system that added each `ConstantLinearAcceleration` to its body's linear velocity increment.
fn apply_constant_linear_acceleration() {}

// Stand-in for the system that added each `ConstantAngularAcceleration` to its body's angular velocity increment.
fn apply_constant_angular_acceleration() {}

// Stand-in for the system that added each `ConstantLocalForce` to its body's accumulated local acceleration.
fn apply_constant_local_forces() {}

// Stand-in for the system that added each `ConstantLocalTorque` to its body's accumulated local acceleration.
fn apply_constant_local_torques() {}

// Stand-in for the system that added each `ConstantLocalLinearAcceleration` to its body's accumulated local acceleration.
fn apply_constant_local_linear_acceleration() {}

// Stand-in for the system that added each `ConstantLocalAngularAcceleration` to its body's accumulated local acceleration.
fn apply_constant_local_angular_acceleration() {}

/// Applies [`AccumulatedLocalAcceleration`] to the linear and angular velocity of bodies.
///
/// This should run in the substepping loop, just before [`IntegrationSystems::Velocity`].
fn apply_local_acceleration(
    mut bodies: Query<(&mut SolverBody, &AccumulatedLocalAcceleration, &Rotation)>,
    time: Res<Time<Substeps>>,
) {
    let delta_secs = time.delta_secs_f64() as Scalar;

    bodies
        .iter_mut()
        .for_each(|(mut body, acceleration, rotation)| {
            let rotation = body.delta_rotation * *rotation;
            let locked_axes = body.flags.locked_axes();

            // Compute the world space velocity increments with locked axes applied.
            let world_linear_acceleration =
                locked_axes.apply_to_vec(rotation * acceleration.linear);
            let world_angular_acceleration =
                locked_axes.apply_to_vec(rotation * acceleration.angular);

            // Apply acceleration.
            body.linear_velocity += world_linear_acceleration * delta_secs;
            {
                body.angular_velocity += world_angular_acceleration * delta_secs;
            }
        });
}

fn clear_accumulated_local_acceleration(mut query: Query<&mut AccumulatedLocalAcceleration>) {
    query.iter_mut().for_each(|mut acceleration| {
        acceleration.linear = Vector::ZERO;
        {
            acceleration.angular = Vector::ZERO;
        }
    });
}
