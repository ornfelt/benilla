use crate::{Dir3, Vec3};

/// An infinite half-line starting at `origin` and going in `direction` in 3D space.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct Ray3d {
    /// The origin of the ray.
    pub origin: Vec3,
    /// The direction of the ray.
    pub direction: Dir3,
}

impl Ray3d {
    /// Creates a new `Ray3d` from a given origin and direction
    #[inline]
    pub const fn new(origin: Vec3, direction: Dir3) -> Self {
        Self { origin, direction }
    }
}
