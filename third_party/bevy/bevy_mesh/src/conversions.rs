//! These implementations allow you to
//! convert `std::vec::Vec<T>` to `VertexAttributeValues::T`.
//!
//! # Examples
//!
//! ```
//! use bevy_mesh::VertexAttributeValues;
//!
//! // creating std::vec::Vec
//! let buffer = vec![[0_f32; 4]; 10];
//!
//! // converting std::vec::Vec to bevy_mesh::VertexAttributeValues
//! let values = VertexAttributeValues::from(buffer.clone());
//!
//! assert_eq!(values, VertexAttributeValues::Float32x4(buffer));
//! ```

use super::VertexAttributeValues;
use bevy_math::Vec3;

macro_rules! impl_from {
    ($from:ty, $variant:tt) => {
        impl From<Vec<$from>> for VertexAttributeValues {
            fn from(vec: Vec<$from>) -> Self {
                VertexAttributeValues::$variant(vec)
            }
        }
    };
}

macro_rules! impl_from_into {
    ($from:ty, $variant:tt) => {
        impl From<Vec<$from>> for VertexAttributeValues {
            fn from(vec: Vec<$from>) -> Self {
                let vec: Vec<_> = vec.into_iter().map(|t| t.into()).collect();
                VertexAttributeValues::$variant(vec)
            }
        }
    };
}

impl_from!([f32; 2], Float32x2);
impl_from!([f32; 3], Float32x3);
impl_from_into!(Vec3, Float32x3);
impl_from!([f32; 4], Float32x4);

impl_from!(u32, Uint32);

#[cfg(test)]
mod tests {
    use bevy_math::Vec3;

    use super::VertexAttributeValues;

    #[test]
    fn u32() {
        let buffer = vec![0_u32; 10];
        let values = VertexAttributeValues::from(buffer.clone());
        assert_eq!(values, VertexAttributeValues::Uint32(buffer));
    }

    #[test]
    fn f32_2() {
        let buffer = vec![[0.0; 2]; 10];
        let values = VertexAttributeValues::from(buffer.clone());
        assert_eq!(values, VertexAttributeValues::Float32x2(buffer));
    }

    #[test]
    fn f32_3() {
        let buffer = vec![[0.0; 3]; 10];
        let values = VertexAttributeValues::from(buffer.clone());
        assert_eq!(values, VertexAttributeValues::Float32x3(buffer));
    }

    #[test]
    fn vec3() {
        let buffer = vec![Vec3::ZERO; 10];
        let values = VertexAttributeValues::from(buffer.clone());
        assert_eq!(
            values,
            VertexAttributeValues::Float32x3(buffer.iter().map(|v| v.to_array()).collect())
        );
    }

    #[test]
    fn f32_4() {
        let buffer = vec![[0.0; 4]; 10];
        let values = VertexAttributeValues::from(buffer.clone());
        assert_eq!(values, VertexAttributeValues::Float32x4(buffer));
    }
}
