use crate::{extract_component::ExtractComponentPlugin, storage::ShaderStorageBuffer};
use bevy_app::{App, Plugin};
use bevy_asset::Handle;
use bevy_ecs::prelude::Component;
use bevy_image::Image;
use bevy_render_macros::ExtractComponent;

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
/// Data is read asynchronously and will be triggered on the entity via the `ReadbackComplete` event
/// when complete. If this component is not removed, the readback will be attempted every frame
#[derive(Component, ExtractComponent, Clone, Debug)]
pub enum Readback {
    Texture(Handle<Image>),
    Buffer {
        buffer: Handle<ShaderStorageBuffer>,
        start_offset_and_size: Option<(u64, u64)>,
    },
}
