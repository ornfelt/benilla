//! This mod re-exports the correct versions of floating-point operations with
//! unspecified precision in the standard library depending on whether the `libm`
//! crate feature is enabled.
//!
//! All the functions here are named according to their versions in the standard
//! library.
//!
//! It also provides `no_std` compatible alternatives to certain floating-point
//! operations which are not provided in the [`core`] library.

// Note: There are some Rust methods with unspecified precision without a `libm`
// equivalent:
// - `f32::powi` (integer powers)
// - `f32::log` (logarithm with specified base)
// - `f32::abs_sub` (actually unsure if `libm` has this, but don't use it regardless)
//
// Additionally, the following nightly API functions are not presently integrated
// into this, but they would be candidates once standardized:
// - `f32::gamma`
// - `f32::ln_gamma`

#[expect(
    clippy::disallowed_methods,
    reason = "Many of the disallowed methods are disallowed to force code to use the feature-conditional re-exports from this module, but this module itself is exempt from that rule."
)]
mod std_ops {

    /// Raises a number to a floating point power.
    ///
    /// Precision is specified when the `libm` feature is enabled.
    #[inline]
    pub fn powf(x: f32, y: f32) -> f32 {
        f32::powf(x, y)
    }

    /// Returns `2^(self)`.
    ///
    /// Precision is specified when the `libm` feature is enabled.
    #[inline]
    pub fn exp2(x: f32) -> f32 {
        f32::exp2(x)
    }

    /// Returns the natural logarithm of the number.
    ///
    /// Precision is specified when the `libm` feature is enabled.
    #[inline]
    pub fn ln(x: f32) -> f32 {
        f32::ln(x)
    }

    /// Computes the sine of a number (in radians).
    ///
    /// Precision is specified when the `libm` feature is enabled.
    #[inline]
    pub fn sin(x: f32) -> f32 {
        f32::sin(x)
    }

    /// Computes the cosine of a number (in radians).
    ///
    /// Precision is specified when the `libm` feature is enabled.
    #[inline]
    pub fn cos(x: f32) -> f32 {
        f32::cos(x)
    }

    /// Computes the tangent of a number (in radians).
    ///
    /// Precision is specified when the `libm` feature is enabled.
    #[inline]
    pub fn tan(x: f32) -> f32 {
        f32::tan(x)
    }

    /// Computes the arccosine of a number. Return value is in radians in
    /// the range [0, pi] or NaN if the number is outside the range
    /// [-1, 1].
    ///
    /// Precision is specified when the `libm` feature is enabled.
    #[inline]
    pub fn acos(x: f32) -> f32 {
        f32::acos(x)
    }

    /// Computes the four-quadrant arctangent of `y` and `x` in radians.
    ///
    /// * `x = 0`, `y = 0`: `0`
    /// * `x >= 0`: `arctan(y/x)` -> `[-pi/2, pi/2]`
    /// * `y >= 0`: `arctan(y/x) + pi` -> `(pi/2, pi]`
    /// * `y < 0`: `arctan(y/x) - pi` -> `(-pi, -pi/2)`
    ///
    /// Precision is specified when the `libm` feature is enabled.
    #[inline]
    pub fn atan2(y: f32, x: f32) -> f32 {
        f32::atan2(y, x)
    }

    /// Simultaneously computes the sine and cosine of the number, `x`. Returns
    /// `(sin(x), cos(x))`.
    ///
    /// Precision is specified when the `libm` feature is enabled.
    #[inline]
    pub fn sin_cos(x: f32) -> (f32, f32) {
        f32::sin_cos(x)
    }
}

#[expect(
    clippy::disallowed_methods,
    reason = "Many of the disallowed methods are disallowed to force code to use the feature-conditional re-exports from this module, but this module itself is exempt from that rule."
)]
mod std_ops_for_no_std {
    //! Provides standardized names for [`f32`] operations which may not be
    //! supported on `no_std` platforms.
    //! On `std` platforms, this forwards directly to the implementations provided
    //! by [`std`].

    /// Computes the absolute value of x.
    ///
    /// This function always returns the precise result.
    #[inline]
    pub fn abs(x: f32) -> f32 {
        f32::abs(x)
    }

    /// Returns the square root of a number.
    ///
    /// The result of this operation is guaranteed to be the rounded infinite-precision result.
    /// It is specified by IEEE 754 as `squareRoot` and guaranteed not to change.
    #[inline]
    pub fn sqrt(x: f32) -> f32 {
        f32::sqrt(x)
    }

    /// Returns a number composed of the magnitude of `x` and the sign of `y`.
    ///
    /// Equal to `x` if the sign of `x` and `y` are the same, otherwise equal to `-x`. If `x` is a
    /// `NaN`, then a `NaN` with the sign bit of `y` is returned. Note, however, that conserving the
    /// sign bit on `NaN` across arithmetical operations is not generally guaranteed.
    #[inline]
    pub fn copysign(x: f32, y: f32) -> f32 {
        f32::copysign(x, y)
    }

    /// Returns the nearest integer to `x`. If a value is half-way between two integers, round away from `0.0`.
    ///
    /// This function always returns the precise result.
    #[inline]
    pub fn round(x: f32) -> f32 {
        f32::round(x)
    }
}

pub use std_ops::*;

pub use std_ops_for_no_std::*;

/// This extension trait covers shortfall in determinacy from the lack of a `libm` counterpart
/// to `f32::powi`. Use this for the common small exponents.
pub trait FloatPow {
    /// Cubes the f32
    fn cubed(self) -> Self;
}

impl FloatPow for f32 {
    #[inline]
    fn cubed(self) -> Self {
        self * self * self
    }
}
