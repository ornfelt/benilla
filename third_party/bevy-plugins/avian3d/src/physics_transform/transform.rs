//! Components for physics positions and rotations.

#![allow(clippy::unnecessary_cast)]

use crate::{physics_transform::PhysicsTransformConfig, prelude::*};
use bevy::{
    ecs::{lifecycle::HookContext, world::DeferredWorld},
    math::DQuat,
    prelude::*,
};
use derive_more::From;

/// The global position of a [rigid body](RigidBody) or a [collider](Collider).
///
/// # Relation to `Transform` and `GlobalTransform`
///
/// [`Position`] is used for physics internally and kept in sync with [`Transform`]
/// by the [`PhysicsTransformPlugin`]. It rarely needs to be used directly in your own code, as [`Transform`] can still
/// be used for almost everything. Using [`Position`] should only be required for managing positions
/// in systems running in the [`SubstepSchedule`]. However, if you prefer, you can also use [`Position`]
/// for everything.
///
/// The reasons why the engine uses a separate [`Position`] component can be found
/// [here](crate#why-are-there-separate-position-and-rotation-components).
///
/// # Example
///
/// ```
/// use avian3d::prelude::*;
/// use bevy::prelude::*;
///
/// fn setup(mut commands: Commands) {
///     commands.spawn((
///         RigidBody::Dynamic,
///          Position::from_xyz(0.0, 2.0, 0.0),
///     ));
/// }
/// ```
#[derive(Reflect, Clone, Copy, Component, Debug, Default, Deref, DerefMut, PartialEq, From)]
#[reflect(Debug, Component, Default, PartialEq)]
pub struct Position(pub Vector);

impl Position {
    /// A placeholder position. This is an invalid position and should *not*
    /// be used to an actually position entities in the world, but can be used
    /// to indicate that a position has not yet been initialized.
    pub const PLACEHOLDER: Self = Self(Vector::MAX);

    /// Creates a [`Position`] component with the given global `position`.
    pub fn new(position: Vector) -> Self {
        Self(position)
    }

    /// Creates a [`Position`] component with the global position `(x, y, z)`.
    pub fn from_xyz(x: Scalar, y: Scalar, z: Scalar) -> Self {
        Self(Vector::new(x, y, z))
    }
}

impl From<GlobalTransform> for Position {
    fn from(value: GlobalTransform) -> Self {
        Self::from_xyz(
            value.translation().adjust_precision().x,
            value.translation().adjust_precision().y,
            value.translation().adjust_precision().z,
        )
    }
}

impl From<&GlobalTransform> for Position {
    fn from(value: &GlobalTransform) -> Self {
        Self::from_xyz(
            value.translation().adjust_precision().x,
            value.translation().adjust_precision().y,
            value.translation().adjust_precision().z,
        )
    }
}

impl Ease for Position {
    fn interpolating_curve_unbounded(start: Self, end: Self) -> impl Curve<Self> {
        FunctionCurve::new(Interval::UNIT, move |t| {
            Position(Vector::lerp(start.0, end.0, t as Scalar))
        })
    }
}

/// The translation accumulated before the XPBD position solve.
#[derive(Reflect, Clone, Copy, Component, Debug, Default, Deref, DerefMut, PartialEq, From)]
#[reflect(Debug, Component, Default, PartialEq)]
pub struct PreSolveDeltaPosition(pub Vector);

/// The rotation accumulated before the XPBD position solve.
#[derive(Reflect, Clone, Copy, Component, Debug, Default, Deref, DerefMut, PartialEq, From)]
#[reflect(Debug, Component, Default, PartialEq)]
pub struct PreSolveDeltaRotation(pub Rotation);

/// Quaternion
#[allow(dead_code)]
pub(crate) type RotationValue = Quaternion;

impl Ease for Rotation {
    fn interpolating_curve_unbounded(start: Self, end: Self) -> impl Curve<Self> {
        FunctionCurve::new(Interval::UNIT, move |t| {
            Rotation::slerp(start, end, t as Scalar)
        })
    }
}

/// The global physics rotation of a [rigid body](RigidBody) or a [collider](Collider).
///
/// # Relation to `Transform` and `GlobalTransform`
///
/// [`Rotation`] is used for physics internally and kept in sync with [`Transform`]
/// by the [`PhysicsTransformPlugin`]. It rarely needs to be used directly in your own code, as [`Transform`] can still
/// be used for almost everything. Using [`Rotation`] should only be required for managing rotations
/// in systems running in the [`SubstepSchedule`], but if you prefer, you can also use [`Rotation`]
/// for everything.
///
/// The reasons why the engine uses a separate [`Rotation`] component can be found
/// [here](crate#why-are-there-separate-position-and-rotation-components).
///
/// # Example
///
/// ```
/// use avian3d::prelude::*;
/// use bevy::prelude::*;
///
/// # #[cfg(feature = "f32")]
/// fn setup(mut commands: Commands) {
///     // Spawn a dynamic rigid body rotated by 1.5 radians around the x axis
///     commands.spawn((RigidBody::Dynamic, Rotation(Quat::from_rotation_x(1.5))));
/// }
/// ```
#[derive(Reflect, Clone, Copy, Component, Debug, Default, Deref, DerefMut, PartialEq)]
#[reflect(Debug, Component, Default, PartialEq)]
pub struct Rotation(pub Quaternion);

impl Rotation {
    /// A placeholder rotation. This is an invalid rotation and should *not*
    /// be used to an actually rotate entities in the world, but can be used
    /// to indicate that a rotation has not yet been initialized.
    pub const PLACEHOLDER: Self = Self(Quaternion::from_xyzw(
        Scalar::MAX,
        Scalar::MAX,
        Scalar::MAX,
        Scalar::MAX,
    ));

    /// No rotation.
    pub const IDENTITY: Self = Self(Quaternion::IDENTITY);

    /// Returns the angle (in radians) for the minimal rotation for transforming this rotation into another.
    #[inline]
    pub fn angle_between(self, other: Self) -> Scalar {
        self.0.angle_between(other.0)
    }

    /// Inverts the rotation.
    #[inline]
    #[must_use]
    pub fn inverse(&self) -> Self {
        Self(self.0.inverse())
    }

    /// Performs a linear interpolation between `self` and `end` based on
    /// the value `s`, and normalizes the rotation afterwards.
    ///
    /// When `s == 0.0`, the result will be equal to `self`.
    /// When `s == 1.0`, the result will be equal to `end`.
    ///
    /// This is slightly more efficient than [`slerp`](Self::slerp), and produces a similar result
    /// when the difference between the two rotations is small. At larger differences,
    /// the result resembles a kind of ease-in-out effect.
    ///
    /// If you would like the angular velocity to remain constant, consider using [`slerp`](Self::slerp) instead.
    #[inline]
    pub fn nlerp(self, end: Self, t: Scalar) -> Self {
        Self(self.0.lerp(end.0, t))
    }

    /// Performs a spherical linear interpolation between `self` and `end`
    /// based on the value `s`.
    ///
    /// This corresponds to interpolating between the two angles at a constant angular velocity.
    ///
    /// When `s == 0.0`, the result will be equal to `self`.
    /// When `s == 1.0`, the result will be equal to `end`.
    ///
    /// If you would like the rotation to have a kind of ease-in-out effect, consider
    /// using the slightly more efficient [`nlerp`](Self::nlerp) instead.
    #[inline]
    pub fn slerp(self, end: Self, t: Scalar) -> Self {
        Self(self.0.slerp(end.0, t))
    }

    /// Returns `self` after an approximate normalization,
    /// assuming the value is already nearly normalized.
    /// Useful for preventing numerical error accumulation.
    #[inline]
    #[must_use]
    pub fn fast_renormalize(self) -> Self {
        // First-order Tayor approximation
        // 1/L = (L^2)^(-1/2) ≈ 1 - (L^2 - 1) / 2 = (3 - L^2) / 2
        let length_squared = self.length_squared();
        let approx_inv_length = 0.5 * (3.0 - length_squared);
        Self(self.0 * approx_inv_length)
    }
}

impl core::ops::Mul<Vector> for Rotation {
    type Output = Vector;

    fn mul(self, vector: Vector) -> Self::Output {
        self.0 * vector
    }
}

impl core::ops::Mul for Rotation {
    type Output = Rotation;

    fn mul(self, rhs: Self) -> Self::Output {
        Self(self.0 * rhs.0)
    }
}

impl core::ops::MulAssign for Rotation {
    fn mul_assign(&mut self, rhs: Self) {
        self.0 *= rhs.0;
    }
}

impl core::ops::Mul<Quaternion> for Rotation {
    type Output = Quaternion;

    fn mul(self, quaternion: Quaternion) -> Self::Output {
        self.0 * quaternion
    }
}

impl core::ops::Mul<Quaternion> for &Rotation {
    type Output = Quaternion;

    fn mul(self, quaternion: Quaternion) -> Self::Output {
        self.0 * quaternion
    }
}

impl core::ops::Mul<Quaternion> for &mut Rotation {
    type Output = Quaternion;

    fn mul(self, quaternion: Quaternion) -> Self::Output {
        self.0 * quaternion
    }
}

impl core::ops::Mul<Rotation> for Quaternion {
    type Output = Rotation;

    fn mul(self, rotation: Rotation) -> Self::Output {
        Rotation(self * rotation.0)
    }
}

impl core::ops::Mul<Rotation> for &Quaternion {
    type Output = Rotation;

    fn mul(self, rotation: Rotation) -> Self::Output {
        Rotation(*self * rotation.0)
    }
}

impl core::ops::Mul<Rotation> for &mut Quaternion {
    type Output = Rotation;

    fn mul(self, rotation: Rotation) -> Self::Output {
        Rotation(*self * rotation.0)
    }
}

impl core::ops::Mul<Dir> for Rotation {
    type Output = Dir;

    fn mul(self, direction: Dir) -> Self::Output {
        Dir::new_unchecked((self * direction.adjust_precision()).f32())
    }
}

impl core::ops::Mul<Vector> for &Rotation {
    type Output = Vector;

    fn mul(self, vector: Vector) -> Self::Output {
        *self * vector
    }
}

impl core::ops::Mul<Dir> for &Rotation {
    type Output = Dir;

    fn mul(self, direction: Dir) -> Self::Output {
        Dir::new_unchecked((*self * direction.adjust_precision()).f32())
    }
}

impl core::ops::Mul<Vector> for &mut Rotation {
    type Output = Vector;

    fn mul(self, vector: Vector) -> Self::Output {
        *self * vector
    }
}

impl core::ops::Mul<Dir> for &mut Rotation {
    type Output = Dir;

    fn mul(self, direction: Dir) -> Self::Output {
        Dir::new_unchecked((*self * direction.adjust_precision()).f32())
    }
}

impl core::ops::Mul<&Vector> for Rotation {
    type Output = Vector;

    fn mul(self, vector: &Vector) -> Self::Output {
        self * *vector
    }
}

impl core::ops::Mul<&Dir> for Rotation {
    type Output = Dir;

    fn mul(self, direction: &Dir) -> Self::Output {
        Dir::new_unchecked((self * direction.adjust_precision()).f32())
    }
}

impl core::ops::Mul<&mut Vector> for Rotation {
    type Output = Vector;

    fn mul(self, vector: &mut Vector) -> Self::Output {
        self * *vector
    }
}

impl core::ops::Mul<&mut Dir> for Rotation {
    type Output = Dir;

    fn mul(self, direction: &mut Dir) -> Self::Output {
        Dir::new_unchecked((self * direction.adjust_precision()).f32())
    }
}

impl core::ops::Mul<&Vector> for &Rotation {
    type Output = Vector;

    fn mul(self, vector: &Vector) -> Self::Output {
        *self * *vector
    }
}

impl core::ops::Mul<&Dir> for &Rotation {
    type Output = Dir;

    fn mul(self, direction: &Dir) -> Self::Output {
        Dir::new_unchecked((*self * direction.adjust_precision()).f32())
    }
}

impl core::ops::Mul<&Vector> for &mut Rotation {
    type Output = Vector;

    fn mul(self, vector: &Vector) -> Self::Output {
        *self * *vector
    }
}

impl core::ops::Mul<&Dir> for &mut Rotation {
    type Output = Dir;

    fn mul(self, direction: &Dir) -> Self::Output {
        Dir::new_unchecked((*self * direction.adjust_precision()).f32())
    }
}

impl core::ops::Mul<&mut Vector> for &Rotation {
    type Output = Vector;

    fn mul(self, vector: &mut Vector) -> Self::Output {
        *self * *vector
    }
}

impl core::ops::Mul<&mut Dir> for &Rotation {
    type Output = Dir;

    fn mul(self, direction: &mut Dir) -> Self::Output {
        Dir::new_unchecked((*self * direction.adjust_precision()).f32())
    }
}

impl core::ops::Mul<&mut Vector> for &mut Rotation {
    type Output = Vector;

    fn mul(self, vector: &mut Vector) -> Self::Output {
        *self * *vector
    }
}

impl core::ops::Mul<&mut Dir> for &mut Rotation {
    type Output = Dir;

    fn mul(self, direction: &mut Dir) -> Self::Output {
        Dir::new_unchecked((*self * direction.adjust_precision()).f32())
    }
}

impl From<Rotation> for Quaternion {
    fn from(rot: Rotation) -> Self {
        rot.0
    }
}

impl From<Transform> for Rotation {
    fn from(value: Transform) -> Self {
        Self::from(value.rotation)
    }
}

impl From<GlobalTransform> for Rotation {
    fn from(value: GlobalTransform) -> Self {
        Self::from(value.compute_transform().rotation)
    }
}

impl From<&GlobalTransform> for Rotation {
    fn from(value: &GlobalTransform) -> Self {
        Self::from(value.compute_transform().rotation)
    }
}

impl From<Quat> for Rotation {
    fn from(quat: Quat) -> Self {
        Self(Quaternion::from_xyzw(
            quat.x as Scalar,
            quat.y as Scalar,
            quat.z as Scalar,
            quat.w as Scalar,
        ))
    }
}

impl From<DQuat> for Rotation {
    fn from(quat: DQuat) -> Self {
        Self(Quaternion::from_xyzw(
            quat.x as Scalar,
            quat.y as Scalar,
            quat.z as Scalar,
            quat.w as Scalar,
        ))
    }
}

pub(crate) fn init_physics_transform(world: &mut DeferredWorld, ctx: &HookContext) {
    let entity_ref = world.entity(ctx.entity);

    // Get the global `Position` and `Rotation`.
    let (mut position, is_pos_placeholder) = entity_ref
        .get::<Position>()
        .map_or((default(), true), |p| (*p, *p == Position::PLACEHOLDER));
    let (mut rotation, is_rot_placeholder) = entity_ref
        .get::<Rotation>()
        .map_or((default(), true), |r| (*r, *r == Rotation::PLACEHOLDER));

    if is_pos_placeholder {
        position.0 = Vector::ZERO;
    }
    if is_rot_placeholder {
        rotation = Rotation::IDENTITY;
    }

    // If either `Position` or `Rotation` was set manually, we want to set `Transform` to match later.
    let is_not_placeholder = !is_pos_placeholder || !is_rot_placeholder;

    let config = world
        .get_resource::<PhysicsTransformConfig>()
        .cloned()
        .unwrap_or_default();

    let mut parent_global_transform = GlobalTransform::default();

    // Compute the global transform by traversing up the hierarchy.
    let mut curr_parent = world.get::<ChildOf>(ctx.entity);
    while let Some(parent) = curr_parent {
        if let Some(parent_transform) = world.get::<Transform>(parent.0) {
            parent_global_transform = *parent_transform * parent_global_transform;
        }
        curr_parent = world.get::<ChildOf>(parent.0);
    }

    let transform = world.get::<Transform>(ctx.entity).copied();
    let global_transform = transform.map(|transform| {
        let global_transform = parent_global_transform * GlobalTransform::from(transform);
        // Update the global transform.
        *world.get_mut::<GlobalTransform>(ctx.entity).unwrap() = global_transform;
        global_transform
    });

    // If either `Position` or `Rotation` was not a placeholder,
    // we need to update the `Transform` to match the current values.
    if is_not_placeholder && config.position_to_transform {
        // Get the parent's global transform if it exists.
        if parent_global_transform != GlobalTransform::default() {
            // The new local transform of the child body, computed from the its global transform
            // and its parents global transform.
            let new_transform = GlobalTransform::from(
                Transform::from_translation(position.f32()).with_rotation(rotation.f32()),
            )
            .reparented_to(&parent_global_transform);

            // Update the `Transform` of the entity with the new local transform.
            if let Some(mut transform) = world.get_mut::<Transform>(ctx.entity) {
                transform.translation = new_transform.translation;
                transform.rotation = new_transform.rotation;
            }
        } else if let Some(mut transform) = world.get_mut::<Transform>(ctx.entity) {
            // If the entity has no parent, we can set the transform directly.
            {
                if !is_pos_placeholder {
                    transform.translation = position.f32();
                }
                if !is_rot_placeholder {
                    transform.rotation = rotation.f32();
                }
            }
        }
    }

    if !config.transform_to_position {
        if is_pos_placeholder && let Some(mut position) = world.get_mut::<Position>(ctx.entity) {
            position.0 = Vector::ZERO;
        }
        if is_rot_placeholder && let Some(mut rotation) = world.get_mut::<Rotation>(ctx.entity) {
            *rotation = Rotation::IDENTITY;
        }
    } else if is_pos_placeholder || is_rot_placeholder {
        // If either `Position` or `Rotation` is a placeholder, we need to compute the global transform
        // from the hierarchy and set the `Position` and/or `Rotation` to the computed values.

        if let Some(global_transform) = global_transform {
            // Set the computed `position` and `rotation` based on the global transform.
            let (_, global_rotation, global_translation) =
                global_transform.to_scale_rotation_translation();
            {
                position.0 = global_translation.adjust_precision();
                rotation.0 = global_rotation.adjust_precision();
            }
        } else {
            // No transform was set. Set the computed `position` and `rotation` to default values.
            if is_pos_placeholder {
                position.0 = Vector::ZERO;
            }
            if is_rot_placeholder {
                rotation = Rotation::IDENTITY;
            }
        }

        // Now we update the actual component values based on the computed global transform.
        let mut entity_mut = world.entity_mut(ctx.entity);

        // Set the position unless it was already set.
        if let Some(mut pos) = entity_mut
            .get_mut::<Position>()
            .filter(|pos| **pos == Position::PLACEHOLDER)
        {
            *pos = position;
        }
        // Set the rotation to the global transform unless it was already set.
        if let Some(mut rot) = entity_mut
            .get_mut::<Rotation>()
            .filter(|rot| **rot == Rotation::PLACEHOLDER)
        {
            *rot = rotation;
        }
    }
}
