use crate::{
    ops::{self, FloatPow},
    Dir3, Vec2, Vec3,
};

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

    /// Get the diameter of the sphere
    #[inline]
    pub const fn diameter(&self) -> f32 {
        2.0 * self.radius
    }

    /// Finds the point on the sphere that is closest to the given `point`.
    ///
    /// If the point is outside the sphere, the returned point will be on the surface of the sphere.
    /// Otherwise, it will be inside the sphere and returned as is.
    #[inline]
    pub fn closest_point(&self, point: Vec3) -> Vec3 {
        let distance_squared = point.length_squared();

        if distance_squared <= self.radius.squared() {
            // The point is inside the sphere.
            point
        } else {
            // The point is outside the sphere.
            // Find the closest point on the surface of the sphere.
            let dir_to_point = point / ops::sqrt(distance_squared);
            self.radius * dir_to_point
        }
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

    /// Create a new `Plane3d` based on three points and compute the geometric center
    /// of those points.
    ///
    /// The direction of the plane normal is determined by the winding order
    /// of the triangular shape formed by the points.
    ///
    /// # Panics
    ///
    /// Panics if a valid normal can not be computed, for example when the points
    /// are *collinear* and lie on the same line.
    #[inline]
    pub fn from_points(a: Vec3, b: Vec3, c: Vec3) -> (Self, Vec3) {
        let normal = Dir3::new((b - a).cross(c - a)).expect(
            "finite plane must be defined by three finite points that don't lie on the same line",
        );
        let translation = (a + b + c) / 3.0;

        (
            Self {
                normal,
                ..Default::default()
            },
            translation,
        )
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

    /// Create a new `Cuboid` from two corner points
    #[inline]
    pub fn from_corners(point1: Vec3, point2: Vec3) -> Self {
        Self {
            half_size: (point2 - point1).abs() / 2.0,
        }
    }

    /// Create a `Cuboid` from a single length.
    /// The resulting `Cuboid` will be the same size in every direction.
    #[inline]
    pub const fn from_length(length: f32) -> Self {
        Self {
            half_size: Vec3::splat(length / 2.0),
        }
    }

    /// Get the size of the cuboid
    #[inline]
    pub fn size(&self) -> Vec3 {
        2.0 * self.half_size
    }

    /// Finds the point on the cuboid that is closest to the given `point`.
    ///
    /// If the point is outside the cuboid, the returned point will be on the surface of the cuboid.
    /// Otherwise, it will be inside the cuboid and returned as is.
    #[inline]
    pub fn closest_point(&self, point: Vec3) -> Vec3 {
        // Clamp point coordinates to the cuboid
        point.clamp(-self.half_size, self.half_size)
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

    #[test]
    fn cuboid_closest_point() {
        let cuboid = Cuboid::new(2.0, 2.0, 2.0);
        assert_eq!(cuboid.closest_point(Vec3::X * 10.0), Vec3::X);
        assert_eq!(cuboid.closest_point(Vec3::NEG_ONE * 10.0), Vec3::NEG_ONE);
        assert_eq!(
            cuboid.closest_point(Vec3::new(0.25, 0.1, 0.3)),
            Vec3::new(0.25, 0.1, 0.3)
        );
    }

    #[test]
    fn sphere_closest_point() {
        let sphere = Sphere { radius: 1.0 };
        assert_eq!(sphere.closest_point(Vec3::X * 10.0), Vec3::X);
        assert_eq!(
            sphere.closest_point(Vec3::NEG_ONE * 10.0),
            Vec3::NEG_ONE.normalize()
        );
        assert_eq!(
            sphere.closest_point(Vec3::new(0.25, 0.1, 0.3)),
            Vec3::new(0.25, 0.1, 0.3)
        );
    }

    #[test]
    fn plane_from_points() {
        let (plane, translation) = Plane3d::from_points(Vec3::X, Vec3::Z, Vec3::NEG_X);
        assert_eq!(*plane.normal, Vec3::NEG_Y, "incorrect normal");
        assert_eq!(plane.half_size, Vec2::new(0.5, 0.5), "incorrect half size");
        assert_eq!(translation, Vec3::Z * 0.33333334, "incorrect translation");
    }
}
