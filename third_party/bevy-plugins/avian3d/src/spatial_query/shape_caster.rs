use crate::prelude::*;
use bevy::prelude::*;

/// Configuration for a shape cast.
#[derive(Clone, Debug, PartialEq, Reflect)]
#[reflect(Debug, PartialEq)]
pub struct ShapeCastConfig {
    /// The maximum distance the shape can travel.
    ///
    /// By default, this is infinite.
    #[doc(alias = "max_time_of_impact")]
    pub max_distance: Scalar,

    /// The separation distance at which the shapes will be considered as impacting.
    ///
    /// If the shapes are separated by a distance smaller than `target_distance` at the origin of the cast,
    /// the computed contact points and normals are only reliable if [`ShapeCastConfig::compute_contact_on_penetration`]
    /// is set to `true`.
    ///
    /// By default, this is `0.0`, so the shapes will only be considered as impacting when they first touch.
    pub target_distance: Scalar,

    /// If `true`, contact points and normals will be calculated even when the cast distance is `0.0`.
    ///
    /// The default is `true`.
    pub compute_contact_on_penetration: bool,

    /// If `true` *and* the shape is travelling away from the object that was hit,
    /// the cast will ignore any impact that happens at the cast origin.
    ///
    /// The default is `false`.
    pub ignore_origin_penetration: bool,
}

impl Default for ShapeCastConfig {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl ShapeCastConfig {
    /// The default [`ShapeCastConfig`] configuration.
    pub const DEFAULT: Self = Self {
        max_distance: Scalar::MAX,
        target_distance: 0.0,
        compute_contact_on_penetration: true,
        ignore_origin_penetration: false,
    };

    /// Creates a new [`ShapeCastConfig`] with a given maximum distance the shape can travel.
    #[inline]
    pub const fn from_max_distance(max_distance: Scalar) -> Self {
        Self {
            max_distance,
            target_distance: 0.0,
            compute_contact_on_penetration: true,
            ignore_origin_penetration: false,
        }
    }
}

/// Data related to a hit during a [shapecast](spatial_query#shapecasting).
#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
#[reflect(Debug, PartialEq)]
pub struct ShapeHitData {
    /// The entity of the collider that was hit by the shape.
    pub entity: Entity,

    /// How far the shape travelled before the initial hit.
    #[doc(alias = "time_of_impact")]
    pub distance: Scalar,

    /// The closest point on the shape that was hit, expressed in world space.
    ///
    /// If the shapes are penetrating or the target distance is greater than zero,
    /// this will be different from `point2`.
    pub point1: Vector,

    /// The closest point on the shape that was cast, expressed in world space.
    ///
    /// If the shapes are penetrating or the target distance is greater than zero,
    /// this will be different from `point1`.
    pub point2: Vector,

    /// The outward surface normal on the hit shape at `point1`, expressed in world space.
    pub normal1: Vector,

    /// The outward surface normal on the cast shape at `point2`, expressed in world space.
    pub normal2: Vector,
}
