//! The [`Curve`] trait, providing a domain-agnostic description of curves.
//!
//! A curve has a [domain], which is a nonempty range of `f32` values, and can be [sampled] on
//! every one of those values, producing output of some fixed type. [`UnevenCore`] holds the data
//! of a curve defined by unevenly-spaced samples or keyframes.
//!
//! [domain]: Curve::domain
//! [sampled]: Curve::sample

pub mod cores;
pub mod interval;

// bevy_math::curve re-exports all commonly-needed curve-related items.
pub use interval::Interval;

#[cfg(feature = "alloc")]
pub use cores::UnevenCore;

use core::ops::Deref;

/// A trait for a type that can represent values of type `T` parametrized over a fixed interval.
///
/// Typical examples of this are actual geometric curves where `T: VectorSpace`, but other kinds
/// of output data can be represented as well. See the [module-level documentation] for details.
///
/// [module-level documentation]: self
pub trait Curve<T> {
    /// The interval over which this curve is parametrized.
    ///
    /// This is the range of values of `t` where we can sample the curve and receive valid output.
    fn domain(&self) -> Interval;

    /// Sample a point on this curve at the parameter value `t`, extracting the associated value.
    /// This is the unchecked version of sampling, which should only be used if the sample time `t`
    /// is already known to lie within the curve's domain.
    ///
    /// Values sampled from outside of a curve's domain are generally considered invalid; data which
    /// is nonsensical or otherwise useless may be returned in such a circumstance, and extrapolation
    /// beyond a curve's domain should not be relied upon.
    fn sample_unchecked(&self, t: f32) -> T;

    /// Sample a point on this curve at the parameter value `t`, returning `None` if the point is
    /// outside of the curve's domain.
    fn sample(&self, t: f32) -> Option<T> {
        match self.domain().contains(t) {
            true => Some(self.sample_unchecked(t)),
            false => None,
        }
    }

    /// Sample a point on this curve at the parameter value `t`, clamping `t` to lie inside the
    /// domain of the curve.
    fn sample_clamped(&self, t: f32) -> T {
        let t = self.domain().clamp(t);
        self.sample_unchecked(t)
    }
}

impl<T, C, D> Curve<T> for D
where
    C: Curve<T> + ?Sized,
    D: Deref<Target = C>,
{
    fn domain(&self) -> Interval {
        <C as Curve<T>>::domain(self)
    }

    fn sample_unchecked(&self, t: f32) -> T {
        <C as Curve<T>>::sample_unchecked(self, t)
    }
}
