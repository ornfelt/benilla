use crate::{
    render_asset::RenderAsset,
    render_resource::{Buffer, BufferUsages},
    wgpu,
};
use bevy_app::{App, Plugin};
use bevy_asset::{Asset, AssetApp, RenderAssetUsages};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_utils::default;
use encase::{internal::WriteInto, ShaderType};

/// Adds [`ShaderStorageBuffer`] as an asset that is extracted and uploaded to the GPU.
#[derive(Default)]
pub struct StoragePlugin;

impl Plugin for StoragePlugin {
    fn build(&self, app: &mut App) {
        // `RenderAssetPlugin::<GpuShaderStorageBuffer>` only reached the RenderApp.
        app.init_asset::<ShaderStorageBuffer>()
            .register_asset_reflect::<ShaderStorageBuffer>();
    }
}

/// A storage buffer that is prepared as a [`RenderAsset`] and uploaded to the GPU.
#[derive(Asset, Reflect, Debug, Clone)]
#[reflect(opaque)]
#[reflect(Default, Debug, Clone)]
pub struct ShaderStorageBuffer {
    /// Optional data used to initialize the buffer.
    pub data: Option<Vec<u8>>,
    /// The buffer description used to create the buffer.
    pub buffer_description: wgpu::BufferDescriptor<'static>,
    /// The asset usage of the storage buffer.
    pub asset_usage: RenderAssetUsages,
}

impl Default for ShaderStorageBuffer {
    fn default() -> Self {
        Self {
            data: None,
            buffer_description: wgpu::BufferDescriptor {
                label: None,
                size: 0,
                usage: BufferUsages::STORAGE,
                mapped_at_creation: false,
            },
            asset_usage: RenderAssetUsages::default(),
        }
    }
}

impl ShaderStorageBuffer {
    /// Creates a new storage buffer with the given data and asset usage.
    pub fn new(data: &[u8], asset_usage: RenderAssetUsages) -> Self {
        let mut storage = ShaderStorageBuffer {
            data: Some(data.to_vec()),
            ..default()
        };
        storage.asset_usage = asset_usage;
        storage
    }

    /// Creates a new storage buffer with the given size and asset usage.
    pub fn with_size(size: usize, asset_usage: RenderAssetUsages) -> Self {
        let mut storage = ShaderStorageBuffer {
            data: None,
            ..default()
        };
        storage.buffer_description.size = size as u64;
        storage.buffer_description.mapped_at_creation = false;
        storage.asset_usage = asset_usage;
        storage
    }

    /// Sets the data of the storage buffer to the given [`ShaderType`].
    pub fn set_data<T>(&mut self, value: T)
    where
        T: ShaderType + WriteInto,
    {
        let size = value.size().get() as usize;
        let mut wrapper = encase::StorageBuffer::<Vec<u8>>::new(Vec::with_capacity(size));
        wrapper.write(&value).unwrap();
        self.data = Some(wrapper.into_inner());
    }
}

impl<T> From<T> for ShaderStorageBuffer
where
    T: ShaderType + WriteInto,
{
    fn from(value: T) -> Self {
        let size = value.size().get() as usize;
        let mut wrapper = encase::StorageBuffer::<Vec<u8>>::new(Vec::with_capacity(size));
        wrapper.write(&value).unwrap();
        Self::new(wrapper.as_ref(), RenderAssetUsages::default())
    }
}

/// A storage buffer that is prepared as a [`RenderAsset`] and uploaded to the GPU.
pub struct GpuShaderStorageBuffer {
    pub buffer: Buffer,
    pub had_data: bool,
}

impl RenderAsset for GpuShaderStorageBuffer {
    type SourceAsset = ShaderStorageBuffer;
}
