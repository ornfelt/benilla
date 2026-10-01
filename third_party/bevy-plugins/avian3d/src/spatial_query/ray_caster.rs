use crate::prelude::*;
use bevy::prelude::*;

/// Data related to a hit during a [raycast](spatial_query#raycasting).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RayHitData {
    /// The entity of the collider that was hit by the ray.
    pub entity: Entity,

    /// How far the ray travelled. This is the distance between the ray origin and the point of intersection.
    pub distance: Scalar,

    /// The normal at the point of intersection, expressed in world space.
    pub normal: Vector,
}
