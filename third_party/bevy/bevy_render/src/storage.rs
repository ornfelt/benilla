use crate::{render_asset::RenderAsset, render_resource::Buffer};
use bevy_app::{App, Plugin};
use bevy_asset::{Asset, AssetApp};
use bevy_reflect::TypePath;

/// Adds [`ShaderStorageBuffer`] as an asset that is extracted and uploaded to the GPU.
#[derive(Default)]
pub struct StoragePlugin;

impl Plugin for StoragePlugin {
    fn build(&self, app: &mut App) {
        // `RenderAssetPlugin::<GpuShaderStorageBuffer>` only reached the RenderApp.
        app.init_asset::<ShaderStorageBuffer>();
    }
}

/// A storage buffer that is prepared as a [`RenderAsset`] and uploaded to the GPU.
#[derive(Asset, TypePath, Debug, Clone, Default)]
pub struct ShaderStorageBuffer;

/// A storage buffer that is prepared as a [`RenderAsset`] and uploaded to the GPU.
pub struct GpuShaderStorageBuffer {
    pub buffer: Buffer,
    pub had_data: bool,
}

impl RenderAsset for GpuShaderStorageBuffer {
    type SourceAsset = ShaderStorageBuffer;
}
