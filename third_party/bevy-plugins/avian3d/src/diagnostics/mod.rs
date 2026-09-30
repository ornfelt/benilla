//! Diagnostics support for tracking physics timers and counters. Useful for profiling and debugging.
//!
//! # Overview
//!
//! Each physics plugin such as [`NarrowPhasePlugin`] and [`SolverPlugin`] is responsible
//! for implementing its own diagnostics resource using the [`PhysicsDiagnostics`] trait
//! and registering it using [`AppDiagnosticsExt::register_physics_diagnostics`].
//!
//! If the `bevy_diagnostic` feature is enabled and the [`PhysicsDiagnosticsPlugin`] is added to the app,
//! these diagnostics will also be automatically written to the [`DiagnosticsStore`] resource.
//!
//! [`NarrowPhasePlugin`]: crate::collision::narrow_phase::NarrowPhasePlugin
//! [`SolverPlugin`]: crate::dynamics::solver::SolverPlugin
//! [`DiagnosticsStore`]: bevy::diagnostic::DiagnosticsStore
//!
//! # Example
//!
//! ```no_run
//! use avian3d::prelude::*;
//! use bevy::prelude::*;
//!
//! fn main() {
//!     App::new()
//!         .add_plugins((
//!             DefaultPlugins,
//!             PhysicsPlugins::default(),
//!             // Add the `PhysicsDiagnosticsPlugin` to write physics diagnostics
//!             // to the `DiagnosticsStore` resource in `bevy_diagnostic`.
//!             // Requires the `bevy_diagnostic` feature.
//! #           #[cfg(feature = "bevy_diagnostic")]
//!             PhysicsDiagnosticsPlugin,
//!         ))
//!         // ...your other plugins, systems and resources
//!         .run();
//! }
//! ```
//!
//! # Supported Diagnostics
//!
//! The following diagnostics are available but not added by default:
//!
//! - [`PhysicsTotalDiagnostics`]: Total physics timers and counters.
//! - [`PhysicsEntityDiagnostics`]: Physics entity counters.
//!
//! Additionally, physics plugins may implement their own diagnostics that *are* enabled by default.
//! These include:
//!
//! - [`CollisionDiagnostics`]: Diagnostics for collision detection.
//! - [`SolverDiagnostics`]: Diagnostics for the physics solver.
//! - [`SpatialQueryDiagnostics`]: Diagnostics for spatial queries.
//!
//! [`CollisionDiagnostics`]: crate::collision::CollisionDiagnostics
//! [`SolverDiagnostics`]: crate::dynamics::solver::SolverDiagnostics
//! [`SpatialQueryDiagnostics`]: crate::spatial_query::SpatialQueryDiagnostics

mod path_macro;

pub(crate) use path_macro::impl_diagnostic_paths;

use crate::{PhysicsStepSystems, schedule::PhysicsSchedule};
use bevy::{
    diagnostic::DiagnosticPath,
    prelude::{App, IntoScheduleConfigs, ResMut, Resource, SystemSet},
};
use core::time::Duration;

/// A system set for [physics diagnostics](crate::diagnostics).
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PhysicsDiagnosticsSystems {
    /// Resets diagnostics to their default values.
    Reset,
    /// Writes physics diagnostics to other resources, commonly `DiagnosticsStore`.
    WriteDiagnostics,
}

/// A trait for resources storing timers and counters for [physics diagnostics](crate::diagnostics).
pub trait PhysicsDiagnostics: Default + Resource {
    /// Maps diagnostic paths to their respective duration fields.
    fn timer_paths(&self) -> Vec<(&'static DiagnosticPath, Duration)> {
        Vec::new()
    }

    /// Maps diagnostic paths to their respective counter fields.
    fn counter_paths(&self) -> Vec<(&'static DiagnosticPath, u32)> {
        Vec::new()
    }

    /// A system that resets the diagnostics to their default values.
    fn reset(mut physics_diagnostics: ResMut<Self>) {
        *physics_diagnostics = Self::default();
    }
}

/// An extension trait for registering [physics diagnostics](crate::diagnostics) in an [`App`].
pub trait AppDiagnosticsExt {
    /// Registers timer and counter diagnostics for a resource implementing [`PhysicsDiagnostics`].
    ///
    /// This method should be called in [`Plugin::finish`] to ensure that [`Diagnostic`]s
    /// are only tracked if the [`PhysicsDiagnosticsPlugin`] was added to the app.
    fn register_physics_diagnostics<T: PhysicsDiagnostics>(&mut self);
}

impl AppDiagnosticsExt for App {
    fn register_physics_diagnostics<T: PhysicsDiagnostics>(&mut self) {
        // Avoid duplicate registrations.
        if self.world().is_resource_added::<T>() {
            return;
        }

        // Initialize the diagnostics resource.
        self.init_resource::<T>();

        // Make sure the system set exists, even if `PhysicsDiagnosticsPlugin` is not added.
        self.configure_sets(
            PhysicsSchedule,
            PhysicsDiagnosticsSystems::Reset.before(PhysicsStepSystems::First),
        );

        // Add a system to reset the resource, even if `PhysicsDiagnosticsPlugin` is not added.
        self.add_systems(
            PhysicsSchedule,
            T::reset
                .in_set(PhysicsDiagnosticsSystems::Reset)
                .ambiguous_with_all(),
        );
    }
}
