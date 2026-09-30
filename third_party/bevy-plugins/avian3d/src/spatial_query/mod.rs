//! Functionality for performing ray casts, shape casts, and other spatial queries.
//!
//! Spatial queries query the world for geometric information about [`Collider`s](Collider)
//! and various types of intersections. Currently, four types of spatial queries are supported:
//!
//! - [Raycasts](#raycasting)
//! - [Shapecasts](#shapecasting)
//! - [Point projection](#point-projection)
//! - [Intersection tests](#intersection-tests)
//!
//! All spatial queries can be done using the various methods provided by the [`SpatialQuery`] system parameter.
//!
//! # Raycasting
//!
//! **Raycasting** is a spatial query that finds intersections between colliders and a half-line. This can be used for
//! a variety of things like getting information about the environment for character controllers and AI,
//! and even rendering using ray tracing.
//!
//! For each hit during raycasting, the hit entity, a distance, and a normal will be stored in [`RayHitData`].
//! The distance is the distance from the ray origin to the point of intersection, indicating how far the ray travelled.
//!
//! Raycasts are performed with the raycasting methods provided by [`SpatialQuery`], like
//! [`cast_ray`](SpatialQuery::cast_ray).
//!
//! To specify which colliders should be considered in the query, use a [spatial query filter](`SpatialQueryFilter`).
//!
//! # Shapecasting
//!
//! **Shapecasting** or **sweep testing** is a spatial query that finds intersections between colliders and a shape
//! that is travelling along a half-line. It is very similar to [raycasting](#raycasting), but instead of a "point"
//! we have an entire shape travelling along a half-line. One use case is determining how far an object can move
//! before it hits the environment.
//!
//! For each hit during shapecasting, the hit entity, a distance, two world-space points of intersection and two world-space
//! normals will be stored in [`ShapeHitData`]. The distance refers to how far the shape travelled before the initial hit.
//!
//! Shapecasts are performed with the shapecasting methods provided by [`SpatialQuery`], like
//! [`cast_shape_predicate`](SpatialQuery::cast_shape_predicate).
//!
//! To specify which colliders should be considered in the query, use a [spatial query filter](`SpatialQueryFilter`).
//!
//! # Intersection tests
//!
//! **Intersection tests** are spatial queries that return the entities of colliders that are intersecting a given
//! shape or area.
//!
//! Intersection tests are methods of the [`SpatialQuery`] system parameter:
//!
//! - [`aabb_intersections_with_aabb`](SpatialQuery::aabb_intersections_with_aabb):
//!   Finds all entities with a [`ColliderAabb`] that is intersecting the given [`ColliderAabb`].
//!
//! See the documentation of the components and methods for more information.
//!
//! To specify which colliders should be considered in the query, use a [spatial query filter](`SpatialQueryFilter`).

mod query_filter;
mod ray_caster;
mod shape_caster;
mod system_param;

mod diagnostics;
pub use diagnostics::SpatialQueryDiagnostics;

pub use query_filter::*;
pub use ray_caster::*;
pub use shape_caster::*;
pub use system_param::*;

use crate::prelude::*;
use bevy::prelude::*;

/// Runs the component-based casts' four systems in [`PhysicsStepSystems::SpatialQuery`], empty since the
/// `RayCaster` and `ShapeCaster` components are gone.
pub struct SpatialQueryPlugin;

impl Plugin for SpatialQueryPlugin {
    fn build(&self, app: &mut App) {
        let physics_schedule = app
            .get_schedule_mut(PhysicsSchedule)
            .expect("add PhysicsSchedule first");

        physics_schedule.add_systems(
            (
                update_ray_caster_positions,
                (update_shape_caster_positions, raycast, shapecast).chain(),
            )
                .chain()
                .in_set(PhysicsStepSystems::SpatialQuery),
        );
    }

    fn finish(&self, app: &mut App) {
        // Register timer diagnostics for spatial queries.
        app.register_physics_diagnostics::<SpatialQueryDiagnostics>();
    }
}

// Stand-in for the system that placed each `RayCaster` at its entity's position.
fn update_ray_caster_positions() {}

// Stand-in for the system that placed each `ShapeCaster` at its entity's position.
fn update_shape_caster_positions() {}

// Stand-in for the system that cast each `RayCaster` into its `RayHits`.
fn raycast() {}

// Stand-in for the system that cast each `ShapeCaster` into its `ShapeHits`.
fn shapecast() {}
