use crate::{Rect, UVec2};

/// A rectangle defined by two opposite corners.
///
/// The rectangle is axis aligned, and defined by its minimum and maximum coordinates,
/// stored in `URect::min` and `URect::max`, respectively. The minimum/maximum invariant
/// must be upheld by the user when directly assigning the fields, otherwise some methods
/// produce invalid results. It is generally recommended to use one of the constructor
/// methods instead, which will ensure this invariant is met, unless you already have
/// the minimum and maximum corners.
#[repr(C)]
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct URect {
    /// The minimum corner point of the rect.
    pub min: UVec2,
    /// The maximum corner point of the rect.
    pub max: UVec2,
}

impl URect {
    /// Create a new rectangle from two corner points.
    ///
    /// The two points do not need to be the minimum and/or maximum corners.
    /// They only need to be two opposite corners.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_math::URect;
    /// let r = URect::new(0, 4, 10, 6); // w=10 h=2
    /// let r = URect::new(2, 4, 5, 0); // w=3 h=4
    /// ```
    #[inline]
    pub fn new(x0: u32, y0: u32, x1: u32, y1: u32) -> Self {
        Self::from_corners(UVec2::new(x0, y0), UVec2::new(x1, y1))
    }

    /// Create a new rectangle from two corner points.
    ///
    /// The two points do not need to be the minimum and/or maximum corners.
    /// They only need to be two opposite corners.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_math::{URect, UVec2};
    /// // Unit rect from [0,0] to [1,1]
    /// let r = URect::from_corners(UVec2::ZERO, UVec2::ONE); // w=1 h=1
    /// // Same; the points do not need to be ordered
    /// let r = URect::from_corners(UVec2::ONE, UVec2::ZERO); // w=1 h=1
    /// ```
    #[inline]
    pub fn from_corners(p0: UVec2, p1: UVec2) -> Self {
        Self {
            min: p0.min(p1),
            max: p0.max(p1),
        }
    }

    /// Check if the rectangle is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_math::{URect, UVec2};
    /// let r = URect::from_corners(UVec2::ZERO, UVec2::new(0, 1)); // w=0 h=1
    /// assert!(r.is_empty());
    /// ```
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.min.cmpge(self.max).any()
    }

    /// Rectangle width (max.x - min.x).
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_math::URect;
    /// let r = URect::new(0, 0, 5, 1); // w=5 h=1
    /// assert_eq!(r.width(), 5);
    /// ```
    #[inline]
    pub const fn width(&self) -> u32 {
        self.max.x - self.min.x
    }

    /// Rectangle height (max.y - min.y).
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_math::URect;
    /// let r = URect::new(0, 0, 5, 1); // w=5 h=1
    /// assert_eq!(r.height(), 1);
    /// ```
    #[inline]
    pub const fn height(&self) -> u32 {
        self.max.y - self.min.y
    }

    /// Rectangle size.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_math::{URect, UVec2};
    /// let r = URect::new(0, 0, 5, 1); // w=5 h=1
    /// assert_eq!(r.size(), UVec2::new(5, 1));
    /// ```
    #[inline]
    pub fn size(&self) -> UVec2 {
        self.max - self.min
    }

    /// Build a new rectangle formed of the intersection of this rectangle and another rectangle.
    ///
    /// The intersection is the largest rectangle enclosed in both rectangles. If the intersection
    /// is empty, this method returns an empty rectangle ([`URect::is_empty()`] returns `true`), but
    /// the actual values of [`URect::min`] and [`URect::max`] are implementation-dependent.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_math::{URect, UVec2};
    /// let r1 = URect::new(0, 0, 2, 2); // w=2 h=2
    /// let r2 = URect::new(1, 1, 3, 3); // w=2 h=2
    /// let r = r1.intersect(r2);
    /// assert_eq!(r.min, UVec2::new(1, 1));
    /// assert_eq!(r.max, UVec2::new(2, 2));
    /// ```
    #[inline]
    pub fn intersect(&self, other: Self) -> Self {
        let mut r = Self {
            min: self.min.max(other.min),
            max: self.max.min(other.max),
        };
        // Collapse min over max to enforce invariants and ensure e.g. width() or
        // height() never return a negative value.
        r.min = r.min.min(r.max);
        r
    }

    /// Returns self as [`Rect`] (f32)
    #[inline]
    pub fn as_rect(&self) -> Rect {
        Rect::from_corners(self.min.as_vec2(), self.max.as_vec2())
    }
}
