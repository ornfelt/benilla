pub mod graph {
    use bevy_render::render_graph::{RenderLabel, RenderSubGraph};

    #[derive(Debug, Hash, PartialEq, Eq, Clone, RenderSubGraph)]
    pub struct Core3d;

    pub mod input {
        pub const VIEW_ENTITY: &str = "view_entity";
    }

    #[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
    pub enum Node3d {
        MsaaWriteback,
        EarlyPrepass,
        EarlyDownsampleDepth,
        LatePrepass,
        EarlyDeferredPrepass,
        LateDeferredPrepass,
        CopyDeferredLightingId,
        EndPrepasses,
        StartMainPass,
        MainOpaquePass,
        MainTransmissivePass,
        MainTransparentPass,
        EndMainPass,
        Wireframe,
        StartMainPassPostProcessing,
        LateDownsampleDepth,
        MotionBlur,
        Taa,
        DlssSuperResolution,
        DlssRayReconstruction,
        Bloom,
        AutoExposure,
        DepthOfField,
        PostProcessing,
        Tonemapping,
        Fxaa,
        Smaa,
        Upscaling,
        ContrastAdaptiveSharpening,
        EndMainPassPostProcessing,
    }
}

use bevy_app::{App, Plugin, PostUpdate};
use bevy_camera::{Camera, Camera3d};
use bevy_ecs::prelude::*;
use bevy_render::{
    camera::CameraRenderGraph, extract_component::ExtractComponentPlugin, prelude::Msaa,
};
use tracing::warn;

use crate::{
    prepass::DeferredPrepass,
    skybox::SkyboxPlugin,
    tonemapping::{DebandDither, Tonemapping},
};

use self::graph::Core3d;

pub struct Core3dPlugin;

impl Plugin for Core3dPlugin {
    fn build(&self, app: &mut App) {
        app.register_required_components_with::<Camera3d, DebandDither>(|| DebandDither::Enabled)
            .register_required_components_with::<Camera3d, CameraRenderGraph>(|| {
                CameraRenderGraph::new(Core3d)
            })
            .register_required_components::<Camera3d, Tonemapping>()
            .add_plugins((SkyboxPlugin, ExtractComponentPlugin::<Camera3d>::default()))
            .add_systems(PostUpdate, check_msaa);
    }
}

pub fn check_msaa(mut deferred_views: Query<&mut Msaa, (With<Camera>, With<DeferredPrepass>)>) {
    for mut msaa in deferred_views.iter_mut() {
        match *msaa {
            Msaa::Off => (),
            _ => {
                warn!("MSAA is incompatible with deferred rendering and has been disabled.");
                *msaa = Msaa::Off;
            }
        };
    }
}
