use bevy_app::{App, Plugin};
use bevy_asset::Handle;
use bevy_ecs::prelude::Component;
use bevy_image::Image;
use bevy_math::Quat;
use bevy_render::sync_component::SyncComponentPlugin;

pub struct SkyboxPlugin;

impl Plugin for SkyboxPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(SyncComponentPlugin::<Skybox>::default());
    }
}

/// Adds a skybox to a 3D camera, based on a cubemap texture.
///
/// Note that this component does not (currently) affect the scene's lighting.
/// To do so, use `EnvironmentMapLight` alongside this component.
///
/// See also <https://en.wikipedia.org/wiki/Skybox_(video_games)>.
#[derive(Component, Clone)]
pub struct Skybox {
    pub image: Handle<Image>,
    /// Scale factor applied to the skybox image.
    /// After applying this multiplier to the image samples, the resulting values should
    /// be in units of [cd/m^2](https://en.wikipedia.org/wiki/Candela_per_square_metre).
    pub brightness: f32,

    /// View space rotation applied to the skybox cubemap.
    /// This is useful for users who require a different axis, such as the Z-axis, to serve
    /// as the vertical axis.
    pub rotation: Quat,
}

impl Default for Skybox {
    fn default() -> Self {
        Skybox {
            image: Handle::default(),
            brightness: 0.0,
            rotation: Quat::IDENTITY,
        }
    }
}
