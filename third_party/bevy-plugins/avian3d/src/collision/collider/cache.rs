use bevy::prelude::*;

/// Empty: it cached the colliders that the cut `ColliderConstructor` and `ColliderConstructorHierarchy`
/// built from meshes, and cleared unused entries in `PreUpdate`.
pub struct ColliderCachePlugin;

impl Plugin for ColliderCachePlugin {
    fn build(&self, _app: &mut App) {}
}
