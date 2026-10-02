use crate::{Dir3, Vec2, Vec3};

/// A sphere primitive, representing the set of all points some distance from the origin
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct Sphere {
    /// The radius of the sphere
    pub radius: f32,
}

impl Default for Sphere {
    /// Returns the default [`Sphere`] with a radius of `0.5`.
    fn default() -> Self {
        Self { radius: 0.5 }
    }
}

impl Sphere {
    /// Create a new [`Sphere`] from a `radius`
    #[inline]
    pub const fn new(radius: f32) -> Self {
        Self { radius }
    }
}

/// A bounded plane in 3D space. It forms a surface starting from the origin with a defined height and width.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct Plane3d {
    /// The normal of the plane. The plane will be placed perpendicular to this direction
    pub normal: Dir3,
    /// Half of the width and height of the plane
    pub half_size: Vec2,
}

impl Default for Plane3d {
    /// Returns the default [`Plane3d`] with a normal pointing in the `+Y` direction, width and height of `1.0`.
    fn default() -> Self {
        Self {
            normal: Dir3::Y,
            half_size: Vec2::splat(0.5),
        }
    }
}

impl Plane3d {
    /// Create a new `Plane3d` from a normal and a half size
    ///
    /// # Panics
    ///
    /// Panics if the given `normal` is zero (or very close to zero), or non-finite.
    #[inline]
    pub fn new(normal: Vec3, half_size: Vec2) -> Self {
        Self {
            normal: Dir3::new(normal).expect("normal must be nonzero and finite"),
            half_size,
        }
    }
}

/// A cuboid primitive, which is like a cube, except that the x, y, and z dimensions are not
/// required to be the same.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct Cuboid {
    /// Half of the width, height and depth of the cuboid
    pub half_size: Vec3,
}

impl Default for Cuboid {
    /// Returns the default [`Cuboid`] with a width, height, and depth of `1.0`.
    fn default() -> Self {
        Self {
            half_size: Vec3::splat(0.5),
        }
    }
}

impl Cuboid {
    /// Create a new `Cuboid` from a full x, y, and z length
    #[inline]
    pub const fn new(x_length: f32, y_length: f32, z_length: f32) -> Self {
        Self::from_size(Vec3::new(x_length, y_length, z_length))
    }

    /// Create a new `Cuboid` from a given full size
    #[inline]
    pub const fn from_size(size: Vec3) -> Self {
        Self {
            half_size: Vec3::new(size.x / 2.0, size.y / 2.0, size.z / 2.0),
        }
    }
}

#[cfg(test)]
mod tests {
    // Reference values were computed by hand and/or with external tools

    use super::*;
    use crate::{InvalidDirectionError, Quat};

    #[test]
    fn direction_creation() {
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
