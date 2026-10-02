//! Core data structures to be used internally in Curve implementations, encapsulating storage
//! and access patterns for reuse.
//!
//! The `Core` types here expose their fields publicly so that it is easier to manipulate and
//! extend them, but in doing so, you must maintain the invariants of those fields yourself. The
//! provided methods all maintain the invariants, so this is only a concern if you manually mutate
//! the fields.

use super::interval::Interval;
use core::fmt::Debug;
use thiserror::Error;

#[cfg(feature = "alloc")]
use {alloc::vec::Vec, itertools::Itertools};

#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;

/// This type expresses the relationship of a value to a fixed collection of values. It is a kind
/// of summary used intermediately by sampling operations.
#[derive(Debug, Copy, Clone, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub enum InterpolationDatum<T> {
    /// This value lies exactly on a value in the family.
    Exact(T),

    /// This value is off the left tail of the family; the inner value is the family's leftmost.
    LeftTail(T),

    /// This value is off the right tail of the family; the inner value is the family's rightmost.
    RightTail(T),

    /// This value lies on the interior, in between two points, with a third parameter expressing
    /// the interpolation factor between the two.
    Between(T, T, f32),
}

/// The data core of a curve defined by unevenly-spaced samples or keyframes. The intention is to
/// use this in concert with implicitly or explicitly-defined interpolation in user-space in
/// order to implement the curve interface using [`domain`] and [`sample_with`].
///
/// The internals are made transparent to give curve authors freedom, but [the provided constructor]
/// enforces the required invariants, and the methods maintain those invariants.
///
/// # Example
/// ```rust
/// # use bevy_math::curve::*;
/// # use bevy_math::curve::cores::*;
/// // Let's make a curve formed by interpolating rotations.
/// // We'll support two common modes of interpolation:
/// // - Normalized linear: First do linear interpolation, then normalize to get a valid rotation.
/// // - Spherical linear: Interpolate through valid rotations with constant angular velocity.
/// enum InterpolationMode {
///     NormalizedLinear,
///     SphericalLinear,
/// }
///
/// // Our interpolation modes will be driven by traits.
/// trait NormalizedLinearInterpolate {
///     fn nlerp(&self, other: &Self, t: f32) -> Self;
/// }
///
/// trait SphericalLinearInterpolate {
///     fn slerp(&self, other: &Self, t: f32) -> Self;
/// }
///
/// // Omitted: These traits would be implemented for `Rot2`, `Quat`, and other rotation representations.
///
/// // The curve itself just needs to use the curve core for keyframes, `UnevenCore`, which handles
/// // everything except for the explicit interpolation used.
/// struct RotationCurve<T> {
///     core: UnevenCore<T>,
///     interpolation_mode: InterpolationMode,
/// }
///
/// impl<T> Curve<T> for RotationCurve<T>
/// where
///     T: NormalizedLinearInterpolate + SphericalLinearInterpolate + Clone,
/// {
///     fn domain(&self) -> Interval {
///         self.core.domain()
///     }
///     
///     fn sample_unchecked(&self, t: f32) -> T {
///         // To sample the curve, we just look at the interpolation mode and
///         // dispatch accordingly.
///         match self.interpolation_mode {
///             InterpolationMode::NormalizedLinear =>
///                 self.core.sample_with(t, <T as NormalizedLinearInterpolate>::nlerp),
///             InterpolationMode::SphericalLinear =>
///                 self.core.sample_with(t, <T as SphericalLinearInterpolate>::slerp),
///         }
///     }
/// }
/// ```
///
/// [`domain`]: UnevenCore::domain
/// [`sample_with`]: UnevenCore::sample_with
/// [the provided constructor]: UnevenCore::new
#[cfg(feature = "alloc")]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
pub struct UnevenCore<T> {
    /// The times for the samples of this curve.
    ///
    /// # Invariants
    /// This must always have a length of at least 2, be sorted, and have no
    /// duplicated or non-finite times.
    pub times: Vec<f32>,

    /// The samples corresponding to the times for this curve.
    ///
    /// # Invariants
    /// This must always have the same length as `times`.
    pub samples: Vec<T>,
}

/// An error indicating that an [`UnevenCore`] could not be constructed.
#[derive(Debug, Error)]
#[error("Could not construct an UnevenCore")]
pub enum UnevenCoreError {
    /// Not enough samples were provided.
    #[error(
        "Need at least two unique samples to create an UnevenCore, but {samples} were provided"
    )]
    NotEnoughSamples {
        /// The number of samples that were provided.
        samples: usize,
    },
}

#[cfg(feature = "alloc")]
impl<T> UnevenCore<T> {
    /// Create a new [`UnevenCore`]. The given samples are filtered to finite times and
    /// sorted internally; if there are not at least 2 valid timed samples, an error will be
    /// returned.
    pub fn new(timed_samples: impl IntoIterator<Item = (f32, T)>) -> Result<Self, UnevenCoreError> {
        // Filter out non-finite sample times first so they don't interfere with sorting/deduplication.
        let mut timed_samples = timed_samples
            .into_iter()
            .filter(|(t, _)| t.is_finite())
            .collect_vec();
        timed_samples
            // Using `total_cmp` is fine because no NANs remain and because deduplication uses
            // `PartialEq` anyway (so -0.0 and 0.0 will be considered equal later regardless).
            .sort_by(|(t0, _), (t1, _)| t0.total_cmp(t1));
        timed_samples.dedup_by_key(|(t, _)| *t);

        if timed_samples.len() < 2 {
            return Err(UnevenCoreError::NotEnoughSamples {
                samples: timed_samples.len(),
            });
        }

        let (times, samples): (Vec<f32>, Vec<T>) = timed_samples.into_iter().unzip();
        Ok(UnevenCore { times, samples })
    }

    /// The domain of the curve derived from this core.
    ///
    /// # Panics
    /// This method may panic if the type's invariants aren't satisfied.
    #[inline]
    pub fn domain(&self) -> Interval {
        let start = self.times.first().unwrap();
        let end = self.times.last().unwrap();
        Interval::new(*start, *end).unwrap()
    }

    /// Obtain a value from the held samples using the given `interpolation` to interpolate
    /// between adjacent samples.
    ///
    /// The interpolation takes two values by reference together with a scalar parameter and
    /// produces an owned value. The expectation is that `interpolation(&x, &y, 0.0)` and
    /// `interpolation(&x, &y, 1.0)` are equivalent to `x` and `y` respectively.
    #[inline]
    pub fn sample_with<I>(&self, t: f32, interpolation: I) -> T
    where
        T: Clone,
        I: Fn(&T, &T, f32) -> T,
    {
        match uneven_interp(&self.times, t) {
            InterpolationDatum::Exact(idx)
            | InterpolationDatum::LeftTail(idx)
            | InterpolationDatum::RightTail(idx) => self.samples[idx].clone(),
            InterpolationDatum::Between(lower_idx, upper_idx, s) => {
                interpolation(&self.samples[lower_idx], &self.samples[upper_idx], s)
            }
        }
    }
}

/// Given a list of `times` and a target value, get the interpolation relationship for the
/// target value in terms of the indices of the starting list. In a sense, this encapsulates the
/// heart of uneven/keyframe sampling.
///
/// `times` is assumed to be sorted, deduplicated, and consisting only of finite values. It is also
/// assumed to contain at least two values.
///
/// # Panics
/// This function will panic if `times` contains NAN.
pub fn uneven_interp(times: &[f32], t: f32) -> InterpolationDatum<usize> {
    match times.binary_search_by(|pt| pt.partial_cmp(&t).unwrap()) {
        Ok(index) => InterpolationDatum::Exact(index),
        Err(index) => {
            if index == 0 {
                // This is before the first keyframe.
                InterpolationDatum::LeftTail(0)
            } else if index >= times.len() {
                // This is after the last keyframe.
                InterpolationDatum::RightTail(times.len() - 1)
            } else {
                // This is actually in the middle somewhere.
                let t_lower = times[index - 1];
                let t_upper = times[index];
                let s = (t - t_lower) / (t_upper - t_lower);
                InterpolationDatum::Between(index - 1, index, s)
            }
        }
    }
}

#[cfg(all(test, feature = "alloc"))]
mod tests {
    use super::UnevenCore;
    use alloc::vec;
    use approx::assert_abs_diff_eq;

    #[test]
    fn uneven_sample_with() {
        let uneven_core = UnevenCore::<f32>::new(vec![
            (0.0, 0.0),
            (1.0, 3.0),
            (2.0, 9.0),
            (4.0, 10.0),
            (8.0, -5.0),
        ])
        .expect("Failed to construct test core");
        let sample = |t: f32| uneven_core.sample_with(t, |a: &f32, b: &f32, s| a + (b - a) * s);

        // The tails clamp to the first and last keyframes.
        assert_eq!(sample(-1.0), 0.0);
        assert_eq!(sample(9.0), -5.0);

        // Exact keyframes.
        assert_eq!(sample(0.0), 0.0);
        assert_eq!(sample(2.0), 9.0);
        assert_eq!(sample(4.0), 10.0);
        assert_eq!(sample(8.0), -5.0);

        // Between keyframes, at the fraction of the gap.
        assert_abs_diff_eq!(sample(0.5), 1.5, epsilon = 1e-6);
        assert_abs_diff_eq!(sample(2.5), 9.25, epsilon = 1e-6);
        assert_abs_diff_eq!(sample(7.0), -1.25, epsilon = 1e-6);
    }
}
