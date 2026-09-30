use bevy::{
    prelude::{ReflectResource, Resource},
    reflect::Reflect,
};
use core::time::Duration;

use crate::diagnostics::PhysicsDiagnostics;

/// Diagnostics for the physics solver.
#[derive(Resource, Debug, Default, Reflect)]
#[reflect(Resource, Debug)]
pub struct SolverDiagnostics {
    /// Time spent preparing constraints.
    pub prepare_constraints: Duration,
    /// Time spent preparing or clearing velocity increments in [`VelocityIntegrationData`]s.
    ///
    /// [`VelocityIntegrationData`]: crate::dynamics::integrator::VelocityIntegrationData
    pub update_velocity_increments: Duration,
    /// Time spent integrating velocities.
    pub integrate_velocities: Duration,
    /// Time spent warm starting the solver.
    pub warm_start: Duration,
    /// Time spent solving constraints with bias.
    pub solve_constraints: Duration,
    /// Time spent integrating positions.
    pub integrate_positions: Duration,
    /// Time spent relaxing velocities.
    pub relax_velocities: Duration,
    /// Time spent applying restitution.
    pub apply_restitution: Duration,
    /// Time spent writing the final results to the bodies.
    pub finalize: Duration,
    /// Time spent storing impulses for warm starting.
    pub store_impulses: Duration,
    /// Time spent on swept CCD.
    pub swept_ccd: Duration,
    /// The number of contact constraints generated.
    pub contact_constraint_count: u32,
}

impl PhysicsDiagnostics for SolverDiagnostics {}
