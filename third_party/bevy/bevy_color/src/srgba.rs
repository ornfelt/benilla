use crate::{Alpha, ColorToComponents, LinearRgba};
use bevy_math::{ops, Vec3, Vec4};
/// Non-linear standard RGB with alpha.
#[doc = include_str!("../docs/conversion.md")]
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct Srgba {
    /// The red channel. [0.0, 1.0]
    pub red: f32,
    /// The green channel. [0.0, 1.0]
    pub green: f32,
    /// The blue channel. [0.0, 1.0]
    pub blue: f32,
    /// The alpha channel. [0.0, 1.0]
    pub alpha: f32,
}

impl Srgba {
    // The standard VGA colors, with alpha set to 1.0.
    // https://en.wikipedia.org/wiki/Web_colors#Basic_colors
    /// <div style="background-color:rgb(100%, 100%, 100%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const WHITE: Srgba = Srgba::new(1.0, 1.0, 1.0, 1.0);

    /// Construct a new [`Srgba`] color from components.
    ///
    /// # Arguments
    ///
    /// * `red` - Red channel. [0.0, 1.0]
    /// * `green` - Green channel. [0.0, 1.0]
    /// * `blue` - Blue channel. [0.0, 1.0]
    /// * `alpha` - Alpha channel. [0.0, 1.0]
    pub const fn new(red: f32, green: f32, blue: f32, alpha: f32) -> Self {
        Self {
            red,
            green,
            blue,
            alpha,
        }
    }

    /// Construct a new [`Srgba`] color from (r, g, b) components, with the default alpha (1.0).
    ///
    /// # Arguments
    ///
    /// * `red` - Red channel. [0.0, 1.0]
    /// * `green` - Green channel. [0.0, 1.0]
    /// * `blue` - Blue channel. [0.0, 1.0]
    pub const fn rgb(red: f32, green: f32, blue: f32) -> Self {
        Self {
            red,
            green,
            blue,
            alpha: 1.0,
        }
    }

    /// Converts a non-linear sRGB value to a linear one via [gamma correction](https://en.wikipedia.org/wiki/Gamma_correction).
    pub fn gamma_function(value: f32) -> f32 {
        if value <= 0.0 {
            return value;
        }
        if value <= 0.04045 {
            value / 12.92 // linear falloff in dark values
        } else {
            ops::powf((value + 0.055) / 1.055, 2.4) // gamma curve in other area
        }
    }

    /// Converts a linear sRGB value to a non-linear one via [gamma correction](https://en.wikipedia.org/wiki/Gamma_correction).
    pub fn gamma_function_inverse(value: f32) -> f32 {
        if value <= 0.0 {
            return value;
        }

        if value <= 0.0031308 {
            value * 12.92 // linear falloff in dark values
        } else {
            (1.055 * ops::powf(value, 1.0 / 2.4)) - 0.055 // gamma curve in other area
        }
    }
}

impl Default for Srgba {
    fn default() -> Self {
        Self::WHITE
    }
}

impl Alpha for Srgba {
    #[inline]
    fn with_alpha(&self, alpha: f32) -> Self {
        Self { alpha, ..*self }
    }

    #[inline]
    fn alpha(&self) -> f32 {
        self.alpha
    }

    #[inline]
    fn set_alpha(&mut self, alpha: f32) {
        self.alpha = alpha;
    }
}

impl ColorToComponents for Srgba {
    fn to_f32_array(self) -> [f32; 4] {
        [self.red, self.green, self.blue, self.alpha]
    }

    fn to_vec4(self) -> Vec4 {
        Vec4::new(self.red, self.green, self.blue, self.alpha)
    }

    fn to_vec3(self) -> Vec3 {
        Vec3::new(self.red, self.green, self.blue)
    }

    fn from_f32_array(color: [f32; 4]) -> Self {
        Self {
            red: color[0],
            green: color[1],
            blue: color[2],
            alpha: color[3],
        }
    }

    fn from_f32_array_no_alpha(color: [f32; 3]) -> Self {
        Self {
            red: color[0],
            green: color[1],
            blue: color[2],
            alpha: 1.0,
        }
    }
}

impl From<LinearRgba> for Srgba {
    #[inline]
    fn from(value: LinearRgba) -> Self {
        Self {
            red: Srgba::gamma_function_inverse(value.red),
            green: Srgba::gamma_function_inverse(value.green),
            blue: Srgba::gamma_function_inverse(value.blue),
            alpha: value.alpha,
        }
    }
}

impl From<Srgba> for LinearRgba {
    #[inline]
    fn from(value: Srgba) -> Self {
        Self {
            red: Srgba::gamma_function(value.red),
            green: Srgba::gamma_function(value.green),
            blue: Srgba::gamma_function(value.blue),
            alpha: value.alpha,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::testing::assert_approx_eq;

    use super::*;

    #[test]
    fn test_to_from_linear() {
        let srgba = Srgba::new(0.0, 0.5, 1.0, 1.0);
        let linear_rgba: LinearRgba = srgba.into();
        assert_eq!(linear_rgba.red, 0.0);
        assert_approx_eq!(linear_rgba.green, 0.2140, 0.0001);
        assert_approx_eq!(linear_rgba.blue, 1.0, 0.0001);
        assert_eq!(linear_rgba.alpha, 1.0);
        let srgba2: Srgba = linear_rgba.into();
        assert_eq!(srgba2.red, 0.0);
        assert_approx_eq!(srgba2.green, 0.5, 0.0001);
        assert_approx_eq!(srgba2.blue, 1.0, 0.0001);
        assert_eq!(srgba2.alpha, 1.0);
    }
}
