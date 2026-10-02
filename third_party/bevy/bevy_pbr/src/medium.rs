use bevy_app::{App, Plugin};
use bevy_asset::{Asset, AssetApp};
use bevy_reflect::TypePath;

#[doc(hidden)]
pub struct ScatteringMediumPlugin;

impl Plugin for ScatteringMediumPlugin {
    fn build(&self, app: &mut App) {
        // `RenderAssetPlugin::<GpuScatteringMedium>` and the sampler only reached the RenderApp.
        app.init_asset::<ScatteringMedium>();
    }
}

/// An asset that defines how a material scatters light.
#[derive(TypePath, Asset, Clone)]
pub struct ScatteringMedium;
