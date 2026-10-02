use crate::Vec2;

/// A rectangle primitive, which is like a square, except that the width and height can be different
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[doc(alias = "Quad")]
pub struct Rectangle {
    /// Half of the width and height of the rectangle
    pub half_size: Vec2,
}

impl Default for Rectangle {
    /// Returns the default [`Rectangle`] with a half-width and half-height of `0.5`.
    fn default() -> Self {
        Self {
            half_size: Vec2::splat(0.5),
        }
    }
}

impl Rectangle {
    /// Create a new `Rectangle` from a full width and height
    #[inline]
    pub const fn new(width: f32, height: f32) -> Self {
        Self::from_size(Vec2::new(width, height))
    }

    /// Create a new `Rectangle` from a given full size
    #[inline]
    pub const fn from_size(size: Vec2) -> Self {
        Self {
            half_size: Vec2::new(size.x / 2.0, size.y / 2.0),
        }
    }

    /// Create a new `Rectangle` from two corner points
    #[inline]
    pub fn from_corners(point1: Vec2, point2: Vec2) -> Self {
        Self {
            half_size: (point2 - point1).abs() / 2.0,
        }
    }

    /// Create a `Rectangle` from a single length.
    /// The resulting `Rectangle` will be the same size in every direction.
    #[inline]
    pub const fn from_length(length: f32) -> Self {
        Self {
            half_size: Vec2::splat(length / 2.0),
        }
    }

    /// Get the size of the rectangle
    #[inline]
    pub fn size(&self) -> Vec2 {
        2.0 * self.half_size
    }

    /// Finds the point on the rectangle that is closest to the given `point`.
    ///
    /// If the point is outside the rectangle, the returned point will be on the perimeter of the rectangle.
    /// Otherwise, it will be inside the rectangle and returned as is.
    #[inline]
    pub fn closest_point(&self, point: Vec2) -> Vec2 {
        // Clamp point coordinates to the rectangle
        point.clamp(-self.half_size, self.half_size)
    }
}

#[cfg(test)]
mod tests {
    // Reference values were computed by hand and/or with external tools

    use super::*;

    #[test]
    fn rectangle_closest_point() {
        let rectangle = Rectangle::new(2.0, 2.0);
        assert_eq!(rectangle.closest_point(Vec2::X * 10.0), Vec2::X);
        assert_eq!(rectangle.closest_point(Vec2::NEG_ONE * 10.0), Vec2::NEG_ONE);
        assert_eq!(
            rectangle.closest_point(Vec2::new(0.25, 0.1)),
            Vec2::new(0.25, 0.1)
        );
    }
}
