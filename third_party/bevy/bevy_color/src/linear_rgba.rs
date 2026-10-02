use crate::{Alpha, ColorToComponents, ColorToPacked, Gray, Mix};
use bevy_math::{ops, Vec3, Vec4};
use bytemuck::{Pod, Zeroable};

/// Linear RGB color with alpha.
#[doc = include_str!("../docs/conversion.md")]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[repr(C)]
pub struct LinearRgba {
    /// The red channel. [0.0, 1.0]
    pub red: f32,
    /// The green channel. [0.0, 1.0]
    pub green: f32,
    /// The blue channel. [0.0, 1.0]
    pub blue: f32,
    /// The alpha channel. [0.0, 1.0]
    pub alpha: f32,
}

impl LinearRgba {
    /// A fully black color with full alpha.
    pub const BLACK: Self = Self {
        red: 0.0,
        green: 0.0,
        blue: 0.0,
        alpha: 1.0,
    };

    /// A fully white color with full alpha.
    pub const WHITE: Self = Self {
        red: 1.0,
        green: 1.0,
        blue: 1.0,
        alpha: 1.0,
    };

    /// A fully transparent color.
    pub const NONE: Self = Self {
        red: 0.0,
        green: 0.0,
        blue: 0.0,
        alpha: 0.0,
    };

    /// An invalid color.
    ///
    /// This type can be used to represent an invalid color value;
    /// in some rendering applications the color will be ignored,
    /// enabling performant hacks like hiding lines by setting their color to `INVALID`.
    pub const NAN: Self = Self {
        red: f32::NAN,
        green: f32::NAN,
        blue: f32::NAN,
        alpha: f32::NAN,
    };

    /// Construct a new [`LinearRgba`] color from components.
    pub const fn new(red: f32, green: f32, blue: f32, alpha: f32) -> Self {
        Self {
            red,
            green,
            blue,
            alpha,
        }
    }

    /// Construct a new [`LinearRgba`] color from (r, g, b) components, with the default alpha (1.0).
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

    /// Converts this color to a u32.
    ///
    /// Maps the RGBA channels in RGBA order to a little-endian byte array (GPUs are little-endian).
    /// `A` will be the most significant byte and `R` the least significant.
    pub fn as_u32(&self) -> u32 {
        u32::from_le_bytes(self.to_u8_array())
    }
}

impl Default for LinearRgba {
    /// Construct a new [`LinearRgba`] color with the default values (white with full alpha).
    fn default() -> Self {
        Self::WHITE
    }
}

impl Mix for LinearRgba {
    #[inline]
    fn mix(&self, other: &Self, factor: f32) -> Self {
        let n_factor = 1.0 - factor;
        Self {
            red: self.red * n_factor + other.red * factor,
            green: self.green * n_factor + other.green * factor,
            blue: self.blue * n_factor + other.blue * factor,
            alpha: self.alpha * n_factor + other.alpha * factor,
        }
    }
}

impl Gray for LinearRgba {
    const BLACK: Self = Self::BLACK;
    const WHITE: Self = Self::WHITE;
}

impl Alpha for LinearRgba {
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

impl ColorToComponents for LinearRgba {
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

impl ColorToPacked for LinearRgba {
    fn to_u8_array(self) -> [u8; 4] {
        [self.red, self.green, self.blue, self.alpha]
            .map(|v| ops::round(v.clamp(0.0, 1.0) * 255.0) as u8)
    }

    fn to_u8_array_no_alpha(self) -> [u8; 3] {
        [self.red, self.green, self.blue].map(|v| ops::round(v.clamp(0.0, 1.0) * 255.0) as u8)
    }

    fn from_u8_array(color: [u8; 4]) -> Self {
        Self::from_f32_array(color.map(|u| u as f32 / 255.0))
    }

    fn from_u8_array_no_alpha(color: [u8; 3]) -> Self {
        Self::from_f32_array_no_alpha(color.map(|u| u as f32 / 255.0))
    }
}

#[cfg(feature = "wgpu-types")]
impl From<LinearRgba> for wgpu_types::Color {
    fn from(color: LinearRgba) -> Self {
        wgpu_types::Color {
            r: color.red as f64,
            g: color.green as f64,
            b: color.blue as f64,
            a: color.alpha as f64,
        }
    }
}

// [`LinearRgba`] is intended to be used with shaders
// So it's the only color type that implements [`ShaderType`] to make it easier to use inside shaders
#[cfg(feature = "encase")]
impl encase::ShaderType for LinearRgba {
    type ExtraMetadata = ();

    const METADATA: encase::private::Metadata<Self::ExtraMetadata> = {
        let size =
            encase::private::SizeValue::from(<f32 as encase::private::ShaderSize>::SHADER_SIZE)
                .mul(4);
        let alignment = encase::private::AlignmentValue::from_next_power_of_two_size(size);

        encase::private::Metadata {
            alignment,
            has_uniform_min_alignment: false,
            is_pod: true,
            min_size: size,
            extra: (),
        }
    };

    const UNIFORM_COMPAT_ASSERT: fn() = || {};
}

#[cfg(feature = "encase")]
impl encase::private::WriteInto for LinearRgba {
    fn write_into<B: encase::private::BufferMut>(&self, writer: &mut encase::private::Writer<B>) {
        for el in &[self.red, self.green, self.blue, self.alpha] {
            encase::private::WriteInto::write_into(el, writer);
        }
    }
}

#[cfg(feature = "encase")]
impl encase::private::ReadFrom for LinearRgba {
    fn read_from<B: encase::private::BufferRef>(
        &mut self,
        reader: &mut encase::private::Reader<B>,
    ) {
        let mut buffer = [0.0f32; 4];
        for el in &mut buffer {
            encase::private::ReadFrom::read_from(el, reader);
        }

        *self = LinearRgba {
            red: buffer[0],
            green: buffer[1],
            blue: buffer[2],
            alpha: buffer[3],
        }
    }
}

#[cfg(feature = "encase")]
impl encase::private::CreateFrom for LinearRgba {
    fn create_from<B>(reader: &mut encase::private::Reader<B>) -> Self
    where
        B: encase::private::BufferRef,
    {
        // These are intentionally not inlined in the constructor to make this
        // resilient to internal Color refactors / implicit type changes.
        let red: f32 = encase::private::CreateFrom::create_from(reader);
        let green: f32 = encase::private::CreateFrom::create_from(reader);
        let blue: f32 = encase::private::CreateFrom::create_from(reader);
        let alpha: f32 = encase::private::CreateFrom::create_from(reader);
        LinearRgba {
            red,
            green,
            blue,
            alpha,
        }
    }
}

#[cfg(feature = "encase")]
impl encase::ShaderSize for LinearRgba {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_and_from_u8() {
        // from_u8_array
        let a = LinearRgba::from_u8_array([255, 0, 0, 255]);
        let b = LinearRgba::new(1.0, 0.0, 0.0, 1.0);
        assert_eq!(a, b);

        // from_u8_array_no_alpha
        let a = LinearRgba::from_u8_array_no_alpha([255, 255, 0]);
        let b = LinearRgba::rgb(1.0, 1.0, 0.0);
        assert_eq!(a, b);

        // to_u8_array
        let a = LinearRgba::new(0.0, 0.0, 1.0, 1.0).to_u8_array();
        let b = [0, 0, 255, 255];
        assert_eq!(a, b);

        // to_u8_array_no_alpha
        let a = LinearRgba::rgb(0.0, 1.0, 1.0).to_u8_array_no_alpha();
        let b = [0, 255, 255];
        assert_eq!(a, b);

        // clamping
        let a = LinearRgba::rgb(0.0, 100.0, -100.0).to_u8_array_no_alpha();
        let b = [0, 255, 0];
        assert_eq!(a, b);
    }
}
