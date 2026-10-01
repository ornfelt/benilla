#![expect(missing_docs, reason = "Not all docs are written yet, see #3492.")]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://bevy.org/assets/icon.png",
    html_favicon_url = "https://bevy.org/assets/icon.png"
)]

extern crate alloc;

mod atmosphere;
pub mod decal;
pub mod deferred;
mod extended_material;
mod fog;
mod lightmap;
mod material;
mod medium;
mod mesh_material;
mod parallax;
mod pbr_material;
mod prepass;
mod render;
mod ssao;
mod ssr;
mod volumetric_fog;

use bevy_color::{Color, LinearRgba};

pub use atmosphere::*;
use bevy_light::{
    AmbientLight, DirectionalLight, PointLight, ShadowFilteringMethod, SimulationLightSystems,
    SpotLight,
};
use bevy_shader::ShaderRef;
pub use decal::clustered::ClusteredDecalPlugin;
pub use extended_material::*;
pub use fog::*;
pub use lightmap::*;
pub use material::*;
pub use medium::*;
pub use mesh_material::*;
pub use parallax::*;
pub use pbr_material::*;
pub use prepass::*;
pub use render::*;
pub use ssao::*;
pub use ssr::*;
pub use volumetric_fog::VolumetricFogPlugin;

/// The PBR prelude.
///
/// This includes the most common types in this crate, re-exported for your convenience.
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        fog::{DistanceFog, FogFalloff},
        material::{Material, MaterialPlugin},
        mesh_material::MeshMaterial3d,
        parallax::ParallaxMappingMethod,
        pbr_material::StandardMaterial,
        ssao::ScreenSpaceAmbientOcclusionPlugin,
    };
}

use crate::deferred::DeferredPbrLightingPlugin;
use bevy_app::prelude::*;
use bevy_asset::{AssetApp, AssetPath, Assets, Handle, RenderAssetUsages};
use bevy_ecs::prelude::*;
use bevy_image::{Image, ImageSampler};
use bevy_render::{
    alpha::AlphaMode,
    extract_resource::ExtractResourcePlugin,
    render_resource::{
        Extent3d, TextureDataOrder, TextureDescriptor, TextureDimension, TextureFormat,
        TextureUsages,
    },
    sync_component::SyncComponentPlugin,
    RenderDebugFlags,
};

use std::path::PathBuf;

fn shader_ref(path: PathBuf) -> ShaderRef {
    ShaderRef::Path(AssetPath::from_path_buf(path).with_source("embedded"))
}

/// Sets up the entire PBR infrastructure of bevy.
pub struct PbrPlugin {
    /// Controls if the prepass is enabled for the [`StandardMaterial`].
    /// For more information about what a prepass is, see the [`bevy_core_pipeline::prepass`] docs.
    pub prepass_enabled: bool,
    /// Controls if [`DeferredPbrLightingPlugin`] is added.
    pub add_default_deferred_lighting_plugin: bool,
    /// Controls if GPU [`MeshUniform`] building is enabled.
    ///
    /// This requires compute shader support and so will be forcibly disabled if
    /// the platform doesn't support those.
    pub use_gpu_instance_buffer_builder: bool,
    /// Debugging flags that can optionally be set when constructing the renderer.
    pub debug_flags: RenderDebugFlags,
}

impl Default for PbrPlugin {
    fn default() -> Self {
        Self {
            prepass_enabled: true,
            add_default_deferred_lighting_plugin: true,
            use_gpu_instance_buffer_builder: true,
            debug_flags: RenderDebugFlags::default(),
        }
    }
}

impl Plugin for PbrPlugin {
    fn build(&self, app: &mut App) {
        // The shader libraries, the mesh, light probe, lightmap and GPU preprocessing plugins,
        // the render-world systems, observers and shadow passes, and `finish` only reached the
        // RenderApp, which gfx does not have.
        app.init_resource::<DefaultOpaqueRendererMethod>()
            .add_plugins((
                MaterialsPlugin {
                    debug_flags: self.debug_flags,
                },
                MaterialPlugin::<StandardMaterial> {
                    debug_flags: self.debug_flags,
                    ..Default::default()
                },
                ScreenSpaceAmbientOcclusionPlugin,
                FogPlugin,
                ExtractResourcePlugin::<DefaultOpaqueRendererMethod>::default(),
                SyncComponentPlugin::<ShadowFilteringMethod>::default(),
                VolumetricFogPlugin,
                ScreenSpaceReflectionsPlugin,
                ClusteredDecalPlugin,
            ))
            .add_plugins((
                decal::ForwardDecalPlugin,
                SyncComponentPlugin::<DirectionalLight>::default(),
                SyncComponentPlugin::<PointLight>::default(),
                SyncComponentPlugin::<SpotLight>::default(),
                SyncComponentPlugin::<AmbientLight>::default(),
            ))
            .add_plugins((ScatteringMediumPlugin, AtmospherePlugin))
            .configure_sets(
                PostUpdate,
                (
                    SimulationLightSystems::AddClusters,
                    SimulationLightSystems::AssignLightsToClusters,
                )
                    .chain(),
            );

        if self.add_default_deferred_lighting_plugin {
            app.add_plugins(DeferredPbrLightingPlugin);
        }

        // Initialize the default material handle.
        app.world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .insert(
                &Handle::<StandardMaterial>::default(),
                StandardMaterial {
                    base_color: Color::srgb(1.0, 0.0, 0.5),
                    ..Default::default()
                },
            )
            .unwrap();

        // The blue-noise placeholder was handed to the render world; with no RenderApp it is
        // added and dropped here, as before.
        let mut images = app.world_mut().resource_mut::<Assets<Image>>();
        let _handle = images.add(stbn_placeholder());
    }
}

pub fn stbn_placeholder() -> Image {
    let format = TextureFormat::Rgba8Unorm;
    let data = vec![255, 0, 255, 255];
    Image {
        data: Some(data),
        data_order: TextureDataOrder::default(),
        texture_descriptor: TextureDescriptor {
            size: Extent3d::default(),
            format,
            dimension: TextureDimension::D2,
            label: None,
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        },
        sampler: ImageSampler::Default,
        texture_view_descriptor: None,
        asset_usage: RenderAssetUsages::RENDER_WORLD,
        copy_on_resize: false,
    }
}
