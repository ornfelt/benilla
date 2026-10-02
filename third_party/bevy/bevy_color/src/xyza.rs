use crate::LinearRgba;

/// [CIE 1931](https://en.wikipedia.org/wiki/CIE_1931_color_space) color space, also known as XYZ, with an alpha channel.
#[doc = include_str!("../docs/conversion.md")]
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct Xyza {
    /// The x-axis. [0.0, 1.0]
    pub x: f32,
    /// The y-axis, intended to represent luminance. [0.0, 1.0]
    pub y: f32,
    /// The z-axis. [0.0, 1.0]
    pub z: f32,
    /// The alpha channel. [0.0, 1.0]
    pub alpha: f32,
}

impl Xyza {
    /// Construct a new [`Xyza`] color from components.
    ///
    /// # Arguments
    ///
    /// * `x` - x-axis. [0.0, 1.0]
    /// * `y` - y-axis. [0.0, 1.0]
    /// * `z` - z-axis. [0.0, 1.0]
    /// * `alpha` - Alpha channel. [0.0, 1.0]
    pub const fn new(x: f32, y: f32, z: f32, alpha: f32) -> Self {
        Self { x, y, z, alpha }
    }
}

impl From<LinearRgba> for Xyza {
    fn from(
        LinearRgba {
            red,
            green,
            blue,
            alpha,
        }: LinearRgba,
    ) -> Self {
        // Linear sRGB to XYZ
        // http://www.brucelindbloom.com/index.html?Eqn_XYZ_to_RGB.html
        // http://www.brucelindbloom.com/index.html?Eqn_RGB_XYZ_Matrix.html (sRGB, RGB to XYZ [M])
        let r = red;
        let g = green;
        let b = blue;

        let x = r * 0.4124564 + g * 0.3575761 + b * 0.1804375;
        let y = r * 0.2126729 + g * 0.7151522 + b * 0.072175;
        let z = r * 0.0193339 + g * 0.119192 + b * 0.9503041;

        Xyza::new(x, y, z, alpha)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{test_colors::TEST_COLORS, testing::assert_approx_eq};

    #[test]
    fn test_to_from_srgba_2() {
        for color in TEST_COLORS.iter() {
            let xyz2 = Xyza::from(LinearRgba::from(color.rgb));
            assert_approx_eq!(color.xyz.x, xyz2.x, 0.001);
            assert_approx_eq!(color.xyz.y, xyz2.y, 0.001);
            assert_approx_eq!(color.xyz.z, xyz2.z, 0.001);
            assert_approx_eq!(color.xyz.alpha, xyz2.alpha, 0.001);
        }
    }
}
