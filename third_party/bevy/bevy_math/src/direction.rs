use crate::{Quat, Vec3};

use core::fmt;
use derive_more::derive::Into;

#[cfg(all(debug_assertions, feature = "std"))]
use std::eprintln;

use thiserror::Error;

/// An error indicating that a direction is invalid.
#[derive(Debug, PartialEq, Error)]
pub enum InvalidDirectionError {
    /// The length of the direction vector is zero or very close to zero.
    #[error("The length of the direction vector is zero or very close to zero")]
    Zero,
    /// The length of the direction vector is `std::f32::INFINITY`.
    #[error("The length of the direction vector is `std::f32::INFINITY`")]
    Infinite,
    /// The length of the direction vector is `NaN`.
    #[error("The length of the direction vector is `NaN`")]
    NaN,
}

impl InvalidDirectionError {
    /// Creates an [`InvalidDirectionError`] from the length of an invalid direction vector.
    pub fn from_length(length: f32) -> Self {
        if length.is_nan() {
            InvalidDirectionError::NaN
        } else if !length.is_finite() {
            // If the direction is non-finite but also not NaN, it must be infinite
            InvalidDirectionError::Infinite
        } else {
            // If the direction is invalid but neither NaN nor infinite, it must be zero
            InvalidDirectionError::Zero
        }
    }
}

/// Checks that a vector with the given squared length is normalized.
///
/// Warns for small error with a length threshold of approximately `1e-4`,
/// and panics for large error with a length threshold of approximately `1e-2`.
///
/// The format used for the logged warning is `"Warning: {warning} The length is {length}`,
/// and similarly for the error.
#[cfg(debug_assertions)]
fn assert_is_normalized(message: &str, length_squared: f32) {
    use crate::ops;

    let length_error_squared = ops::abs(length_squared - 1.0);

    // Panic for large error and warn for slight error.
    if length_error_squared > 2e-2 || length_error_squared.is_nan() {
        // Length error is approximately 1e-2 or more.
        panic!(
            "Error: {message} The length is {}.",
            ops::sqrt(length_squared)
        );
    } else if length_error_squared > 2e-4 {
        // Length error is approximately 1e-4 or more.
        #[cfg(feature = "std")]
        #[expect(clippy::print_stderr, reason = "Allowed behind `std` feature gate.")]
        {
            eprintln!(
                "Warning: {message} The length is {}.",
                ops::sqrt(length_squared)
            );
        }
    }
}

/// A normalized vector pointing in a direction in 3D space
#[derive(Clone, Copy, Debug, PartialEq, Into)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[doc(alias = "Direction3d")]
pub struct Dir3(Vec3);
impl Dir3 {
    /// A unit vector pointing along the positive X axis.
    pub const X: Self = Self(Vec3::X);
    /// A unit vector pointing along the positive Y axis.
    pub const Y: Self = Self(Vec3::Y);
    /// A unit vector pointing along the negative Y axis.
    pub const NEG_Y: Self = Self(Vec3::NEG_Y);
    /// A unit vector pointing along the negative Z axis.
    pub const NEG_Z: Self = Self(Vec3::NEG_Z);

    /// Create a direction from a finite, nonzero [`Vec3`], normalizing it.
    ///
    /// Returns [`Err(InvalidDirectionError)`](InvalidDirectionError) if the length
    /// of the given vector is zero (or very close to zero), infinite, or `NaN`.
    pub fn new(value: Vec3) -> Result<Self, InvalidDirectionError> {
        Self::new_and_length(value).map(|(dir, _)| dir)
    }

    /// Create a [`Dir3`] from a [`Vec3`] that is already normalized.
    ///
    /// # Warning
    ///
    /// `value` must be normalized, i.e its length must be `1.0`.
    pub fn new_unchecked(value: Vec3) -> Self {
        #[cfg(debug_assertions)]
        assert_is_normalized(
            "The vector given to `Dir3::new_unchecked` is not normalized.",
            value.length_squared(),
        );

        Self(value)
    }

    /// Create a direction from a finite, nonzero [`Vec3`], normalizing it and
    /// also returning its original length.
    ///
    /// Returns [`Err(InvalidDirectionError)`](InvalidDirectionError) if the length
    /// of the given vector is zero (or very close to zero), infinite, or `NaN`.
    pub fn new_and_length(value: Vec3) -> Result<(Self, f32), InvalidDirectionError> {
        let length = value.length();
        let direction = (length.is_finite() && length > 0.0).then_some(value / length);

        direction
            .map(|dir| (Self(dir), length))
            .ok_or(InvalidDirectionError::from_length(length))
    }

    /// Returns the inner [`Vec3`]
    pub const fn as_vec3(&self) -> Vec3 {
        self.0
    }
}

impl TryFrom<Vec3> for Dir3 {
    type Error = InvalidDirectionError;

    fn try_from(value: Vec3) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl core::ops::Deref for Dir3 {
    type Target = Vec3;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl core::ops::Neg for Dir3 {
    type Output = Self;
    fn neg(self) -> Self::Output {
        Self(-self.0)
    }
}

impl core::ops::Mul<f32> for Dir3 {
    type Output = Vec3;
    fn mul(self, rhs: f32) -> Self::Output {
        self.0 * rhs
    }
}

impl core::ops::Mul<Dir3> for f32 {
    type Output = Vec3;
    fn mul(self, rhs: Dir3) -> Self::Output {
        self * rhs.0
    }
}

impl core::ops::Mul<Dir3> for Quat {
    type Output = Dir3;

    /// Rotates the [`Dir3`] using a [`Quat`].
    fn mul(self, direction: Dir3) -> Self::Output {
        let rotated = self * *direction;

        #[cfg(debug_assertions)]
        assert_is_normalized(
            "`Dir3` is denormalized after rotation.",
            rotated.length_squared(),
        );

        Dir3(rotated)
    }
}

impl fmt::Display for Dir3 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(feature = "approx")]
impl approx::AbsDiffEq for Dir3 {
    type Epsilon = f32;
    fn default_epsilon() -> f32 {
        f32::EPSILON
    }
    fn abs_diff_eq(&self, other: &Self, epsilon: f32) -> bool {
        self.as_ref().abs_diff_eq(other.as_ref(), epsilon)
    }
}

#[cfg(feature = "approx")]
impl approx::RelativeEq for Dir3 {
    fn default_max_relative() -> f32 {
        f32::EPSILON
    }
    fn relative_eq(&self, other: &Self, epsilon: f32, max_relative: f32) -> bool {
        self.as_ref()
            .relative_eq(other.as_ref(), epsilon, max_relative)
    }
}

#[cfg(feature = "approx")]
impl approx::UlpsEq for Dir3 {
    fn default_max_ulps() -> u32 {
        4
    }
    fn ulps_eq(&self, other: &Self, epsilon: f32, max_ulps: u32) -> bool {
        self.as_ref().ulps_eq(other.as_ref(), epsilon, max_ulps)
    }
}

#[cfg(test)]
#[cfg(feature = "approx")]
mod tests {
    use super::*;

    #[test]
    fn dir3_creation() {
        assert_eq!(Dir3::new(Vec3::X * 12.5), Ok(Dir3::X));
        assert_eq!(
            Dir3::new(Vec3::new(0.0, 0.0, 0.0)),
            Err(InvalidDirectionError::Zero)
        );
        assert_eq!(
            Dir3::new(Vec3::new(f32::INFINITY, 0.0, 0.0)),
            Err(InvalidDirectionError::Infinite)
        );
        assert_eq!(
            Dir3::new(Vec3::new(f32::NEG_INFINITY, 0.0, 0.0)),
            Err(InvalidDirectionError::Infinite)
        );
        assert_eq!(
            Dir3::new(Vec3::new(f32::NAN, 0.0, 0.0)),
            Err(InvalidDirectionError::NaN)
        );
        assert_eq!(Dir3::new_and_length(Vec3::X * 6.5), Ok((Dir3::X, 6.5)));

        // Test rotation
        assert!(
            (Quat::from_rotation_z(core::f32::consts::FRAC_PI_2) * Dir3::X)
                .abs_diff_eq(Vec3::Y, 10e-6)
        );
    }
}
