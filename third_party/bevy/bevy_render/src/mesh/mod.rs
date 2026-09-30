use bevy_app::{App, Plugin};
use bevy_ecs::prelude::*;
#[cfg(feature = "morph")]
use bevy_mesh::morph::{MeshMorphWeights, MorphWeights};
use bevy_mesh::*;

/// Makes sure that [`Mesh`]es are extracted and prepared for the GPU.
/// Does *not* add the [`Mesh`] as an asset. Use [`MeshPlugin`] for that.
pub struct MeshRenderAssetPlugin;

impl Plugin for MeshRenderAssetPlugin {
    fn build(&self, _app: &mut App) {
        // `RenderAssetPlugin::<RenderMesh, GpuImage>`, the mesh allocator and the vertex buffer
        // layouts only reached the RenderApp.
    }
}

/// [Inherit weights](inherit_weights) from glTF mesh parent entity to direct
/// bevy mesh child entities (ie: glTF primitive).
#[cfg(feature = "morph")]
pub struct MorphPlugin;
#[cfg(feature = "morph")]
impl Plugin for MorphPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            bevy_app::PostUpdate,
            inherit_weights.in_set(InheritWeightSystems),
        );
    }
}

/// Bevy meshes are gltf primitives, [`MorphWeights`] on the bevy node entity
/// should be inherited by children meshes.
///
/// Only direct children are updated, to fulfill the expectations of glTF spec.
#[cfg(feature = "morph")]
pub fn inherit_weights(
    morph_nodes: Query<(&Children, &MorphWeights), (Without<Mesh3d>, Changed<MorphWeights>)>,
    mut morph_primitives: Query<&mut MeshMorphWeights, With<Mesh3d>>,
) {
    for (children, parent_weights) in &morph_nodes {
        let mut iter = morph_primitives.iter_many_mut(children);
        while let Some(mut child_weight) = iter.fetch_next() {
            child_weight.clear_weights();
            child_weight.extend_weights(parent_weights.weights());
        }
    }
}
