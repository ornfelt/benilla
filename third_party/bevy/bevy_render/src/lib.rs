//! # Useful Environment Variables
//!
//! Both `bevy_render` and `wgpu` have a number of environment variable options for changing the runtime behavior
//! of both crates. Many of these may be useful in development or release environments.
//!
//! - `WGPU_DEBUG=1` enables debug labels, which can be useful in release builds.
//! - `WGPU_VALIDATION=0` disables validation layers. This can help with particularly spammy errors.
//! - `WGPU_FORCE_FALLBACK_ADAPTER=1` attempts to force software rendering. This typically matches what is used in CI.
//! - `WGPU_ADAPTER_NAME` allows selecting a specific adapter by name.
//! - `WGPU_SETTINGS_PRIO=webgl2` uses webgl2 limits.
//! - `WGPU_SETTINGS_PRIO=compatibility` uses webgpu limits.
//! - `VERBOSE_SHADER_ERROR=1` prints more detailed information about WGSL compilation errors, such as shader defs and shader entrypoint.

#![expect(missing_docs, reason = "Not all docs are written yet, see #3492.")]
#![expect(unsafe_code, reason = "Unsafe code is used to improve performance.")]
#![cfg_attr(
    any(docsrs, docsrs_dep),
    expect(
        internal_features,
        reason = "rustdoc_internals is needed for fake_variadic"
    )
)]
#![cfg_attr(any(docsrs, docsrs_dep), feature(doc_cfg, rustdoc_internals))]
#![doc(
    html_logo_url = "https://bevy.org/assets/icon.png",
    html_favicon_url = "https://bevy.org/assets/icon.png"
)]

#[cfg(target_pointer_width = "16")]
compile_error!("bevy_render cannot compile for a 16-bit platform.");

extern crate alloc;
extern crate core;

// Required to make proc macros work in bevy itself.
extern crate self as bevy_render;

pub mod alpha;
pub mod batching;
pub mod camera;
pub mod diagnostic;
pub mod experimental;
pub mod extract_component;
mod extract_param;
pub mod extract_resource;
pub mod globals;
pub mod gpu_readback;
pub mod mesh;
pub mod pipelined_rendering;
pub mod render_asset;
pub mod render_graph;
pub mod render_resource;
pub mod renderer;
pub mod settings;
pub mod storage;
pub mod sync_component;
pub mod sync_world;
pub mod texture;
pub mod view;
mod wgpu;

/// The render prelude.
///
/// This includes the most common types in this crate, re-exported for your convenience.
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        alpha::AlphaMode, camera::NormalizedRenderTargetExt as _, texture::ManualTextureViews,
        view::Msaa, ExtractSchedule,
    };
}

pub use extract_param::Extract;

use crate::{
    camera::CameraPlugin,
    gpu_readback::GpuReadbackPlugin,
    mesh::MeshRenderAssetPlugin,
    storage::StoragePlugin,
    texture::TexturePlugin,
    view::{ViewPlugin, WindowRenderPlugin},
};
use batching::gpu_preprocessing::BatchingPlugin;
use bevy_app::{App, AppLabel, Plugin};
use bevy_asset::AssetApp;
use bevy_ecs::{prelude::*, schedule::ScheduleLabel};
use bevy_shader::{Shader, ShaderLoader};
use bitflags::bitflags;
use core::ops::{Deref, DerefMut};
use experimental::occlusion_culling::OcclusionCullingPlugin;
use globals::GlobalsPlugin;
use render_asset::RenderAssetBytesPerFrame;
use sync_world::SyncWorldPlugin;

/// Contains the default Bevy rendering backend based on wgpu.
///
/// Rendering is done in a [`SubApp`], which exchanges data with the main app
/// between main schedule iterations.
///
/// Rendering can be executed between iterations of the main schedule,
/// or it can be executed in parallel with main schedule when
/// [`PipelinedRenderingPlugin`](pipelined_rendering::PipelinedRenderingPlugin) is enabled.
#[derive(Default)]
pub struct RenderPlugin {
    /// Debugging flags that can optionally be set when constructing the renderer.
    pub debug_flags: RenderDebugFlags,
}

bitflags! {
    /// Debugging flags that can optionally be set when constructing the renderer.
    #[derive(Clone, Copy, PartialEq, Default, Debug)]
    pub struct RenderDebugFlags: u8 {
        /// If true, this sets the `COPY_SRC` flag on indirect draw parameters
        /// so that they can be read back to CPU.
        ///
        /// This is a debugging feature that may reduce performance. It
        /// primarily exists for the `occlusion_culling` example.
        const ALLOW_COPIES_FROM_INDIRECT_PARAMETERS = 1;
    }
}

/// The startup schedule of the [`RenderApp`]
#[derive(ScheduleLabel, Debug, Hash, PartialEq, Eq, Clone, Default)]
pub struct RenderStartup;

/// The main render schedule.
#[derive(ScheduleLabel, Debug, Hash, PartialEq, Eq, Clone, Default)]
pub struct Render;

/// Schedule in which data from the main world is 'extracted' into the render world.
///
/// This step should be kept as short as possible to increase the "pipelining potential" for
/// running the next frame while rendering the current frame.
///
/// This schedule is run on the render world, but it also has access to the main world.
/// See [`MainWorld`] and [`Extract`] for details on how to access main world data from this schedule.
#[derive(ScheduleLabel, PartialEq, Eq, Debug, Clone, Hash, Default)]
pub struct ExtractSchedule;

/// The simulation [`World`] of the application, stored as a resource.
///
/// This resource is only available during [`ExtractSchedule`] and not
/// during command application of that schedule.
/// See [`Extract`] for more details.
#[derive(Resource, Default)]
pub struct MainWorld(World);

impl Deref for MainWorld {
    type Target = World;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for MainWorld {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// A label for the rendering sub-app.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, AppLabel)]
pub struct RenderApp;

impl Plugin for RenderPlugin {
    /// Adds the main-world half of the renderer. The rendering sub-app, the wgpu device and every
    /// system that ran in them were built only for a `render_creation` that opens a device: under
    /// gfx `RenderPlugin` is disabled and no `RenderApp` exists.
    fn build(&self, app: &mut App) {
        app.init_asset::<Shader>()
            .init_asset_loader::<ShaderLoader>();

        app.add_plugins((
            WindowRenderPlugin,
            CameraPlugin,
            ViewPlugin,
            MeshRenderAssetPlugin,
            GlobalsPlugin,
            #[cfg(feature = "morph")]
            mesh::MorphPlugin,
            TexturePlugin,
            BatchingPlugin {
                debug_flags: self.debug_flags,
            },
            SyncWorldPlugin,
            StoragePlugin,
            GpuReadbackPlugin::default(),
            OcclusionCullingPlugin,
            #[cfg(feature = "tracing-tracy")]
            diagnostic::RenderDiagnosticsPlugin,
        ));

        app.init_resource::<RenderAssetBytesPerFrame>();
    }
}
