#![expect(missing_docs, reason = "Not all docs are written yet, see #3492.")]

extern crate alloc;
extern crate core;

mod components;
mod conversions;
mod index;
mod mesh;
#[cfg(feature = "morph")]
pub mod morph;
pub mod primitives;
pub mod skinning;
mod vertex;
use bevy_app::{App, Plugin, PostUpdate};
use bevy_asset::{AssetApp, AssetEventSystems};
use bevy_ecs::schedule::{IntoScheduleConfigs, SystemSet};
use bitflags::bitflags;
pub use components::*;
pub use index::*;
pub use mesh::*;
pub use primitives::*;
pub use vertex::*;
pub use wgpu_types::VertexFormat;

/// The mesh prelude.
///
/// This includes the most common types in this crate, re-exported for your convenience.
pub mod prelude {
    #[cfg(feature = "morph")]
    pub use crate::morph::MorphWeights;
    #[doc(hidden)]
    pub use crate::{primitives::MeshBuilder, primitives::Meshable, Mesh, Mesh2d, Mesh3d};
}

bitflags! {
    /// Our base mesh pipeline key bits start from the highest bit and go
    /// downward. The PBR mesh pipeline key bits start from the lowest bit and
    /// go upward. This allows the PBR bits in the downstream crate `bevy_pbr`
    /// to coexist in the same field without any shifts.
    #[derive(Clone, Debug)]
    pub struct BaseMeshPipelineKey: u64 {
        const MORPH_TARGETS = 1 << (u64::BITS - 1);
    }
}

/// Adds [`Mesh`] as an asset.
#[derive(Default)]
pub struct MeshPlugin;

impl Plugin for MeshPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<Mesh>()
            .init_asset::<skinning::SkinnedMeshInverseBindposes>()
            .add_systems(
                PostUpdate,
                mark_3d_meshes_as_changed_if_their_assets_changed.after(AssetEventSystems),
            );
    }
}

impl BaseMeshPipelineKey {
    pub const PRIMITIVE_TOPOLOGY_MASK_BITS: u64 = 0b111;
    pub const PRIMITIVE_TOPOLOGY_SHIFT_BITS: u64 =
        (u64::BITS - 1 - Self::PRIMITIVE_TOPOLOGY_MASK_BITS.count_ones()) as u64;
}

/// `bevy_render::mesh::inherit_weights` runs in this `SystemSet`
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub struct InheritWeightSystems;
