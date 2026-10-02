use bevy_app::{App, Plugin};

/// A plugin that enables reading back gpu buffers and textures to the cpu.
#[derive(Default)]
pub struct GpuReadbackPlugin;

impl Plugin for GpuReadbackPlugin {
    fn build(&self, _app: &mut App) {
        // The buffer pool, the copies and the mapping only reached the RenderApp, and nothing
        // in the build constructs a `Readback` request.
    }
}
