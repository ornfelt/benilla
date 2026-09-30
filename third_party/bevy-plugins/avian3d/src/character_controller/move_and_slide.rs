//! Contains the *move and slide* algorithm and utilities for kinematic character controllers.
//!
//! See the documentation of [`MoveAndSlide`] for more information.

pub use super::velocity_project::*;

use crate::prelude::*;
use bevy::{ecs::system::SystemParam, prelude::*};

/// Needed to improve stability when `n.dot(dir)` happens to be very close to zero.
const DOT_EPSILON: Scalar = 0.005;

/// Cosine of 5 degrees.
#[allow(clippy::excessive_precision)]
pub const COS_5_DEGREES: Scalar = 0.99619469809;

/// A [`SystemParam`] for the *move and slide* algorithm, also known as *collide and slide* or *step slide*.
///
/// Move and slide is the core movement and collision algorithm used by most kinematic character controllers.
/// It attempts to move a shape along a desired velocity vector, sliding along any colliders that are hit on the way.
///
/// # Algorithm
///
/// At a high level, the algorithm works as follows:
///
/// 1. Sweep the shape along the desired velocity vector.
/// 2. If no collision is detected, move the full distance.
/// 3. If a collision is detected:
///    - Move up to the point of collision.
///    - Project the remaining velocity onto the contact surfaces to obtain a new sliding velocity.
/// 4. Repeat with the new sliding velocity until movement is complete.
///
/// The algorithm also includes depenetration passes before and after movement to improve stability
/// and ensure that the shape does not intersect any colliders.
///
/// # Configuration
///
/// [`MoveAndSlideConfig`] allows configuring various aspects of the algorithm.
/// See its documentation for more information.
///
/// # Utilities
///
/// This system parameter provides utilities for:
///
/// - Performing shape casts optimized for movement via [`cast_move`](MoveAndSlide::cast_move).
/// - Projecting velocities to slide along contact planes via [`project_velocity`](MoveAndSlide::project_velocity).
///
/// These methods are used internally by the move and slide algorithm, but can also be used independently
/// for custom movement and collision handling.
///
/// # Resources
///
/// Some useful resources for learning more about the move and slide algorithm include:
///
/// - [*Collide And Slide - \*Actually Decent\* Character Collision From Scratch*](https://youtu.be/YR6Q7dUz2uk) by [Poke Dev](https://www.youtube.com/@poke_gamedev) (video)
/// - [`PM_SlideMove`](https://github.com/id-Software/Quake-III-Arena/blob/dbe4ddb10315479fc00086f08e25d968b4b43c49/code/game/bg_slidemove.c#L45) in Quake III Arena (source code)
///
/// Note that while the high-level concepts are similar across different implementations, details may vary.
#[derive(SystemParam)]
#[doc(alias = "CollideAndSlide")]
#[doc(alias = "StepSlide")]
pub struct MoveAndSlide<'w, 's> {
    /// The [`SpatialQuery`] system parameter used to perform shape casts and other geometric queries.
    pub spatial_query: SpatialQuery<'w, 's>,
    /// The [`Query`] used to query for colliders.
    pub colliders: Query<
        'w,
        's,
        (
            &'static Collider,
            &'static Position,
            &'static Rotation,
            Option<&'static CollisionLayers>,
        ),
        (With<ColliderOf>, Without<Sensor>),
    >,
    /// A units-per-meter scaling factor that adjusts some thresholds and tolerances
    /// to the scale of the world for better behavior.
    pub length_unit: Res<'w, PhysicsLengthUnit>,
}

/// Configuration for the move and slide algorithm.
#[derive(Clone, Debug, PartialEq, Reflect)]
#[reflect(Debug, PartialEq)]
pub struct MoveAndSlideConfig {
    /// How many iterations to use when moving the character.
    ///
    /// A single iteration consists of:
    ///
    /// - Moving the character as far as possible in the desired velocity direction.
    /// - Modifying the velocity to slide along any contact surfaces.
    ///
    /// Increasing this allows the character to slide along more surfaces in a single frame,
    /// which can help with complex geometry and high speeds, but increases computation time.
    pub move_and_slide_iterations: usize,

    /// How many iterations to use when performing depenetration.
    ///
    /// Depenetration is an iterative process that solves penetrations for all planes,
    /// until we either reached [`MoveAndSlideConfig::move_and_slide_iterations`]
    /// or the accumulated error is less than [`MoveAndSlideConfig::max_depenetration_error`].
    ///
    /// To disable depenetration, set this to `0`.
    pub depenetration_iterations: usize,

    /// The target error to achieve when performing depenetration.
    ///
    /// Depenetration is an iterative process that solves penetrations for all planes,
    /// until we either reached [`MoveAndSlideConfig::move_and_slide_iterations`]
    /// or the accumulated error is less than [`MoveAndSlideConfig::max_depenetration_error`].
    ///
    /// This is implicitly scaled by the [`PhysicsLengthUnit`].
    pub max_depenetration_error: Scalar,

    /// The maximum penetration depth that is allowed for a contact to be resolved during depenetration.
    ///
    /// This is used to reject invalid contacts that have an excessively high penetration depth,
    /// which can lead to clipping through geometry. This may be removed in the future once the
    /// collision errors in the underlying collision detection system are fixed.
    ///
    /// This is implicitly scaled by the [`PhysicsLengthUnit`].
    pub penetration_rejection_threshold: Scalar,

    /// A minimal distance to always keep between the collider and any other colliders.
    ///
    /// This exists to improve numerical stability and ensure that the collider never intersects anything.
    /// Set this to a small enough value that you don't see visual artifacts but have good stability.
    ///
    /// Increase the value if you notice your character getting stuck in geometry.
    /// Decrease it when you notice jittering, especially around V-shaped walls.
    ///
    /// This is implicitly scaled by the [`PhysicsLengthUnit`].
    pub skin_width: Scalar,

    /// The initial planes to consider for the move and slide algorithm.
    ///
    /// This will be expanded during the algorithm with contact planes, but you can also initialize it
    /// with some predefined planes that the algorithm should never move against.
    ///
    /// A common use case is adding the ground plane when a character controller is standing or walking on the ground.
    pub planes: Vec<Dir>,

    /// The dot product threshold to consider two planes as similar when pruning nearly parallel planes.
    /// The comparison used is `n1.dot(n2) >= plane_similarity_dot_threshold`.
    ///
    /// This is used to reduce the number of planes considered during move and slide,
    /// which can improve performance for dense geomtry. However, setting this value too high
    /// can lead to unwanted behavior, as it may discard important planes.
    ///
    /// The default value of [`COS_5_DEGREES`] (≈0.996) corresponds to a 5 degree angle between the planes.
    pub plane_similarity_dot_threshold: Scalar,

    /// The maximum number of planes to solve while performing move and slide.
    ///
    /// If the number of planes exceeds this value, the algorithm will stop collecting new planes.
    /// This is a safety measure to prevent excessive computation time for dense geometry.
    pub max_planes: usize,
}

impl Default for MoveAndSlideConfig {
    fn default() -> Self {
        let default_depen_cfg = DepenetrationConfig::default();
        Self {
            move_and_slide_iterations: 4,
            depenetration_iterations: default_depen_cfg.depenetration_iterations,
            max_depenetration_error: default_depen_cfg.max_depenetration_error,
            penetration_rejection_threshold: default_depen_cfg.penetration_rejection_threshold,
            skin_width: default_depen_cfg.skin_width,
            planes: Vec::new(),
            plane_similarity_dot_threshold: COS_5_DEGREES,
            max_planes: 20,
        }
    }
}

/// Configuration for depenetration.
#[derive(Clone, Debug, PartialEq, Reflect)]
#[reflect(Debug, PartialEq)]
pub struct DepenetrationConfig {
    /// How many iterations to use when performing depenetration.
    ///
    /// Depenetration is an iterative process that solves penetrations for all planes,
    /// until we either reached [`MoveAndSlideConfig::move_and_slide_iterations`]
    /// or the accumulated error is less than [`MoveAndSlideConfig::max_depenetration_error`].
    ///
    /// To disable depenetration, set this to `0`.
    pub depenetration_iterations: usize,

    /// The target error to achieve when performing depenetration.
    ///
    /// Depenetration is an iterative process that solves penetrations for all planes,
    /// until we either reached [`MoveAndSlideConfig::move_and_slide_iterations`]
    /// or the accumulated error is less than [`MoveAndSlideConfig::max_depenetration_error`].
    ///
    /// This is implicitly scaled by the [`PhysicsLengthUnit`].
    pub max_depenetration_error: Scalar,

    /// The maximum penetration depth that is allowed for a contact to be resolved during depenetration.
    ///
    /// This is used to reject invalid contacts that have an excessively high penetration depth,
    /// which can lead to clipping through geometry. This may be removed in the future once the
    /// collision errors in the underlying collision detection system are fixed.
    ///
    /// This is implicitly scaled by the [`PhysicsLengthUnit`].
    pub penetration_rejection_threshold: Scalar,

    /// A minimal distance to always keep between the collider and any other colliders.
    ///
    /// This exists to improve numerical stability and ensure that the collider never intersects anything.
    /// Set this to a small enough value that you don't see visual artifacts but have good stability.
    ///
    /// Increase the value if you notice your character getting stuck in geometry.
    /// Decrease it when you notice jittering, especially around V-shaped walls.
    ///
    /// This is implicitly scaled by the [`PhysicsLengthUnit`].
    pub skin_width: Scalar,
}

impl Default for DepenetrationConfig {
    fn default() -> Self {
        Self {
            depenetration_iterations: 16,
            max_depenetration_error: 0.0001,
            penetration_rejection_threshold: 0.5,
            skin_width: 0.01,
        }
    }
}

impl From<&MoveAndSlideConfig> for DepenetrationConfig {
    fn from(config: &MoveAndSlideConfig) -> Self {
        Self {
            depenetration_iterations: config.depenetration_iterations,
            max_depenetration_error: config.max_depenetration_error,
            penetration_rejection_threshold: config.penetration_rejection_threshold,
            skin_width: config.skin_width,
        }
    }
}

/// Output from the move and slide algorithm.
#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
#[reflect(Debug, PartialEq)]
pub struct MoveAndSlideOutput {
    /// The final position of the character after move and slide.
    ///
    /// Set your [`Transform::translation`] to this value.
    pub position: Vector,

    /// The final velocity of the character after move and slide.
    ///
    /// This corresponds to the remaining velocity after the algorithm has slid along all contact surfaces.
    /// For example, if the character is trying to move to the right, but there is a ramp in its path,
    /// the projected velocity will point up the ramp, with reduced magnitude.
    ///
    /// It is useful to store this value or apply it to [`LinearVelocity`] and use it as the input velocity
    /// for the next frame's call to the move and slide algorithm.
    ///
    /// Note that if you apply this to [`LinearVelocity`], it is recommended to use [`CustomPositionIntegration`].
    /// This ways, the character's position is only updated via the move and slide algorithm,
    /// and not also by the physics integrator.
    pub projected_velocity: Vector,
}

/// Data related to a hit during the move and slide algorithm.
#[derive(Debug, PartialEq)]
pub struct MoveAndSlideHitData<'a> {
    /// The entity of the collider that was hit by the shape.
    pub entity: Entity,

    /// The maximum distance that is safe to move in the given direction so that the collider
    /// still keeps a distance of `skin_width` to the other colliders.
    ///
    /// This is `0.0` when any of the following is true:
    ///
    /// - The collider started off intersecting another collider.
    /// - The collider is moving toward another collider that is already closer than `skin_width`.
    ///
    /// If you want to know the real distance to the next collision, use [`Self::collision_distance`].
    pub distance: Scalar,

    /// The hit point on the shape that was hit, expressed in world space.
    pub point: Vector,

    /// The outward surface normal on the hit shape at `point`, expressed in world space.
    pub normal: &'a mut Dir,

    /// The position of the collider at the time of the move and slide iteration.
    pub position: &'a mut Vector,

    /// The velocity of the collider at the time of the move and slide iteration.
    pub velocity: &'a mut Vector,

    /// The raw distance to the next collision, not respecting skin width.
    /// To move the shape, use [`Self::distance`] instead.
    #[doc(alias = "time_of_impact")]
    pub collision_distance: Scalar,
}

/// Indicates how to handle a hit detected during the move and slide algorithm.
///
/// This is returned by the `on_hit` callback provided to the move and slide algorithm.
#[derive(Debug, PartialEq)]
pub enum MoveAndSlideHitResponse {
    /// Accept the hit and continue the move and slide algorithm.
    Accept,

    /// Ignore the hit and continue the move and slide algorithm.
    ///
    /// Note that the shape will still be moved up to the point of collision,
    /// but the velocity will not be modified to slide along the surface.
    Ignore,

    /// Ignore the hit and abort the move and slide algorithm.
    ///
    /// Note that the shape will still be moved up to the point of collision,
    /// but no further movement or velocity modification will be performed.
    Abort,
}

/// Data related to a hit during [`MoveAndSlide::cast_move`].
#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
#[reflect(Debug, PartialEq)]
pub struct MoveHitData {
    /// The entity of the collider that was hit by the shape.
    pub entity: Entity,

    /// The maximum distance that is safe to move in the given direction so that the collider
    /// still keeps a distance of `skin_width` to the other colliders.
    ///
    /// This is `0.0` when any of the following is true:
    ///
    /// - The collider started off intersecting another collider.
    /// - The collider is moving toward another collider that is already closer than `skin_width`.
    ///
    /// If you want to know the real distance to the next collision, use [`Self::collision_distance`].
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

    /// The raw distance to the next collision, not respecting skin width.
    /// To move the shape, use [`Self::distance`] instead.
    #[doc(alias = "time_of_impact")]
    pub collision_distance: Scalar,
}

impl<'w, 's> MoveAndSlide<'w, 's> {
    /// A [shape cast](spatial_query#shapecasting) optimized for movement. Use this if you want to move a collider
    /// with a given velocity and stop so that it keeps a distance of `skin_width` from the first collider on its path.
    ///
    /// This operation is most useful when you ensure that the character is not intersecting any colliders before moving.
    ///
    /// It is often useful to clip the velocity afterwards so that it no longer points into the contact plane using [`Self::project_velocity`].
    ///
    /// # Arguments
    ///
    /// - `shape`: The shape being cast represented as a [`Collider`].
    /// - `shape_position`: Where the shape is cast from.
    /// - `shape_rotation`: The rotation of the shape being cast.
    /// - `movement`: The direction and magnitude of the movement. If this is [`Vector::ZERO`], this method can still return `Some(MoveHitData)` if the shape started off intersecting a collider.
    /// - `skin_width`: A [`ShapeCastConfig`] that determines the behavior of the cast.
    /// - `filter`: A [`SpatialQueryFilter`] that determines which colliders are taken into account in the query. It is highly recommended to exclude the entity holding the collider itself,
    ///   otherwise the character will collide with itself.
    ///
    /// # Returns
    ///
    /// - `Some(MoveHitData)` if the shape hit a collider on the way, or started off intersecting a collider.
    /// - `None` if the shape is able to move the full distance without hitting a collider.
    ///
    /// # Example
    ///
    /// ```
    /// use bevy::prelude::*;
    /// use avian3d::{prelude::*, math::{Vector, Dir, AdjustPrecision as _, AsF32 as _}};
    ///
    /// #[derive(Component)]
    /// struct CharacterController {
    ///     velocity: Vector,
    /// }
    ///
    /// fn perform_cast_move(
    ///     player: Single<(Entity, &Collider, &mut CharacterController, &mut Transform)>,
    ///     move_and_slide: MoveAndSlide,
    ///     time: Res<Time>
    /// ) {
    ///     let (entity, collider, mut controller, mut transform) = player.into_inner();
    ///     let filter = SpatialQueryFilter::from_excluded_entities([entity]);
    ///     let config = MoveAndSlideConfig::default();
    ///     let velocity = controller.velocity;
    ///
    ///     let hit = move_and_slide.cast_move(
    ///         collider,
    ///          transform.translation.adjust_precision(),
    ///          transform.rotation.adjust_precision(),
    ///         velocity * time.delta_secs().adjust_precision(),
    ///         config.skin_width,
    ///         &filter,
    ///     );
    ///     if let Some(hit) = hit {
    ///         // We collided with something on the way. Advance as much as possible.
    ///          transform.translation += (velocity.normalize_or_zero() * hit.distance).f32();
    ///         // Then project the velocity to make sure it no longer points towards the contact plane.
    ///         controller.velocity =
    ///             MoveAndSlide::project_velocity(velocity, &[Dir::new_unchecked(hit.normal1.f32())])
    ///     } else {
    ///         // We traveled the full distance without colliding.
    ///          transform.translation += velocity.f32();
    ///     }
    /// }
    /// ```
    ///
    /// # Related methods
    ///
    /// - [`SpatialQuery::cast_shape`]
    #[must_use]
    #[doc(alias = "sweep")]
    pub fn cast_move(
        &self,
        shape: &Collider,
        shape_position: Vector,
        shape_rotation: RotationValue,
        movement: Vector,
        skin_width: Scalar,
        filter: &SpatialQueryFilter,
    ) -> Option<MoveHitData> {
        let (direction, distance) = Dir::new_and_length(movement.f32()).unwrap_or((Dir::X, 0.0));
        let distance = distance.adjust_precision() + skin_width;
        let shape_hit = self.spatial_query.cast_shape_predicate(
            shape,
            shape_position,
            shape_rotation,
            direction,
            &ShapeCastConfig {
                ignore_origin_penetration: true,
                ..ShapeCastConfig::from_max_distance(distance)
            },
            filter,
            // Make sure we don't hit sensors.
            // TODO: Replace this when spatial queries support excluding sensors directly.
            &|entity| self.colliders.contains(entity),
        )?;
        let safe_distance = if distance == 0.0 {
            0.0
        } else {
            Self::pull_back(shape_hit, direction, skin_width)
        };
        Some(MoveHitData {
            distance: safe_distance,
            collision_distance: distance,
            entity: shape_hit.entity,
            point1: shape_hit.point1,
            point2: shape_hit.point2,
            normal1: shape_hit.normal1,
            normal2: shape_hit.normal2,
        })
    }

    /// Returns a [`ShapeHitData::distance`] that is reduced such that the hit distance is at least `skin_width`.
    /// The result will never be negative, so if the hit is already closer than `skin_width`, the returned distance will be zero.
    #[must_use]
    fn pull_back(hit: ShapeHitData, dir: Dir, skin_width: Scalar) -> Scalar {
        let dot = dir.adjust_precision().dot(-hit.normal1).max(DOT_EPSILON);
        let skin_distance = skin_width / dot;
        (hit.distance - skin_distance).max(0.0)
    }

    /// Projects input velocity `v` onto the planes defined by the given `normals`.
    /// This ensures that `velocity` does not point into any of the planes, but along them.
    ///
    /// This is often used after [`MoveAndSlide::cast_move`] to ensure a character moved that way
    /// does not try to continue moving into colliding geometry.
    #[must_use]
    pub fn project_velocity(v: Vector, normals: &[Dir]) -> Vector {
        project_velocity(v, normals)
    }
}
