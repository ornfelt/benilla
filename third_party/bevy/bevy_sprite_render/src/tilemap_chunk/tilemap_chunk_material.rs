use crate::{AlphaMode2d, Material2d, Material2dPlugin};
use bevy_app::{App, Plugin};
use bevy_asset::{embedded_path, Asset, AssetPath, Handle};
use bevy_image::Image;
use bevy_reflect::prelude::*;
use bevy_render::render_resource::*;
use bevy_shader::ShaderRef;

/// Plugin that adds support for tilemap chunk materials.
pub struct TilemapChunkMaterialPlugin;

impl Plugin for TilemapChunkMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(Material2dPlugin::<TilemapChunkMaterial>::default());
    }
}

/// Material used for rendering tilemap chunks.
///
/// The tilemap system rendered each chunk of tiles with it, in a single draw call per chunk.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct TilemapChunkMaterial {
    pub alpha_mode: AlphaMode2d,

    #[texture(0, dimension = "2d_array")]
    #[sampler(1)]
    pub tileset: Handle<Image>,

    #[texture(2, sample_type = "u_int")]
    pub tile_data: Handle<Image>,
}

impl Material2d for TilemapChunkMaterial {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Path(
            AssetPath::from_path_buf(embedded_path!("tilemap_chunk_material.wgsl"))
                .with_source("embedded"),
        )
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        self.alpha_mode
    }
}
