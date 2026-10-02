//! This module contains traits and implements for working with bounding shapes
//!
//! [`BoundingVolume`] is a generic abstraction for any bounding volume.

/// A trait that generalizes different bounding volumes.
/// Bounding volumes are simplified shapes that are used to get simpler ways to check for
/// overlapping elements or finding intersections.
///
/// This trait supports both 2D and 3D bounding shapes.
pub trait BoundingVolume: Sized {
    /// The position type used for the volume. This should be `Vec2` for 2D and `Vec3` for 3D.
    type Translation: Clone + Copy + PartialEq;

    /// The type used for the size of the bounding volume. Usually a half size. For example an
    /// `f32` radius for a circle, or a `Vec3` with half sizes for x, y and z for a 3D axis-aligned
    /// bounding box
    type HalfSize;

    /// Returns the center of the bounding volume.
    fn center(&self) -> Self::Translation;

    /// Returns the half size of the bounding volume.
    fn half_size(&self) -> Self::HalfSize;
}

mod bounded3d;
pub use bounded3d::*;
