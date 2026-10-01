use bevy::prelude::*;
use derive_more::From;

use crate::prelude::*;

/// Which translational and rotational axes of a [rigid body](RigidBody) are locked.
///
/// The axes are represented using a total of six bits, one for each axis. They can be set directly
/// with the [`from_bits`](Self::from_bits) and [`to_bits`](Self::to_bits) methods.
#[derive(Reflect, Clone, Copy, Debug, Default, From)]
#[reflect(Debug, Default)]
pub struct LockedAxes(u8);

impl LockedAxes {
    /// Creates a new [`LockedAxes`] configuration using bits.
    ///
    /// The first three bits correspond to translational axes, while the last three bits correspond to rotational
    /// axes. For example, `0b100_010` would lock translation along the `X` axis and rotation around the `Y` axis.
    pub const fn from_bits(bits: u8) -> Self {
        Self(bits)
    }

    /// Returns the locked axes as bits.
    ///
    /// The first three bits correspond to translational axes, while the last three bits correspond to rotational
    /// axes. For example, `0b100_010` would mean that translation along the `X` axis and rotation around the `Y` axis
    /// are locked.
    pub const fn to_bits(&self) -> u8 {
        self.0
    }

    /// Returns true if translation is locked along the `X` axis.
    pub const fn is_translation_x_locked(&self) -> bool {
        (self.0 & 0b100_000) != 0
    }

    /// Returns true if translation is locked along the `X` axis.
    pub const fn is_translation_y_locked(&self) -> bool {
        (self.0 & 0b010_000) != 0
    }

    /// Returns true if translation is locked along the `X` axis.
    pub const fn is_translation_z_locked(&self) -> bool {
        (self.0 & 0b001_000) != 0
    }

    /// Returns true if rotation is locked around the `X` axis.
    pub const fn is_rotation_x_locked(&self) -> bool {
        (self.0 & 0b000_100) != 0
    }

    /// Returns true if rotation is locked around the `Y` axis.
    pub const fn is_rotation_y_locked(&self) -> bool {
        (self.0 & 0b000_010) != 0
    }

    /// Returns true if rotation is locked around the `Z` axis.
    pub const fn is_rotation_z_locked(&self) -> bool {
        (self.0 & 0b000_001) != 0
    }

    /// Returns true if all rotation is locked.
    pub const fn is_rotation_locked(&self) -> bool {
        (self.0 & 0b000_111) == 0b000_111
    }

    /// Sets translational axes of the given vector to zero based on the [`LockedAxes`] configuration.
    pub(crate) fn apply_to_vec(&self, mut vector: Vector) -> Vector {
        if self.is_translation_x_locked() {
            vector.x = 0.0;
        }
        if self.is_translation_y_locked() {
            vector.y = 0.0;
        }
        if self.is_translation_z_locked() {
            vector.z = 0.0;
        }
        vector
    }
}
