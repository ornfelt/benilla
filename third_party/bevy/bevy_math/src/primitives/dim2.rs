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
}
