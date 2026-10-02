#![cfg_attr(docsrs, feature(doc_cfg))]
#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://bevy.org/assets/icon.png",
    html_favicon_url = "https://bevy.org/assets/icon.png"
)]
#![no_std]

//! Representations of colors in various color spaces.
//!
//! This crate provides a number of color representations, including:
//!
//! - [`Srgba`] (standard RGBA, with gamma correction)
//! - [`LinearRgba`] (linear RGBA, without gamma correction)
//! - [`Xyza`] (x-axis, y-axis, z-axis, alpha)
//!
//! Each of these color spaces is represented as a distinct Rust type.
//!
//! # Color Space Usage
//!
//! Rendering engines typically use linear RGBA colors, which allow for physically accurate
//! lighting calculations. However, linear RGBA colors are not perceptually uniform, because
//! both human eyes and computer monitors have non-linear responses to light. "Standard" RGBA
//! represents an industry-wide compromise designed to encode colors in a way that looks good to
//! humans in as few bits as possible, but it is not suitable for lighting calculations.
//!
//! Most image file formats and scene graph formats use standard RGBA, because graphic design
//! tools are intended to be used by humans. However, 3D lighting calculations operate in linear
//! RGBA, so it is important to convert standard colors to linear before sending them to the GPU.
//! Most Bevy APIs will handle this conversion automatically, but if you are writing a custom
//! shader, you will need to do this conversion yourself.
//!
//! XYZ is a foundational space commonly used in the definition of other more modern color
//! spaces. The space is more formally known as CIE 1931, where the `x` and `z` axes represent
//! a form of chromaticity, while `y` defines an illuminance level.
//!
//! See also the [Wikipedia article on color spaces](https://en.wikipedia.org/wiki/Color_space).
#![doc = include_str!("../docs/conversion.md")]
//!
//! # Other Utilities
//!
//! The crate also provides a number of color operations, such as blending ([`Mix`]).
//!
//! In addition, there is a [`Color`] enum that can represent any of the color
//! types in this crate. This is useful when you need to store a color in a data structure
//! that can't be generic over the color type.
//!
//! # Example
//!
//! ```
//! use bevy_color::{Srgba, LinearRgba};
//!
//! let srgba = Srgba::new(0.5, 0.2, 0.8, 1.0);
//! let linear: LinearRgba = srgba.into();
//!
//! println!("Srgba: {:?}", srgba);
//! println!("LinearRgba: {:?}", linear);
//! ```

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "alloc")]
extern crate alloc;

mod color;
mod color_ops;
mod linear_rgba;
pub mod palettes;
mod srgba;
#[cfg(test)]
mod test_colors;
#[cfg(test)]
mod testing;
mod xyza;

/// The color prelude.
///
/// This includes the most common types in this crate, re-exported for your convenience.
pub mod prelude {
    pub use crate::{color::*, color_ops::*, linear_rgba::*, srgba::*, xyza::*};
}

pub use color::*;
pub use color_ops::*;
pub use linear_rgba::*;
pub use srgba::*;
pub use xyza::*;
