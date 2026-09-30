use crate::{extract_component::ExtractComponentPlugin, storage::ShaderStorageBuffer};
use bevy_app::{App, Plugin};
use bevy_asset::Handle;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{entity::Entity, event::EntityEvent, prelude::Component};
use bevy_image::Image;
use bevy_reflect::Reflect;
use bevy_render_macros::ExtractComponent;
use encase::internal::ReadFrom;
use encase::private::Reader;
use encase::ShaderType;

/// A plugin that enables reading back gpu buffers and textures to the cpu.
#[derive(Default)]
pub struct GpuReadbackPlugin;

impl Plugin for GpuReadbackPlugin {
    fn build(&self, app: &mut App) {
        // The buffer pool, the copies and the mapping only reached the RenderApp.
        app.add_plugins(ExtractComponentPlugin::<Readback>::default());
    }
}

/// A component that registers the wrapped handle for gpu readback, either a texture or a buffer.
///
/// Data is read asynchronously and will be triggered on the entity via the [`ReadbackComplete`] event
/// when complete. If this component is not removed, the readback will be attempted every frame
#[derive(Component, ExtractComponent, Clone, Debug)]
pub enum Readback {
    Texture(Handle<Image>),
    Buffer {
        buffer: Handle<ShaderStorageBuffer>,
        start_offset_and_size: Option<(u64, u64)>,
    },
}

impl Readback {
    /// Create a readback component for a texture using the given handle.
    pub fn texture(image: Handle<Image>) -> Self {
        Self::Texture(image)
    }

    /// Create a readback component for a full buffer using the given handle.
    pub fn buffer(buffer: Handle<ShaderStorageBuffer>) -> Self {
        Self::Buffer {
            buffer,
            start_offset_and_size: None,
        }
    }

    /// Create a readback component for a buffer range using the given handle, a start offset in bytes
    /// and a number of bytes to read.
    pub fn buffer_range(buffer: Handle<ShaderStorageBuffer>, start_offset: u64, size: u64) -> Self {
        Self::Buffer {
            buffer,
            start_offset_and_size: Some((start_offset, size)),
        }
    }
}

/// An event that is triggered when a gpu readback is complete.
///
/// The event contains the data as a `Vec<u8>`, which can be interpreted as the raw bytes of the
/// requested buffer or texture.
#[derive(EntityEvent, Deref, DerefMut, Reflect, Debug)]
#[reflect(Debug)]
pub struct ReadbackComplete {
    pub entity: Entity,
    #[deref]
    pub data: Vec<u8>,
}

impl ReadbackComplete {
    /// Convert the raw bytes of the event to a shader type.
    pub fn to_shader_type<T: ShaderType + ReadFrom + Default>(&self) -> T {
        let mut val = T::default();
        let mut reader = Reader::new::<T>(&self.data, 0).expect("Failed to create Reader");
        T::read_from(&mut val, &mut reader);
        val
    }
}
