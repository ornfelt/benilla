use bevy::prelude::*;

/// A feature ID indicating the type of a geometric feature: a vertex, an edge, or (in 3D) a face.
///
/// This type packs the feature type into the same value as the feature index,
/// which indicates the specific vertex/edge/face that this ID belongs to.
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, Reflect)]
#[reflect(Debug, Hash, PartialEq)]
pub struct PackedFeatureId(pub u32);

impl PackedFeatureId {
    /// Packed feature id identifying an unknown feature.
    pub const UNKNOWN: Self = Self(0);
}

impl From<u32> for PackedFeatureId {
    fn from(code: u32) -> Self {
        Self(code)
    }
}

impl From<crate::parry::shape::PackedFeatureId> for PackedFeatureId {
    fn from(id: crate::parry::shape::PackedFeatureId) -> Self {
        Self(id.0)
    }
}
