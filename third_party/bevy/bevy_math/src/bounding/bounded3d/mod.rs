use super::BoundingVolume;
use crate::Vec3A;

#[cfg(feature = "serialize")]
use serde::{Deserialize, Serialize};

/// A 3D axis-aligned bounding box
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(Serialize), derive(Deserialize))]
pub struct Aabb3d {
    /// The minimum point of the box
    pub min: Vec3A,
    /// The maximum point of the box
    pub max: Vec3A,
}

impl BoundingVolume for Aabb3d {
    type Translation = Vec3A;
    type HalfSize = Vec3A;

    #[inline]
    fn center(&self) -> Self::Translation {
        (self.min + self.max) / 2.
    }

    #[inline]
    fn half_size(&self) -> Self::HalfSize {
        (self.max - self.min) / 2.
    }
}

#[cfg(test)]
mod aabb3d_tests {
    use super::Aabb3d;
    use crate::{bounding::BoundingVolume, Vec3A};

    #[test]
    fn center() {
        let aabb = Aabb3d {
            min: Vec3A::new(-0.5, -1., -0.5),
            max: Vec3A::new(1., 1., 2.),
        };
        assert!((aabb.center() - Vec3A::new(0.25, 0., 0.75)).length() < f32::EPSILON);
        let aabb = Aabb3d {
            min: Vec3A::new(5., 5., -10.),
            max: Vec3A::new(10., 10., -5.),
        };
        assert!((aabb.center() - Vec3A::new(7.5, 7.5, -7.5)).length() < f32::EPSILON);
    }

    #[test]
    fn half_size() {
        let aabb = Aabb3d {
            min: Vec3A::new(-0.5, -1., -0.5),
            max: Vec3A::new(1., 1., 2.),
        };
        assert!((aabb.half_size() - Vec3A::new(0.75, 1., 1.25)).length() < f32::EPSILON);
    }
}
