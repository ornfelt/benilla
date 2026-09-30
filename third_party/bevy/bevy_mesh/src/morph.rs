use super::Mesh;
use bevy_asset::Handle;
use bevy_ecs::prelude::*;
use bevy_reflect::prelude::*;

/// Controls the [morph targets] for all child [`Mesh3d`](crate::Mesh3d) entities. In most cases, [`MorphWeights`] should be considered
/// the "source of truth" when writing morph targets for meshes. However you can choose to write child [`MeshMorphWeights`]
/// if your situation requires more granularity. Just note that if you set [`MorphWeights`], it will overwrite child
/// [`MeshMorphWeights`] values.
///
/// This exists because Bevy's [`Mesh`] corresponds to a _single_ surface / material, whereas morph targets
/// as defined in the GLTF spec exist on "multi-primitive meshes" (where each primitive is its own surface with its own material).
/// Therefore in Bevy [`MorphWeights`] an a parent entity are the "canonical weights" from a GLTF perspective, which then
/// synchronized to child [`Mesh3d`](crate::Mesh3d) / [`MeshMorphWeights`] (which correspond to "primitives" / "surfaces" from a GLTF perspective).
///
/// Add this to the parent of one or more [`Entities`](`Entity`) with a [`Mesh3d`](crate::Mesh3d) with a [`MeshMorphWeights`].
///
/// [morph targets]: https://en.wikipedia.org/wiki/Morph_target_animation
#[derive(Reflect, Default, Debug, Clone, Component)]
#[reflect(Debug, Component, Default, Clone)]
pub struct MorphWeights {
    weights: Vec<f32>,
    /// The first mesh primitive assigned to these weights
    first_mesh: Option<Handle<Mesh>>,
}

impl MorphWeights {
    pub fn weights(&self) -> &[f32] {
        &self.weights
    }
    pub fn weights_mut(&mut self) -> &mut [f32] {
        &mut self.weights
    }
}

/// Control a specific [`Mesh`] instance's [morph targets]. These control the weights of
/// specific "mesh primitives" in scene formats like GLTF. They can be set manually, but
/// in most cases they should "automatically" synced by setting the [`MorphWeights`] component
/// on a parent entity.
///
/// See [`MorphWeights`] for more details on Bevy's morph target implementation.
///
/// Add this to an [`Entity`] with a [`Mesh3d`](crate::Mesh3d) to control individual weights
/// of each morph target.
///
/// [morph targets]: https://en.wikipedia.org/wiki/Morph_target_animation
#[derive(Reflect, Default, Debug, Clone, Component)]
#[reflect(Debug, Component, Default, Clone)]
pub struct MeshMorphWeights {
    weights: Vec<f32>,
}

impl MeshMorphWeights {
    pub fn weights(&self) -> &[f32] {
        &self.weights
    }
    pub fn weights_mut(&mut self) -> &mut [f32] {
        &mut self.weights
    }
    pub fn clear_weights(&mut self) {
        self.weights.clear();
    }
    pub fn extend_weights(&mut self, weights: &[f32]) {
        self.weights.extend(weights);
    }
}
