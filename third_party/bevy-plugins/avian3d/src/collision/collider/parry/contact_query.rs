//! Geometric queries for computing information about contacts between two [`Collider`]s.
//!
//! This module contains the following contact queries:
//!
//! | Contact query         | Description                                                               |
//! | --------------------- | ------------------------------------------------------------------------- |
//! | [`contact_manifolds`] | Computes all [`ContactManifold`]s between two [`Collider`]s.              |
//!
//! For geometric queries that query the entire world for intersections, like raycasting, shapecasting
//! and point projection, see [spatial queries](spatial_query).

use crate::prelude::*;
use bevy::prelude::*;
use parry::query::{PersistentQueryDispatcher, Unsupported};

/// An error indicating that a [contact query](self) is not supported for one of the [`Collider`] shapes.
pub type UnsupportedShape = Unsupported;

// TODO: Add a persistent version of this that tries to reuse previous contact manifolds
// by exploiting spatial and temporal coherence. This is supported by Parry's contact_manifolds,
// but requires using Parry's `ContactManifold` type.
/// Computes all [`ContactManifold`]s between two [`Collider`]s.
///
/// The contact points are expressed in world space, relative to the origin
/// of the first and second shape respectively.
///
/// Returns an empty vector if the colliders are separated by a distance greater than `prediction_distance`
/// or if the given shapes are invalid.
///
/// # Example
///
/// ```
/// # #[cfg(feature = "2d")]
/// # use avian2d::{collision::collider::contact_query::contact_manifolds, prelude::*};
/// # #[cfg(feature = "3d")]
/// use avian3d::{collision::collider::contact_query::contact_manifolds, prelude::*};
/// use bevy::prelude::*;
///
/// # #[cfg(all(feature = "3d", feature = "f32"))]
/// # {
/// let collider1 = Collider::sphere(0.5);
/// let collider2 = Collider::cuboid(1.0, 1.0, 1.0);
///
/// // Compute contact manifolds a collision that should be penetrating
/// let mut manifolds = Vec::new();
/// contact_manifolds(
///     // First collider
///     &collider1,
///     Vec3::default(),
///     Quat::default(),
///     // Second collider
///     &collider2,
///     Vec3::X * 0.25,
///     Quat::default(),
///     // Prediction distance
///     0.0,
///     // Output manifolds
///     &mut manifolds,
/// );
///
/// assert_eq!(manifolds.is_empty(), false);
/// # }
/// ```
pub fn contact_manifolds(
    collider1: &Collider,
    position1: impl Into<Position>,
    rotation1: impl Into<Rotation>,
    collider2: &Collider,
    position2: impl Into<Position>,
    rotation2: impl Into<Rotation>,
    prediction_distance: Scalar,
    manifolds: &mut Vec<ContactManifold>,
) {
    let position1: Position = position1.into();
    let position2: Position = position2.into();
    let rotation1: Rotation = rotation1.into();
    let rotation2: Rotation = rotation2.into();
    let isometry1 = make_pose(position1, rotation1);
    let isometry2 = make_pose(position2, rotation2);
    let isometry12 = isometry1.inv_mul(&isometry2);

    // TODO: Reuse manifolds from previous frame to improve performance
    let mut new_manifolds =
        Vec::<parry::query::ContactManifold<(), ()>>::with_capacity(manifolds.len());
    let result = parry::query::DefaultQueryDispatcher.contact_manifolds(
        &isometry12,
        collider1.shape_scaled().0.as_ref(),
        collider2.shape_scaled().0.as_ref(),
        prediction_distance,
        &mut new_manifolds,
        &mut None,
    );

    // Clear the old manifolds.
    manifolds.clear();

    // Fall back to support map contacts for unsupported (custom) shapes.
    if result.is_err()
        && let (Some(shape1), Some(shape2)) = (
            collider1.shape_scaled().as_support_map(),
            collider2.shape_scaled().as_support_map(),
        )
        && let Some(contact) = parry::query::contact::contact_support_map_support_map(
            &isometry12,
            shape1,
            shape2,
            prediction_distance,
        )
    {
        let normal = rotation1 * contact.normal1;

        // Make sure the normal is valid
        if !normal.is_normalized() {
            return;
        }

        let local_point1: Vector = contact.point1;

        // The contact point is the midpoint of the two points in world space.
        // The anchors are relative to the positions of the colliders.
        let point1 = rotation1 * local_point1;
        let anchor1 = point1 + normal * contact.dist * 0.5;
        let anchor2 = anchor1 + (position1.0 - position2.0);
        let world_point = position1.0 + anchor1;
        let points = [ContactPoint::new(
            anchor1,
            anchor2,
            world_point,
            -contact.dist,
        )];

        manifolds.push(ContactManifold::new(points, normal));
    }

    manifolds.extend(new_manifolds.iter().filter_map(|manifold| {
        // Skip empty manifolds.
        if manifold.contacts().is_empty() {
            return None;
        }

        let subpos1 = manifold.subshape_pos1.unwrap_or_default();
        let local_normal: Vector = (subpos1.rotation * manifold.local_n1).normalize();
        let normal = rotation1 * local_normal;

        // Make sure the normal is valid
        if !normal.is_normalized() {
            return None;
        }

        let points = manifold.contacts().iter().map(|contact| {
            // The contact point is the midpoint of the two points in world space.
            // The anchors are relative to the positions of the colliders.
            let point1 = rotation1 * subpos1.transform_point(contact.local_p1);
            let anchor1 = point1 + normal * contact.dist * 0.5;
            let anchor2 = anchor1 + (position1.0 - position2.0);
            let world_point = position1.0 + anchor1;
            ContactPoint::new(anchor1, anchor2, world_point, -contact.dist)
                .with_feature_ids(contact.fid1.into(), contact.fid2.into())
        });

        let manifold = ContactManifold::new(points, normal);

        Some(manifold)
    }));
}

/// Information about the closest points between two [`Collider`]s.
#[derive(Reflect, Clone, Copy, Debug, PartialEq)]
#[reflect(Debug, PartialEq)]
pub enum ClosestPoints {
    /// The two shapes are intersecting each other.
    Intersecting,
    /// The two shapes are not intersecting each other but the distance between the closest points
    /// is below the user-defined maximum distance.
    ///
    /// The points are expressed in world space.
    WithinMargin(Vector, Vector),
    /// The two shapes are not intersecting each other and the distance between the closest points
    /// exceeds the user-defined maximum distance.
    OutsideMargin,
}
