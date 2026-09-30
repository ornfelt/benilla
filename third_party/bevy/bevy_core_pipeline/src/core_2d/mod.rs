pub mod graph {
    use bevy_render::render_graph::{RenderLabel, RenderSubGraph};

    #[derive(Debug, Hash, PartialEq, Eq, Clone, RenderSubGraph)]
    pub struct Core2d;

    pub mod input {
        pub const VIEW_ENTITY: &str = "view_entity";
    }

    #[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
    pub enum Node2d {
        MsaaWriteback,
        StartMainPass,
        MainOpaquePass,
        MainTransparentPass,
        EndMainPass,
        Wireframe,
        StartMainPassPostProcessing,
        Bloom,
        PostProcessing,
        Tonemapping,
        Fxaa,
        Smaa,
        Upscaling,
        ContrastAdaptiveSharpening,
        EndMainPassPostProcessing,
    }
}

use bevy_app::{App, Plugin};
use bevy_camera::Camera2d;
use bevy_render::{camera::CameraRenderGraph, extract_component::ExtractComponentPlugin};

use crate::tonemapping::{DebandDither, Tonemapping};

use self::graph::Core2d;

pub struct Core2dPlugin;

impl Plugin for Core2dPlugin {
    fn build(&self, app: &mut App) {
        app.register_required_components::<Camera2d, DebandDither>()
            .register_required_components_with::<Camera2d, CameraRenderGraph>(|| {
                CameraRenderGraph::new(Core2d)
            })
            .register_required_components_with::<Camera2d, Tonemapping>(|| Tonemapping::None)
            .add_plugins(ExtractComponentPlugin::<Camera2d>::default());
    }
}
