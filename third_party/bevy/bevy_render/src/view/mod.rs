pub mod visibility;
pub mod window;

pub use visibility::*;
pub use window::*;

use crate::wgpu::{RenderPassColorAttachment, TextureFormat};
use crate::{
    experimental::occlusion_culling::OcclusionCulling,
    extract_component::ExtractComponentPlugin,
    render_resource::{Texture, TextureView},
    texture::{ColorAttachment, OutputColorAttachment},
};
use alloc::sync::Arc;
use bevy_app::{App, Plugin};
use bevy_color::LinearRgba;
use bevy_ecs::prelude::*;
use bevy_render_macros::ExtractComponent;
use core::sync::atomic::{AtomicUsize, Ordering};

pub struct ViewPlugin;

impl Plugin for ViewPlugin {
    fn build(&self, app: &mut App) {
        app
            // NOTE: windows.is_changed() handles cases where a window was resized
            .add_plugins((
                ExtractComponentPlugin::<Hdr>::default(),
                ExtractComponentPlugin::<Msaa>::default(),
                ExtractComponentPlugin::<OcclusionCulling>::default(),
                RenderVisibilityRangePlugin,
            ));
    }
}

/// Component for configuring the number of samples for [Multi-Sample Anti-Aliasing](https://en.wikipedia.org/wiki/Multisample_anti-aliasing)
/// for a [`Camera`](bevy_camera::Camera).
///
/// Defaults to 4 samples. A higher number of samples results in smoother edges.
///
/// Some advanced rendering features may require that MSAA is disabled.
///
/// Note that the web currently only supports 1 or 4 samples.
#[derive(
    Component, Default, Clone, Copy, ExtractComponent, PartialEq, PartialOrd, Eq, Hash, Debug,
)]
pub enum Msaa {
    Off = 1,
    Sample2 = 2,
    #[default]
    Sample4 = 4,
    Sample8 = 8,
}

impl Msaa {
    #[inline]
    pub fn samples(&self) -> u32 {
        *self as u32
    }
}

/// If this component is added to a camera, the camera will use an intermediate "high dynamic range" render texture.
/// This allows rendering with a wider range of lighting values. However, this does *not* affect
/// whether the camera will render with hdr display output (which bevy does not support currently)
/// and only affects the intermediate render texture.
#[derive(Component, Default, Copy, Clone, ExtractComponent, PartialEq, Eq, Hash, Debug)]
pub struct Hdr;

/// Configures filmic color grading parameters to adjust the image appearance.
///
/// Color grading is applied just before tonemapping for a given
/// [`Camera`](bevy_camera::Camera) entity.
#[derive(Component, Debug, Default, Clone)]
pub struct ColorGrading;

#[derive(Component, Clone)]
pub struct ViewTarget {
    main_textures: MainTargetTextures,
    main_texture_format: TextureFormat,
    /// 0 represents `main_textures.a`, 1 represents `main_textures.b`
    /// This is shared across view targets with the same render target
    main_texture: Arc<AtomicUsize>,
    out_texture: OutputColorAttachment,
}

pub struct PostProcessWrite<'a> {
    pub source: &'a TextureView,
    pub source_texture: &'a Texture,
    pub destination: &'a TextureView,
    pub destination_texture: &'a Texture,
}

/// Add this component to a camera to disable *indirect mode*.
///
/// Indirect mode, automatically enabled on supported hardware, allows Bevy to
/// offload transform and cull operations to the GPU, reducing CPU overhead.
/// Doing this, however, reduces the amount of control that your app has over
/// instancing decisions. In certain circumstances, you may want to disable
/// indirect drawing so that your app can manually instance meshes as it sees
/// fit. See the `custom_shader_instancing` example.
///
/// The vast majority of applications will not need to use this component, as it
/// generally reduces rendering performance.
///
/// Note: This component should only be added when initially spawning a camera. Adding
/// or removing after spawn can result in unspecified behavior.
#[derive(Component, Default)]
pub struct NoIndirectDrawing;

impl ViewTarget {
    /// The "main" unsampled texture.
    pub fn main_texture_view(&self) -> &TextureView {
        if self.main_texture.load(Ordering::SeqCst) == 0 {
            &self.main_textures.a.texture.default_view
        } else {
            &self.main_textures.b.texture.default_view
        }
    }

    #[inline]
    pub fn main_texture_format(&self) -> TextureFormat {
        self.main_texture_format
    }

    pub fn out_texture_color_attachment(
        &self,
        clear_color: Option<LinearRgba>,
    ) -> RenderPassColorAttachment<'_> {
        self.out_texture.get_attachment(clear_color)
    }

    /// The format of the final texture this view will render to
    #[inline]
    pub fn out_texture_view_format(&self) -> TextureFormat {
        self.out_texture.view_format
    }

    /// This will start a new "post process write", which assumes that the caller
    /// will write the [`PostProcessWrite`]'s `source` to the `destination`.
    ///
    /// `source` is the "current" main texture. This will internally flip this
    /// [`ViewTarget`]'s main texture to the `destination` texture, so the caller
    /// _must_ ensure `source` is copied to `destination`, with or without modifications.
    /// Failing to do so will cause the current main texture information to be lost.
    pub fn post_process_write(&self) -> PostProcessWrite<'_> {
        let old_is_a_main_texture = self.main_texture.fetch_xor(1, Ordering::SeqCst);
        // if the old main texture is a, then the post processing must write from a to b
        if old_is_a_main_texture == 0 {
            self.main_textures.b.mark_as_cleared();
            PostProcessWrite {
                source: &self.main_textures.a.texture.default_view,
                source_texture: &self.main_textures.a.texture.texture,
                destination: &self.main_textures.b.texture.default_view,
                destination_texture: &self.main_textures.b.texture.texture,
            }
        } else {
            self.main_textures.a.mark_as_cleared();
            PostProcessWrite {
                source: &self.main_textures.b.texture.default_view,
                source_texture: &self.main_textures.b.texture.texture,
                destination: &self.main_textures.a.texture.default_view,
                destination_texture: &self.main_textures.a.texture.texture,
            }
        }
    }
}

#[derive(Clone)]
struct MainTargetTextures {
    a: ColorAttachment,
    b: ColorAttachment,
    /// 0 represents `main_textures.a`, 1 represents `main_textures.b`
    /// This is shared across view targets with the same render target
    #[expect(
        dead_code,
        reason = "read only by the render world's view-target preparation, which is cut"
    )]
    main_texture: Arc<AtomicUsize>,
}
