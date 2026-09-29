//! Clustered decals, bounding regions that project textures onto surfaces.
//!
//! A *clustered decal* is a bounding box that projects a texture onto any
//! surface within its bounds along the positive Z axis. In Bevy, clustered
//! decals use the *clustered forward* rendering technique.
//!
//! Clustered decals are the highest-quality types of decals that Bevy supports,
//! but they require bindless textures. This means that they presently can't be
//! used on WebGL 2 or WebGPU. Bevy's clustered decals can be used
//! with forward or deferred rendering and don't require a prepass.
//!
//! Each clustered decal may contain up to 4 textures. By default, the 4
//! textures correspond to the base color, a normal map, a metallic-roughness
//! map, and an emissive map respectively. However, with a custom shader, you
//! can use these 4 textures for whatever you wish. Additionally, you can use
//! the built-in *tag* field to store additional application-specific data; by
//! reading the tag in the shader, you can modify the appearance of a clustered
//! decal arbitrarily. See the documentation in `clustered.wgsl` for more
//! information and the `clustered_decals` example for an example of use.

use bevy_app::{App, Plugin};
use bevy_light::ClusteredDecal;
use bevy_render::sync_component::SyncComponentPlugin;

/// A plugin that adds support for clustered decals.
///
/// In environments where bindless textures aren't available, clustered decals
/// can still be added to a scene, but they won't project any decals.
pub struct ClusteredDecalPlugin;

impl Plugin for ClusteredDecalPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(SyncComponentPlugin::<ClusteredDecal>::default());
    }
}
