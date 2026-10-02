//! Isometry types for expressing rigid motions in three dimensions.

use crate::{Affine3, Affine3A, Dir3, Mat3, Mat3A, Quat, Vec3, Vec3A};
use core::ops::Mul;

#[cfg(feature = "approx")]
use approx::{AbsDiffEq, RelativeEq, UlpsEq};

/// An isometry in three dimensions, representing a rotation followed by a translation.
/// This can often be useful for expressing relative positions and transformations from one position to another.
///
/// In particular, this type represents a distance-preserving transformation known as a *rigid motion* or a *direct motion*,
/// and belongs to the special [Euclidean group] SE(3). This includes translation and rotation, but excludes reflection.
///
/// [Euclidean group]: https://en.wikipedia.org/wiki/Euclidean_group
///
/// # Example
///
/// Isometries can be created from a given translation and rotation:
///
/// ```
/// # use bevy_math::{Isometry3d, Quat, Vec3};
/// # use std::f32::consts::FRAC_PI_2;
/// #
/// let iso = Isometry3d::new(Vec3::new(2.0, 1.0, 3.0), Quat::from_rotation_z(FRAC_PI_2));
/// ```
///
/// Or from separate parts:
///
/// ```
/// # use bevy_math::{Isometry3d, Quat, Vec3};
/// # use std::f32::consts::FRAC_PI_2;
/// #
/// let iso1 = Isometry3d::from_translation(Vec3::new(2.0, 1.0, 3.0));
/// let iso2 = Isometry3d::from_rotation(Quat::from_rotation_z(FRAC_PI_2));
/// ```
///
/// The isometries can be used to transform points:
///
/// ```
/// # use approx::assert_relative_eq;
/// # use bevy_math::{Isometry3d, Quat, Vec3};
/// # use std::f32::consts::FRAC_PI_2;
/// #
/// let iso = Isometry3d::new(Vec3::new(2.0, 1.0, 3.0), Quat::from_rotation_z(FRAC_PI_2));
/// let point = Vec3::new(4.0, 4.0, 4.0);
///
/// // These are equivalent
/// let result = iso.transform_point(point);
/// let result = iso * point;
///
/// assert_relative_eq!(result, Vec3::new(-2.0, 5.0, 7.0));
/// ```
///
/// Isometries can also be composed together:
///
/// ```
/// # use bevy_math::{Isometry3d, Quat, Vec3};
/// # use std::f32::consts::FRAC_PI_2;
/// #
/// # let iso = Isometry3d::new(Vec3::new(2.0, 1.0, 3.0), Quat::from_rotation_z(FRAC_PI_2));
/// # let iso1 = Isometry3d::from_translation(Vec3::new(2.0, 1.0, 3.0));
/// # let iso2 = Isometry3d::from_rotation(Quat::from_rotation_z(FRAC_PI_2));
/// #
/// assert_eq!(iso1 * iso2, iso);
/// ```
///
/// One common operation is to compute an isometry representing the relative positions of two objects
/// for things like intersection tests. This can be done with an inverse transformation:
///
/// ```
/// # use bevy_math::{Isometry3d, Quat, Vec3};
/// # use std::f32::consts::FRAC_PI_2;
/// #
/// let sphere_iso = Isometry3d::from_translation(Vec3::new(2.0, 1.0, 3.0));
/// let cuboid_iso = Isometry3d::from_rotation(Quat::from_rotation_z(FRAC_PI_2));
///
/// // Compute the relative position and orientation between the two shapes
/// let relative_iso = sphere_iso.inverse() * cuboid_iso;
///
/// // Or alternatively, to skip an extra rotation operation:
/// let relative_iso = sphere_iso.inverse_mul(cuboid_iso);
/// ```
#[derive(Copy, Clone, Default, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct Isometry3d {
    /// The rotational part of a three-dimensional isometry.
    pub rotation: Quat,
    /// The translational part of a three-dimensional isometry.
    pub translation: Vec3A,
}

impl Isometry3d {
    /// Create a three-dimensional isometry from a rotation and a translation.
    #[inline]
    pub fn new(translation: impl Into<Vec3A>, rotation: Quat) -> Self {
        Isometry3d {
            rotation,
            translation: translation.into(),
        }
    }

    /// Create a three-dimensional isometry from a rotation.
    #[inline]
    pub const fn from_rotation(rotation: Quat) -> Self {
        Isometry3d {
            rotation,
            translation: Vec3A::ZERO,
        }
    }

    /// Create a three-dimensional isometry from a translation.
    #[inline]
    pub fn from_translation(translation: impl Into<Vec3A>) -> Self {
        Isometry3d {
            rotation: Quat::IDENTITY,
            translation: translation.into(),
        }
    }

    /// Transform a point by rotating and translating it using this isometry.
    #[inline]
    pub fn transform_point(&self, point: impl Into<Vec3A>) -> Vec3A {
        self.rotation * point.into() + self.translation
    }
}

impl From<Isometry3d> for Affine3 {
    #[inline]
    fn from(iso: Isometry3d) -> Self {
        Affine3 {
            matrix3: Mat3::from_quat(iso.rotation),
            translation: iso.translation.into(),
        }
    }
}

impl From<Isometry3d> for Affine3A {
    #[inline]
    fn from(iso: Isometry3d) -> Self {
        Affine3A {
            matrix3: Mat3A::from_quat(iso.rotation),
            translation: iso.translation,
        }
    }
}

impl From<Vec3> for Isometry3d {
    #[inline]
    fn from(translation: Vec3) -> Self {
        Isometry3d::from_translation(translation)
    }
}

impl From<Vec3A> for Isometry3d {
    #[inline]
    fn from(translation: Vec3A) -> Self {
        Isometry3d::from_translation(translation)
    }
}

impl From<Quat> for Isometry3d {
    #[inline]
    fn from(rotation: Quat) -> Self {
        Isometry3d::from_rotation(rotation)
    }
}

impl Mul for Isometry3d {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: Self) -> Self::Output {
        Isometry3d {
            rotation: self.rotation * rhs.rotation,
            translation: self.rotation * rhs.translation + self.translation,
        }
    }
}

impl Mul<Vec3A> for Isometry3d {
    type Output = Vec3A;

    #[inline]
    fn mul(self, rhs: Vec3A) -> Self::Output {
        self.transform_point(rhs)
    }
}

impl Mul<Vec3> for Isometry3d {
    type Output = Vec3;

    #[inline]
    fn mul(self, rhs: Vec3) -> Self::Output {
        self.transform_point(rhs).into()
    }
}

impl Mul<Dir3> for Isometry3d {
    type Output = Dir3;

    #[inline]
    fn mul(self, rhs: Dir3) -> Self::Output {
        self.rotation * rhs
    }
}

#[cfg(feature = "approx")]
impl AbsDiffEq for Isometry3d {
    type Epsilon = <f32 as AbsDiffEq>::Epsilon;

    fn default_epsilon() -> Self::Epsilon {
        f32::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.rotation.abs_diff_eq(other.rotation, epsilon)
            && self.translation.abs_diff_eq(other.translation, epsilon)
    }
}

#[cfg(feature = "approx")]
impl RelativeEq for Isometry3d {
    fn default_max_relative() -> Self::Epsilon {
        Self::default_epsilon()
    }

    fn relative_eq(
        &self,
        other: &Self,
        epsilon: Self::Epsilon,
        max_relative: Self::Epsilon,
    ) -> bool {
        self.rotation
            .relative_eq(&other.rotation, epsilon, max_relative)
            && self
                .translation
                .relative_eq(&other.translation, epsilon, max_relative)
    }
}

#[cfg(feature = "approx")]
impl UlpsEq for Isometry3d {
    fn default_max_ulps() -> u32 {
        4
    }

    fn ulps_eq(&self, other: &Self, epsilon: Self::Epsilon, max_ulps: u32) -> bool {
        self.rotation.ulps_eq(&other.rotation, epsilon, max_ulps)
            && self
                .translation
                .ulps_eq(&other.translation, epsilon, max_ulps)
    }
}

#[cfg(test)]
#[cfg(feature = "approx")]
mod tests {
    use super::*;
    use crate::vec3;
    use approx::assert_abs_diff_eq;
    use core::f32::consts::FRAC_PI_2;

    #[test]
    fn mul_3d() {
        let iso1 = Isometry3d::new(vec3(1.0, 0.0, 0.0), Quat::from_rotation_x(FRAC_PI_2));
        let iso2 = Isometry3d::new(vec3(0.0, 1.0, 0.0), Quat::IDENTITY);
        let expected = Isometry3d::new(vec3(1.0, 0.0, 1.0), Quat::from_rotation_x(FRAC_PI_2));
        assert_abs_diff_eq!(iso1 * iso2, expected);
    }

    #[test]
    fn transform_3d() {
        let iso = Isometry3d::new(vec3(1.0, 0.0, 0.0), Quat::from_rotation_y(FRAC_PI_2));
        let point = vec3(1.0, 1.0, 1.0);
        assert_abs_diff_eq!(vec3(2.0, 1.0, -1.0), iso * point);
    }
}
