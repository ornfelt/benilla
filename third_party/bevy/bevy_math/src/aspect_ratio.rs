//! Provides a simple aspect ratio struct to help with calculations.

use crate::Vec2;
use derive_more::derive::Into;
use thiserror::Error;

/// An `AspectRatio` is the ratio of width to height.
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Into)]
pub struct AspectRatio(f32);

impl AspectRatio {
    /// Attempts to create a new [`AspectRatio`] from a given width and height.
    ///
    /// # Errors
    ///
    /// Returns an `Err` with `AspectRatioError` if:
    /// - Either width or height is zero (`AspectRatioError::Zero`)
    /// - Either width or height is infinite (`AspectRatioError::Infinite`)
    /// - Either width or height is NaN (`AspectRatioError::NaN`)
    #[inline]
    pub const fn try_new(width: f32, height: f32) -> Result<Self, AspectRatioError> {
        match (width, height) {
            (w, h) if w == 0.0 || h == 0.0 => Err(AspectRatioError::Zero),
            (w, h) if w.is_infinite() || h.is_infinite() => Err(AspectRatioError::Infinite),
            (w, h) if w.is_nan() || h.is_nan() => Err(AspectRatioError::NaN),
            _ => Ok(Self(width / height)),
        }
    }

    /// Attempts to create a new [`AspectRatio`] from a given amount of x pixels and y pixels.
    #[inline]
    pub const fn try_from_pixels(x: u32, y: u32) -> Result<Self, AspectRatioError> {
        Self::try_new(x as f32, y as f32)
    }

    /// Returns the aspect ratio as a f32 value.
    #[inline]
    pub const fn ratio(&self) -> f32 {
        self.0
    }
}

impl TryFrom<Vec2> for AspectRatio {
    type Error = AspectRatioError;

    #[inline]
    fn try_from(value: Vec2) -> Result<Self, Self::Error> {
        Self::try_new(value.x, value.y)
    }
}

/// An Error type for when [`AspectRatio`](`super::AspectRatio`) is provided invalid width or height values
#[derive(Error, Debug, PartialEq, Eq, Clone, Copy)]
pub enum AspectRatioError {
    /// Error due to width or height having zero as a value.
    #[error("AspectRatio error: width or height is zero")]
    Zero,
    /// Error due towidth or height being infinite.
    #[error("AspectRatio error: width or height is infinite")]
    Infinite,
    /// Error due to width or height being Not a Number (NaN).
    #[error("AspectRatio error: width or height is NaN")]
    NaN,
}
