use crate::{Alpha, LinearRgba, Srgba};
use derive_more::derive::From;

/// An enumerated type that can represent an sRGB or a linear RGB color.
///
/// This is useful when you need to store a color in a data structure that can't be generic over
/// the color type.
#[doc = include_str!("../docs/conversion.md")]
#[derive(Debug, Clone, Copy, PartialEq, From)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub enum Color {
    /// A color in the sRGB color space with alpha.
    Srgba(Srgba),
    /// A color in the linear sRGB color space with alpha.
    LinearRgba(LinearRgba),
}

impl Color {
    /// Return the color as a linear RGBA color.
    pub fn to_linear(&self) -> LinearRgba {
        (*self).into()
    }

    /// Return the color as an SRGBA color.
    pub fn to_srgba(&self) -> Srgba {
        (*self).into()
    }

    /// Creates a new [`Color`] object storing a [`Srgba`] color.
    ///
    /// # Arguments
    ///
    /// * `red` - Red channel. [0.0, 1.0]
    /// * `green` - Green channel. [0.0, 1.0]
    /// * `blue` - Blue channel. [0.0, 1.0]
    /// * `alpha` - Alpha channel. [0.0, 1.0]
    pub const fn srgba(red: f32, green: f32, blue: f32, alpha: f32) -> Self {
        Self::Srgba(Srgba {
            red,
            green,
            blue,
            alpha,
        })
    }

    /// Creates a new [`Color`] object storing a [`Srgba`] color with an alpha of 1.0.
    ///
    /// # Arguments
    ///
    /// * `red` - Red channel. [0.0, 1.0]
    /// * `green` - Green channel. [0.0, 1.0]
    /// * `blue` - Blue channel. [0.0, 1.0]
    pub const fn srgb(red: f32, green: f32, blue: f32) -> Self {
        Self::Srgba(Srgba {
            red,
            green,
            blue,
            alpha: 1.0,
        })
    }

    /// Creates a new [`Color`] object storing a [`Srgba`] color from [`u8`] values.
    ///
    /// # Arguments
    ///
    /// * `red` - Red channel. [0, 255]
    /// * `green` - Green channel. [0, 255]
    /// * `blue` - Blue channel. [0, 255]
    /// * `alpha` - Alpha channel. [0, 255]
    pub const fn srgba_u8(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        Self::Srgba(Srgba {
            red: red as f32 / 255.0,
            green: green as f32 / 255.0,
            blue: blue as f32 / 255.0,
            alpha: alpha as f32 / 255.0,
        })
    }

    /// Creates a new [`Color`] object storing a [`Srgba`] color from [`u8`] values with an alpha of 1.0.
    ///
    /// # Arguments
    ///
    /// * `red` - Red channel. [0, 255]
    /// * `green` - Green channel. [0, 255]
    /// * `blue` - Blue channel. [0, 255]
    pub const fn srgb_u8(red: u8, green: u8, blue: u8) -> Self {
        Self::Srgba(Srgba {
            red: red as f32 / 255.0,
            green: green as f32 / 255.0,
            blue: blue as f32 / 255.0,
            alpha: 1.0,
        })
    }

    /// Creates a new [`Color`] object storing a [`LinearRgba`] color.
    ///
    /// # Arguments
    ///
    /// * `red` - Red channel. [0.0, 1.0]
    /// * `green` - Green channel. [0.0, 1.0]
    /// * `blue` - Blue channel. [0.0, 1.0]
    /// * `alpha` - Alpha channel. [0.0, 1.0]
    pub const fn linear_rgba(red: f32, green: f32, blue: f32, alpha: f32) -> Self {
        Self::LinearRgba(LinearRgba {
            red,
            green,
            blue,
            alpha,
        })
    }

    /// Creates a new [`Color`] object storing a [`LinearRgba`] color with an alpha of 1.0.
    ///
    /// # Arguments
    ///
    /// * `red` - Red channel. [0.0, 1.0]
    /// * `green` - Green channel. [0.0, 1.0]
    /// * `blue` - Blue channel. [0.0, 1.0]
    pub const fn linear_rgb(red: f32, green: f32, blue: f32) -> Self {
        Self::LinearRgba(LinearRgba {
            red,
            green,
            blue,
            alpha: 1.0,
        })
    }

    /// A fully white [`Color::LinearRgba`] color with an alpha of 1.0.
    pub const WHITE: Self = Self::linear_rgb(1.0, 1.0, 1.0);

    /// A fully black [`Color::LinearRgba`] color with an alpha of 1.0.
    pub const BLACK: Self = Self::linear_rgb(0., 0., 0.);

    /// A fully transparent [`Color::LinearRgba`] color with 0 red, green and blue.
    pub const NONE: Self = Self::linear_rgba(0., 0., 0., 0.);
}

impl Default for Color {
    /// A fully white [`Color::LinearRgba`] color with an alpha of 1.0.
    fn default() -> Self {
        Color::WHITE
    }
}

impl Alpha for Color {
    fn with_alpha(&self, alpha: f32) -> Self {
        let mut new = *self;

        match &mut new {
            Color::Srgba(x) => *x = x.with_alpha(alpha),
            Color::LinearRgba(x) => *x = x.with_alpha(alpha),
        }

        new
    }

    fn alpha(&self) -> f32 {
        match self {
            Color::Srgba(x) => x.alpha(),
            Color::LinearRgba(x) => x.alpha(),
        }
    }

    fn set_alpha(&mut self, alpha: f32) {
        match self {
            Color::Srgba(x) => x.set_alpha(alpha),
            Color::LinearRgba(x) => x.set_alpha(alpha),
        }
    }
}

impl From<Color> for Srgba {
    fn from(value: Color) -> Self {
        match value {
            Color::Srgba(srgba) => srgba,
            Color::LinearRgba(linear) => linear.into(),
        }
    }
}

impl From<Color> for LinearRgba {
    fn from(value: Color) -> Self {
        match value {
            Color::Srgba(srgba) => srgba.into(),
            Color::LinearRgba(linear) => linear,
        }
    }
}
