#![expect(missing_docs, reason = "Not all docs are written yet, see #3492.")]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://bevy.org/assets/icon.png",
    html_favicon_url = "https://bevy.org/assets/icon.png"
)]

//! Provides 2D sprite rendering functionality.

extern crate alloc;

mod mesh2d;
mod texture_slice;
mod tilemap_chunk;

/// The sprite prelude.
///
/// This includes the most common types in this crate, re-exported for your convenience.
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{ColorMaterial, MeshMaterial2d};
}

pub use mesh2d::*;
pub(crate) use texture_slice::*;
pub use tilemap_chunk::*;

use bevy_app::prelude::*;
use bevy_asset::AssetEventSystems;
use bevy_ecs::prelude::*;
use bevy_image::{prelude::*, TextureAtlasPlugin};
use bevy_render::sync_world::SyncToRenderWorld;
use bevy_sprite::Sprite;

/// Adds support for 2D sprite rendering.
#[derive(Default)]
pub struct SpriteRenderPlugin;

/// System set for sprite rendering.
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum SpriteSystems {
    ExtractSprites,
    ComputeSlices,
}

impl Plugin for SpriteRenderPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<TextureAtlasPlugin>() {
            app.add_plugins(TextureAtlasPlugin);
        }

        app.add_plugins((
            ColorMaterialPlugin,
            TilemapChunkPlugin,
            TilemapChunkMaterialPlugin,
        ))
        .add_systems(
            PostUpdate,
            (
                compute_slices_on_asset_event.before(AssetEventSystems),
                compute_slices_on_sprite_change,
            )
                .in_set(SpriteSystems::ComputeSlices),
        );

        app.register_required_components::<Sprite, SyncToRenderWorld>();
    }
}
