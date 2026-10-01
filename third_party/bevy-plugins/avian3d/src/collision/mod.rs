//! Collision detection for [`Collider`]s.
//!
//! Collision detection involves determining pairs of objects that may currently be in contact
//! (or are expected to come into contact), and computing contact data for each intersection.
//! These contacts are then used by the [solver](dynamics::solver) to generate [`ContactConstraint`]s
//! and finally resolve overlap.
//!
//! [`ContactConstraint`]: dynamics::solver::contact::ContactConstraint
//!
//! # Plugins
//!
//! In Avian, collision detection is split into two:
//!
//! - [`broad_phase`]: Finds pairs of entities with overlapping [AABBs](ColliderAabb) to reduce the number of potential contacts for the [narrow phase](narrow_phase).
//! - [`narrow_phase`]: Updates and manages contact pairs in the [`ContactGraph`].
//!
//! Spatial queries are handled separately by the [`SpatialQueryPlugin`].
//!
//! You can also find several utility methods for computing contacts in the [`contact_query`](collider::contact_query) module.
//!
//! # Accessing Collisions
//!
//! Contact pairs found by Avian are stored in the [`ContactGraph`] resource.
//! It contains all contacs between entities with overlapping [`ColliderAabb`]s,
//! including contacts where the colliders themselves may not be touching.
//!
//! # Contact Filtering and Modification
//!
//! Some advanced contact scenarios may need to filter or modify contacts
//! with user-defined logic. This can include:
//!
//! - One-way platforms
//! - Conveyor belts
//! - Non-uniform friction and restitution
//!
//! In Avian, this can be done by defining [`CollisionHooks`]. They let you hook into
//! the collision pipeline, and filter or modify contacts with (almost) full ECS access.
//!
//! See the documentation of [`CollisionHooks`] for more information and usage examples.

pub mod broad_phase;
pub mod collider;
pub mod contact_types;
pub mod hooks;
pub mod narrow_phase;

mod diagnostics;
pub use diagnostics::CollisionDiagnostics;

/// Re-exports common types related to collision detection functionality.
pub mod prelude {
    pub use super::broad_phase::{BroadPhaseCorePlugin, BroadPhaseSystems, BvhBroadPhasePlugin};
    pub use super::collider::Collider;
    pub use super::collider::ColliderCachePlugin;
    pub use super::collider::{
        AabbContext, AnyCollider, ColliderAabb, ColliderBackendPlugin, ColliderDisabled,
        ColliderMarker, CollisionLayers, ContactManifoldContext, IntoCollider, LayerMask,
        PhysicsLayer, ScalableCollider, Sensor, SimpleCollider,
        collider_hierarchy::{ColliderHierarchyPlugin, ColliderOf, RigidBodyColliders},
        collider_transform::{ColliderTransform, ColliderTransformPlugin},
    };
    pub use super::contact_types::{
        ContactEdge, ContactGraph, ContactManifold, ContactPair, ContactPairFlags, ContactPoint,
    };
    pub use super::hooks::{ActiveCollisionHooks, CollisionHooks};
    pub use super::narrow_phase::{NarrowPhaseConfig, NarrowPhasePlugin, NarrowPhaseSystems};
}

#[expect(unused_imports)]
use crate::prelude::*;
