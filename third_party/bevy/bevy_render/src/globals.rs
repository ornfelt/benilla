use crate::{extract_resource::ExtractResource, render_resource::ShaderType};
use bevy_app::{App, Plugin};
use bevy_ecs::prelude::*;
use bevy_reflect::prelude::*;

pub struct GlobalsPlugin;

impl Plugin for GlobalsPlugin {
    fn build(&self, _app: &mut App) {
        // The globals buffer and its extraction only reached the RenderApp.
    }
}

/// Contains global values useful when writing shaders.
/// Currently only contains values related to time.
#[derive(Default, Clone, Resource, ExtractResource, Reflect, ShaderType)]
#[reflect(Resource, Default, Clone)]
pub struct GlobalsUniform {
    /// The time since startup in seconds.
    /// Wraps to 0 after 1 hour.
    time: f32,
    /// The delta time since the previous frame in seconds
    delta_time: f32,
    /// Frame count since the start of the app.
    /// It wraps to zero when it reaches the maximum value of a u32.
    frame_count: u32,
}
