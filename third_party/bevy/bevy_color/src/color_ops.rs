use bevy_math::{Vec3, Vec4};

/// Linear interpolation of two colors within a given color space.
pub trait Mix: Sized {
    /// Linearly interpolate between this and another color, by factor.
    /// Factor should be between 0.0 and 1.0.
    fn mix(&self, other: &Self, factor: f32) -> Self;
}

/// Trait for returning a grayscale color of a provided lightness.
pub trait Gray: Mix + Sized {
    /// A pure black color.
    const BLACK: Self;
    /// A pure white color.
    const WHITE: Self;

    /// Returns a grey color with the provided lightness from (0.0 - 1.0). 0 is black, 1 is white.
    fn gray(lightness: f32) -> Self {
        Self::BLACK.mix(&Self::WHITE, lightness)
    }
}

/// Methods for manipulating alpha values.
pub trait Alpha: Sized {
    /// Return a new version of this color with the given alpha value.
    fn with_alpha(&self, alpha: f32) -> Self;

    /// Return the alpha component of this color.
    fn alpha(&self) -> f32;

    /// Sets the alpha component of this color.
    fn set_alpha(&mut self, alpha: f32);

    /// Is the alpha component of this color less than or equal to 0.0?
    fn is_fully_transparent(&self) -> bool {
        self.alpha() <= 0.0
    }
}

/// Trait with methods for converting colors to non-color types
pub trait ColorToComponents {
    /// Convert to an f32 array
    fn to_f32_array(self) -> [f32; 4];
    /// Convert to a Vec4
    fn to_vec4(self) -> Vec4;
    /// Convert to a Vec3
    fn to_vec3(self) -> Vec3;
    /// Convert from an f32 array
    fn from_f32_array(color: [f32; 4]) -> Self;
    /// Convert from an f32 array without the alpha value
    fn from_f32_array_no_alpha(color: [f32; 3]) -> Self;
}

/// Trait with methods for converting colors to packed non-color types
pub trait ColorToPacked {
    /// Convert to [u8; 4] where that makes sense (Srgba is most relevant)
    fn to_u8_array(self) -> [u8; 4];
    /// Convert to [u8; 3] where that makes sense (Srgba is most relevant)
    fn to_u8_array_no_alpha(self) -> [u8; 3];
    /// Convert from [u8; 4] where that makes sense (Srgba is most relevant)
    fn from_u8_array(color: [u8; 4]) -> Self;
    /// Convert to [u8; 3] where that makes sense (Srgba is most relevant)
    fn from_u8_array_no_alpha(color: [u8; 3]) -> Self;
}

#[cfg(test)]
mod tests {
    use core::fmt::Debug;

    use super::*;

    fn verify_gray<Col>()
    where
        Col: Gray + Debug + PartialEq,
    {
        assert_eq!(Col::gray(0.), Col::BLACK);
        assert_eq!(Col::gray(1.), Col::WHITE);
    }

    #[test]
    fn test_gray() {
        verify_gray::<crate::LinearRgba>();
    }
}
