use bevy_app::{App, Plugin, Update};

mod tilemap_chunk_material;

pub use tilemap_chunk_material::*;

/// Plugin that handles the updating of tilemap chunks.
pub struct TilemapChunkPlugin;

impl Plugin for TilemapChunkPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_tilemap_chunk_indices);
    }
}

/// Stand-in for `update_tilemap_chunk_indices`, which repacked the tile data image of each
/// tilemap chunk whose tile data changed. No entity has a `TilemapChunk`.
fn update_tilemap_chunk_indices() {}
