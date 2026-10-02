//! This module defines primitive shapes.
//! The origin is (0, 0) for 2D primitives and (0, 0, 0) for 3D primitives,
//! unless stated otherwise.

mod dim2;
pub use dim2::*;
mod dim3;
pub use dim3::*;
