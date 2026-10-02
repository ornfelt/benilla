//! Traits and type for interpolating between values.

use bevy_math::*;
use bevy_reflect::Reflect;

/// An animatable value type.
pub trait Animatable: Reflect + Sized + Send + Sync + 'static {
    /// Interpolates between `a` and `b` with an interpolation factor of `time`.
    ///
    /// The `time` parameter here may not be clamped to the range `[0.0, 1.0]`.
    fn interpolate(a: &Self, b: &Self, time: f32) -> Self;
}

impl Animatable for Vec3 {
    #[inline]
    fn interpolate(a: &Self, b: &Self, t: f32) -> Self {
        (*a) * (1.0 - t) + (*b) * t
    }
}

impl Animatable for Quat {
    /// Performs a slerp to smoothly interpolate between quaternions.
    #[inline]
    fn interpolate(a: &Self, b: &Self, t: f32) -> Self {
        // We want to smoothly interpolate between the two quaternions by default,
        // rather than using a quicker but less correct linear interpolation.
        a.slerp(*b, t)
    }
}
