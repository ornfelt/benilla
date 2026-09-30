//! Order Independent Transparency (OIT) for 3d rendering. See [`OrderIndependentTransparencyPlugin`] for more details.

use bevy_app::prelude::*;
use bevy_camera::{Camera3d, RenderTarget};
use bevy_ecs::{component::*, lifecycle::ComponentHook, prelude::*};
use bevy_platform::collections::HashSet;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::{
    extract_component::{ExtractComponent, ExtractComponentPlugin},
    render_resource::{ShaderType, TextureUsages},
    view::Msaa,
};
use bevy_window::PrimaryWindow;
use tracing::warn;

/// Used to identify which camera will use OIT to render transparent meshes
/// and to configure OIT.
// TODO consider supporting multiple OIT techniques like WBOIT, Moment Based OIT,
// depth peeling, stochastic transparency, ray tracing etc.
// This should probably be done by adding an enum to this component.
// We use the same struct to pass on the settings to the drawing shader.
#[derive(Clone, Copy, ExtractComponent, Reflect, ShaderType)]
#[reflect(Clone, Default)]
pub struct OrderIndependentTransparencySettings {
    /// Controls how many layers will be used to compute the blending.
    /// The more layers you use the more memory it will use but it will also give better results.
    /// 8 is generally recommended, going above 32 is probably not worth it in the vast majority of cases
    pub layer_count: i32,
    /// Threshold for which fragments will be added to the blending layers.
    /// This can be tweaked to optimize quality / layers count. Higher values will
    /// allow lower number of layers and a better performance, compromising quality.
    pub alpha_threshold: f32,
}

impl Default for OrderIndependentTransparencySettings {
    fn default() -> Self {
        Self {
            layer_count: 8,
            alpha_threshold: 0.0,
        }
    }
}

// OrderIndependentTransparencySettings is also a Component. We explicitly implement the trait so
// we can hook on_add to issue a warning in case `layer_count` is seemingly too high.
impl Component for OrderIndependentTransparencySettings {
    const STORAGE_TYPE: StorageType = StorageType::SparseSet;
    type Mutability = Mutable;

    fn on_add() -> Option<ComponentHook> {
        Some(|world, context| {
            if let Some(value) = world.get::<OrderIndependentTransparencySettings>(context.entity)
                && value.layer_count > 32
            {
                warn!("{}OrderIndependentTransparencySettings layer_count set to {} might be too high.",
                        context.caller.map(|location|format!("{location}: ")).unwrap_or_default(),
                        value.layer_count
                    );
            }
        })
    }
}

/// A plugin that adds support for Order Independent Transparency (OIT).
/// This can correctly render some scenes that would otherwise have artifacts due to alpha blending, but uses more memory.
///
/// To enable OIT for a camera you need to add the [`OrderIndependentTransparencySettings`] component to it.
///
/// If you want to use OIT for your custom material you need to call `oit_draw(position, color)` in your fragment shader.
/// You also need to make sure that your fragment shader doesn't output any colors.
///
/// # Implementation details
/// This implementation uses 2 passes.
///
/// The first pass writes the depth and color of all the fragments to a big buffer.
/// The buffer contains N layers for each pixel, where N can be set with [`OrderIndependentTransparencySettings::layer_count`].
/// This pass is essentially a forward pass.
///
/// The second pass is a single fullscreen triangle pass that sorts all the fragments then blends them together
/// and outputs the result to the screen.
pub struct OrderIndependentTransparencyPlugin;
impl Plugin for OrderIndependentTransparencyPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractComponentPlugin::<OrderIndependentTransparencySettings>::default())
            .add_systems(Update, check_msaa)
            .add_systems(Last, configure_depth_texture_usages);
    }
}

// WARN This should only happen for cameras with the [`OrderIndependentTransparencySettings`] component
// but when multiple cameras are present on the same window
// bevy reuses the same depth texture so we need to set this on all cameras with the same render target.
fn configure_depth_texture_usages(
    p: Query<Entity, With<PrimaryWindow>>,
    cameras: Query<(&RenderTarget, Has<OrderIndependentTransparencySettings>)>,
    mut new_cameras: Query<(&mut Camera3d, &RenderTarget), Added<Camera3d>>,
) {
    if new_cameras.is_empty() {
        return;
    }

    // Find all the render target that potentially uses OIT
    let primary_window = p.single().ok();
    let mut render_target_has_oit = <HashSet<_>>::default();
    for (render_target, has_oit) in &cameras {
        if has_oit {
            render_target_has_oit.insert(render_target.normalize(primary_window));
        }
    }

    // Update the depth texture usage for cameras with a render target that has OIT
    for (mut camera_3d, render_target) in &mut new_cameras {
        if render_target_has_oit.contains(&render_target.normalize(primary_window)) {
            let mut usages = TextureUsages::from(camera_3d.depth_texture_usages);
            usages |= TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING;
            camera_3d.depth_texture_usages = usages.into();
        }
    }
}

fn check_msaa(cameras: Query<&Msaa, With<OrderIndependentTransparencySettings>>) {
    for msaa in &cameras {
        if msaa.samples() > 1 {
            panic!("MSAA is not supported when using OrderIndependentTransparency");
        }
    }
}
