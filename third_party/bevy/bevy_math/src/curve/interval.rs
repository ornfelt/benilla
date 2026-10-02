//! The [`Interval`] type for nonempty intervals used by the [`Curve`](super::Curve) trait.

use core::ops::RangeInclusive;
use thiserror::Error;

/// A nonempty closed interval, possibly unbounded in either direction.
///
/// In other words, the interval may stretch all the way to positive or negative infinity, but it
/// will always have some nonempty interior.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct Interval {
    start: f32,
    end: f32,
}

/// An error that indicates that an operation would have returned an invalid [`Interval`].
#[derive(Debug, Error)]
#[error("The resulting interval would be invalid (empty or with a NaN endpoint)")]
pub struct InvalidIntervalError;

impl Interval {
    /// Create a new [`Interval`] with the specified `start` and `end`. The interval can be unbounded
    /// but cannot be empty (so `start` must be less than `end`) and neither endpoint can be NaN; invalid
    /// parameters will result in an error.
    #[inline]
    pub const fn new(start: f32, end: f32) -> Result<Self, InvalidIntervalError> {
        if start >= end || start.is_nan() || end.is_nan() {
            Err(InvalidIntervalError)
        } else {
            Ok(Self { start, end })
        }
    }

    /// Get the end of this interval.
    #[inline]
    pub const fn end(self) -> f32 {
        self.end
    }

    /// Returns `true` if `item` is contained in this interval.
    #[inline]
    pub fn contains(self, item: f32) -> bool {
        (self.start..=self.end).contains(&item)
    }

    /// Clamp the given `value` to lie within this interval.
    #[inline]
    pub const fn clamp(self, value: f32) -> f32 {
        value.clamp(self.start, self.end)
    }
}

impl TryFrom<RangeInclusive<f32>> for Interval {
    type Error = InvalidIntervalError;
    fn try_from(range: RangeInclusive<f32>) -> Result<Self, Self::Error> {
        Interval::new(*range.start(), *range.end())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn make_intervals() {
        let ivl = Interval::new(2.0, -1.0);
        assert!(ivl.is_err());

        let ivl = Interval::new(-0.0, 0.0);
        assert!(ivl.is_err());

        let ivl = Interval::new(f32::NEG_INFINITY, 15.5);
        assert!(ivl.is_ok());

        let ivl = Interval::new(-2.0, f32::INFINITY);
        assert!(ivl.is_ok());

        let ivl = Interval::new(f32::NEG_INFINITY, f32::INFINITY);
        assert!(ivl.is_ok());

        let ivl = Interval::new(f32::INFINITY, f32::NEG_INFINITY);
        assert!(ivl.is_err());

        let ivl = Interval::new(-1.0, f32::NAN);
        assert!(ivl.is_err());

        let ivl = Interval::new(f32::NAN, -42.0);
        assert!(ivl.is_err());

        let ivl = Interval::new(f32::NAN, f32::NAN);
        assert!(ivl.is_err());

        let ivl = Interval::new(0.0, 1.0);
        assert!(ivl.is_ok());
    }

    #[test]
    fn containment() {
        let ivl = Interval::new(0.0, 1.0).unwrap();
        assert!(ivl.contains(0.0));
        assert!(ivl.contains(1.0));
        assert!(ivl.contains(0.5));
        assert!(!ivl.contains(-0.1));
        assert!(!ivl.contains(1.1));
        assert!(!ivl.contains(f32::NAN));

        let ivl = Interval::new(3.0, f32::INFINITY).unwrap();
        assert!(ivl.contains(3.0));
        assert!(ivl.contains(2.0e5));
        assert!(ivl.contains(3.5e6));
        assert!(!ivl.contains(2.5));
        assert!(!ivl.contains(-1e5));
        assert!(!ivl.contains(f32::NAN));
    }
}
