//! A unique key for a pair of identifiers.

use bevy::prelude::*;

/// A unique key for a pair of identifiers.
///
/// This can be used for efficient storage and lookup of pairs of entities or other objects.
#[derive(Clone, Copy, Debug, Deref, DerefMut, PartialEq, Eq, Hash)]
pub struct PairKey(pub u64);

impl PairKey {
    /// Creates a new pair key from two IDs.
    #[inline]
    pub const fn new(id1: u32, id2: u32) -> Self {
        if id1 < id2 {
            Self(((id1 as u64) << 32) | id2 as u64)
        } else {
            Self(((id2 as u64) << 32) | id1 as u64)
        }
    }
}
