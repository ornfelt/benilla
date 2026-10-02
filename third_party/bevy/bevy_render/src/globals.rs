use bevy_app::{App, Plugin};

pub struct GlobalsPlugin;

impl Plugin for GlobalsPlugin {
    fn build(&self, _app: &mut App) {
        // The globals buffer and its extraction only reached the RenderApp.
    }
}
