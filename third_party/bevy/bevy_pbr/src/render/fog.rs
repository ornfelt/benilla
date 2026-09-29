use bevy_app::{App, Plugin};
use bevy_render::extract_component::ExtractComponentPlugin;

use crate::DistanceFog;

/// A plugin that consolidates fog extraction, preparation and related resources/assets
pub struct FogPlugin;

impl Plugin for FogPlugin {
    fn build(&self, app: &mut App) {
        // Fog's uniform buffer only reached the RenderApp.
        app.add_plugins(ExtractComponentPlugin::<DistanceFog>::default());
    }
}
