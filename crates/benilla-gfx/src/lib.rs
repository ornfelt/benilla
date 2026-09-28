//! The gfx DLL backend, behind the `gfx` feature: the window, input and every draw go through the
//! gfx library (`Code2/General/gfx/gfx_dll`) instead of winit and wgpu. [`swap_in`] takes the
//! tuned `DefaultPlugins` and leaves out `WinitPlugin` and `RenderPlugin`, so neither opens a
//! window or a device; the ECS types the game builds on (`Mesh`, `Image`, materials, UI nodes)
//! stay, and the gfx renderer reads them from the main world.
//!
//! Backends: `WOW_GFX_WINDOW` (`x11`, `win32`, `glfw`, `sdl`) and `WOW_GFX_DEVICE` (`gl4`, `gl3`,
//! `gles3`, `vk`, `d3d11`, `d3d12`), see [`backend`]. Shaders: `shaders/compile.sh`.
//!
//! Without the feature this crate is empty.

#![cfg(feature = "gfx")]

pub mod backend;
pub mod bevy_ui;
pub mod context;
pub mod data;
pub mod draw;
#[cfg(feature = "egui")]
pub mod egui;
pub mod events;
pub mod ffi;
pub mod gizmos;
pub mod images;
pub mod input;
pub mod material;
pub mod meshes;
pub mod noop_device;
pub mod overlay;
pub mod pipelines;
pub mod post;
pub mod probe;
pub mod render;
pub mod runner;
pub mod screenshot;
pub mod shader_def;
pub mod shader_loader;
pub mod target;
pub mod timer;
pub mod ui;
pub mod window;

use bevy::app::PluginGroupBuilder;
use bevy::prelude::*;

pub use bevy_ui::{GfxBevyUiPlugin, GfxUiMaterialPlugin};
pub use context::GfxContext;
pub use data::{DataTexture, GfxDataTextures};
#[cfg(feature = "egui")]
pub use egui::GfxEguiPlugin;
pub use images::{GfxTextureWrite, GfxTextureWrites};
pub use material::{
    GfxAlpha, GfxDrawState, GfxMaterial2dPlugin, GfxMaterialDesc, GfxMaterialPlugin, GfxProgram,
    GfxTextureSlot,
};
pub use overlay::{GfxOverlayDraw, GfxOverlayFrame, GfxOverlays};
pub use post::GfxFfxGlow;
pub use probe::{GfxDepthProbe, GfxDepthReadback, GfxDepthRequest, GfxPhaseRecord, GfxViewPhases};
pub use render::{GfxRender, GfxRenderSystems};
pub use runner::GfxMsaaCounts;
pub use timer::GfxGpuMeter;
pub use ui::GfxUiLane;

/// `group` (the `DefaultPlugins` set) with winit and wgpu swapped out for gfx.
pub fn swap_in(group: PluginGroupBuilder) -> PluginGroupBuilder {
    group
        .disable::<bevy::winit::WinitPlugin>()
        .disable::<bevy::render::RenderPlugin>()
        .add_after::<bevy::render::RenderPlugin>(RenderMainWorldPlugin)
        .add(GfxPlugin)
}

/// The gfx runner and frame schedule.
pub struct GfxPlugin;

impl Plugin for GfxPlugin {
    fn build(&self, app: &mut App) {
        render::build(app);
        timer::build(app);
        window::build(app);
        app.add_plugins(GfxMaterialPlugin::<StandardMaterial>::new(
            material::standard,
        ));
        app.add_systems(Startup, gizmos::init)
            .add_systems(GfxRender, gizmos::collect.in_set(GfxRenderSystems::Collect));
        app.set_runner(runner::run);
    }

    fn finish(&self, app: &mut App) {
        // The sampler an `ImageSampler::Default` image gets, as `TexturePlugin::finish` reads it.
        let sampler = app
            .get_added_plugins::<bevy::image::ImagePlugin>()
            .first()
            .map(|p| p.default_sampler.clone())
            .unwrap_or_default();
        app.insert_resource(runner::DefaultSampler(sampler));
        screenshot::finish(app);
    }
}

/// The main-world half of `RenderPlugin` (bevy_render 0.18.1, `RenderPlugin::build`) with no
/// render sub-app: the shader asset and the camera, view, mesh, texture and readback plugins,
/// each of which registers its main-world types and systems and skips its render-world part when
/// there is no `RenderApp`. The same set `RenderPlugin` adds when it creates no device; like that
/// headless configuration, each `Extract*Plugin` among them logs that there is no render app.
struct RenderMainWorldPlugin;

impl Plugin for RenderMainWorldPlugin {
    fn build(&self, app: &mut App) {
        use bevy::render::*;
        app.init_asset::<bevy::shader::Shader>()
            .init_asset_loader::<bevy::shader::ShaderLoader>();
        // What `RenderPlugin::finish` publishes from the device, read by `TexturePlugin::finish`
        // and the BLP loader: BC for every gfx device that uploads BC blocks, which is all but
        // gles3 (`gles3.c` has no S3TC formats), where BLPs decode to RGBA8. The device does not
        // exist yet; the backend it will be does.
        let bc = crate::backend::Backends::from_env()
            .is_ok_and(|b| b.device != crate::ffi::GfxDeviceBackend::Gles3);
        app.insert_resource(bevy::image::CompressedImageFormatSupport(if bc {
            bevy::image::CompressedImageFormats::BC
        } else {
            bevy::image::CompressedImageFormats::NONE
        }));
        app.add_plugins((
            view::window::WindowRenderPlugin,
            camera::CameraPlugin,
            view::ViewPlugin,
            mesh::MeshRenderAssetPlugin,
            // `RenderPlugin` adds it under bevy_render's `morph`, which the workspace's bevy has.
            mesh::MorphPlugin,
            globals::GlobalsPlugin,
            texture::TexturePlugin,
            batching::gpu_preprocessing::BatchingPlugin::default(),
            sync_world::SyncWorldPlugin,
            storage::StoragePlugin,
            gpu_readback::GpuReadbackPlugin::default(),
            experimental::occlusion_culling::OcclusionCullingPlugin,
        ));
        app.init_resource::<render_asset::RenderAssetBytesPerFrame>();
    }
}
