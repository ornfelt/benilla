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
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render_macros::ExtractComponent;
use core::{
    ops::Range,
    sync::atomic::{AtomicUsize, Ordering},
};

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
    Component,
    Default,
    Clone,
    Copy,
    ExtractComponent,
    Reflect,
    PartialEq,
    PartialOrd,
    Eq,
    Hash,
    Debug,
)]
#[reflect(Component, Default, PartialEq, Hash, Debug)]
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

    pub fn from_samples(samples: u32) -> Self {
        match samples {
            1 => Msaa::Off,
            2 => Msaa::Sample2,
            4 => Msaa::Sample4,
            8 => Msaa::Sample8,
            _ => panic!("Unsupported MSAA sample count: {samples}"),
        }
    }
}

/// If this component is added to a camera, the camera will use an intermediate "high dynamic range" render texture.
/// This allows rendering with a wider range of lighting values. However, this does *not* affect
/// whether the camera will render with hdr display output (which bevy does not support currently)
/// and only affects the intermediate render texture.
#[derive(
    Component, Default, Copy, Clone, ExtractComponent, Reflect, PartialEq, Eq, Hash, Debug,
)]
#[reflect(Component, Default, PartialEq, Hash, Debug)]
pub struct Hdr;

/// Configures filmic color grading parameters to adjust the image appearance.
///
/// Color grading is applied just before tonemapping for a given
/// [`Camera`](bevy_camera::Camera) entity, with the sole exception of the
/// `post_saturation` value in [`ColorGradingGlobal`], which is applied after
/// tonemapping.
#[derive(Component, Reflect, Debug, Default, Clone)]
#[reflect(Component, Default, Debug, Clone)]
pub struct ColorGrading {
    /// Filmic color grading values applied to the image as a whole (as opposed
    /// to individual sections, like shadows and highlights).
    pub global: ColorGradingGlobal,

    /// Color grading values that are applied to the darker parts of the image.
    ///
    /// The cutoff points can be customized with the
    /// [`ColorGradingGlobal::midtones_range`] field.
    pub shadows: ColorGradingSection,

    /// Color grading values that are applied to the parts of the image with
    /// intermediate brightness.
    ///
    /// The cutoff points can be customized with the
    /// [`ColorGradingGlobal::midtones_range`] field.
    pub midtones: ColorGradingSection,

    /// Color grading values that are applied to the lighter parts of the image.
    ///
    /// The cutoff points can be customized with the
    /// [`ColorGradingGlobal::midtones_range`] field.
    pub highlights: ColorGradingSection,
}

/// Filmic color grading values applied to the image as a whole (as opposed to
/// individual sections, like shadows and highlights).
#[derive(Clone, Debug, Reflect)]
#[reflect(Default, Clone)]
pub struct ColorGradingGlobal {
    /// Exposure value (EV) offset, measured in stops.
    pub exposure: f32,

    /// An adjustment made to the [CIE 1931] chromaticity *x* value.
    ///
    /// Positive values make the colors redder. Negative values make the colors
    /// bluer. This has no effect on luminance (brightness).
    ///
    /// [CIE 1931]: https://en.wikipedia.org/wiki/CIE_1931_color_space#CIE_xy_chromaticity_diagram_and_the_CIE_xyY_color_space
    pub temperature: f32,

    /// An adjustment made to the [CIE 1931] chromaticity *y* value.
    ///
    /// Positive values make the colors more magenta. Negative values make the
    /// colors greener. This has no effect on luminance (brightness).
    ///
    /// [CIE 1931]: https://en.wikipedia.org/wiki/CIE_1931_color_space#CIE_xy_chromaticity_diagram_and_the_CIE_xyY_color_space
    pub tint: f32,

    /// An adjustment to the [hue], in radians.
    ///
    /// Adjusting this value changes the perceived colors in the image: red to
    /// yellow to green to blue, etc. It has no effect on the saturation or
    /// brightness of the colors.
    ///
    /// [hue]: https://en.wikipedia.org/wiki/HSL_and_HSV#Formal_derivation
    pub hue: f32,

    /// Saturation adjustment applied after tonemapping.
    /// Values below 1.0 desaturate, with a value of 0.0 resulting in a grayscale image
    /// with luminance defined by ITU-R BT.709
    /// Values above 1.0 increase saturation.
    pub post_saturation: f32,

    /// The luminance (brightness) ranges that are considered part of the
    /// "midtones" of the image.
    ///
    /// This affects which [`ColorGradingSection`]s apply to which colors. Note
    /// that the sections smoothly blend into one another, to avoid abrupt
    /// transitions.
    ///
    /// The default value is 0.2 to 0.7.
    pub midtones_range: Range<f32>,
}

/// A section of color grading values that can be selectively applied to
/// shadows, midtones, and highlights.
#[derive(Reflect, Debug, Copy, Clone, PartialEq)]
#[reflect(Clone, PartialEq)]
pub struct ColorGradingSection {
    /// Values below 1.0 desaturate, with a value of 0.0 resulting in a grayscale image
    /// with luminance defined by ITU-R BT.709.
    /// Values above 1.0 increase saturation.
    pub saturation: f32,

    /// Adjusts the range of colors.
    ///
    /// A value of 1.0 applies no changes. Values below 1.0 move the colors more
    /// toward a neutral gray. Values above 1.0 spread the colors out away from
    /// the neutral gray.
    pub contrast: f32,

    /// A nonlinear luminance adjustment, mainly affecting the high end of the
    /// range.
    ///
    /// This is the *n* exponent in the standard [ASC CDL] formula for color
    /// correction:
    ///
    /// ```text
    /// out = (i × s + o)ⁿ
    /// ```
    ///
    /// [ASC CDL]: https://en.wikipedia.org/wiki/ASC_CDL#Combined_Function
    pub gamma: f32,

    /// A linear luminance adjustment, mainly affecting the middle part of the
    /// range.
    ///
    /// This is the *s* factor in the standard [ASC CDL] formula for color
    /// correction:
    ///
    /// ```text
    /// out = (i × s + o)ⁿ
    /// ```
    ///
    /// [ASC CDL]: https://en.wikipedia.org/wiki/ASC_CDL#Combined_Function
    pub gain: f32,

    /// A fixed luminance adjustment, mainly affecting the lower part of the
    /// range.
    ///
    /// This is the *o* term in the standard [ASC CDL] formula for color
    /// correction:
    ///
    /// ```text
    /// out = (i × s + o)ⁿ
    /// ```
    ///
    /// [ASC CDL]: https://en.wikipedia.org/wiki/ASC_CDL#Combined_Function
    pub lift: f32,
}

impl Default for ColorGradingGlobal {
    fn default() -> Self {
        Self {
            exposure: 0.0,
            temperature: 0.0,
            tint: 0.0,
            hue: 0.0,
            post_saturation: 1.0,
            midtones_range: 0.2..0.7,
        }
    }
}

impl Default for ColorGradingSection {
    fn default() -> Self {
        Self {
            saturation: 1.0,
            contrast: 1.0,
            gamma: 1.0,
            gain: 1.0,
            lift: 0.0,
        }
    }
}

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
