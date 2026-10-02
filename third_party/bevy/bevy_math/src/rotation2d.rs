use crate::prelude::Mat2;

/// A 2D rotation, stored as the unit complex number `cos + i sin`.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[doc(alias = "rotation", alias = "rotation2d", alias = "rotation_2d")]
pub struct Rot2 {
    /// The cosine of the rotation angle.
    ///
    /// This is the real part of the unit complex number representing the rotation.
    pub cos: f32,
    /// The sine of the rotation angle.
    ///
    /// This is the imaginary part of the unit complex number representing the rotation.
    pub sin: f32,
}

impl Default for Rot2 {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Rot2 {
    /// No rotation.
    /// Also equals a full turn that returns back to its original position.
    pub const IDENTITY: Self = Self { cos: 1.0, sin: 0.0 };
}

impl From<Rot2> for Mat2 {
    /// Creates a [`Mat2`] rotation matrix from a [`Rot2`].
    fn from(rot: Rot2) -> Self {
        Mat2::from_cols_array(&[rot.cos, rot.sin, -rot.sin, rot.cos])
    }
}

#[cfg(any(feature = "approx", test))]
impl approx::AbsDiffEq for Rot2 {
    type Epsilon = f32;
    fn default_epsilon() -> f32 {
        f32::EPSILON
    }
    fn abs_diff_eq(&self, other: &Self, epsilon: f32) -> bool {
        self.cos.abs_diff_eq(&other.cos, epsilon) && self.sin.abs_diff_eq(&other.sin, epsilon)
    }
}

#[cfg(any(feature = "approx", test))]
impl approx::RelativeEq for Rot2 {
    fn default_max_relative() -> f32 {
        f32::EPSILON
    }
    fn relative_eq(&self, other: &Self, epsilon: f32, max_relative: f32) -> bool {
        self.cos.relative_eq(&other.cos, epsilon, max_relative)
            && self.sin.relative_eq(&other.sin, epsilon, max_relative)
    }
}

#[cfg(any(feature = "approx", test))]
impl approx::UlpsEq for Rot2 {
    fn default_max_ulps() -> u32 {
        4
    }
    fn ulps_eq(&self, other: &Self, epsilon: f32, max_ulps: u32) -> bool {
        self.cos.ulps_eq(&other.cos, epsilon, max_ulps)
            && self.sin.ulps_eq(&other.sin, epsilon, max_ulps)
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use crate::{Mat2, Rot2, Vec2};

    #[test]
    fn rotation_matrix() {
        // A quarter turn counterclockwise.
        let rotation = Rot2 { cos: 0.0, sin: 1.0 };
        let matrix: Mat2 = rotation.into();

        // Check that the matrix is correct.
        assert_relative_eq!(matrix.x_axis, Vec2::Y);
        assert_relative_eq!(matrix.y_axis, Vec2::NEG_X);

        // Check that the matrix rotates vectors correctly.
        assert_relative_eq!(matrix * Vec2::X, Vec2::Y);
        assert_relative_eq!(matrix * Vec2::Y, Vec2::NEG_X);
        assert_relative_eq!(matrix * Vec2::NEG_X, Vec2::NEG_Y);
        assert_relative_eq!(matrix * Vec2::NEG_Y, Vec2::X);
    }
}
