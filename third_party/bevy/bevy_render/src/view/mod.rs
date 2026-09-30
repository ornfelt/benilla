pub mod visibility;
pub mod window;

pub use visibility::*;
pub use window::*;

use crate::wgpu::{RenderPassColorAttachment, TextureFormat};
use crate::{
    experimental::occlusion_culling::OcclusionCulling,
    extract_component::ExtractComponentPlugin,
    render_resource::{Texture, TextureView},
    sync_world::MainEntity,
    texture::{ColorAttachment, OutputColorAttachment},
};
use alloc::sync::Arc;
use bevy_app::{App, Plugin};
use bevy_color::LinearRgba;
use bevy_ecs::prelude::*;
use bevy_math::{Mat4, UVec4};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render_macros::ExtractComponent;
use bevy_transform::components::GlobalTransform;
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

/// An identifier for a view that is stable across frames.
///
/// We can't use [`Entity`] for this because render world entities aren't
/// stable, and we can't use just [`MainEntity`] because some main world views
/// extract to multiple render world views. For example, a directional light
/// extracts to one render world view per cascade, and a point light extracts to
/// one render world view per cubemap face. So we pair the main entity with an
/// *auxiliary entity* and a *subview index*, which *together* uniquely identify
/// a view in the render world in a way that's stable from frame to frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RetainedViewEntity {
    /// The main entity that this view corresponds to.
    pub main_entity: MainEntity,

    /// Another entity associated with the view entity.
    ///
    /// This is currently used for shadow cascades. If there are multiple
    /// cameras, each camera needs to have its own set of shadow cascades. Thus
    /// the light and subview index aren't themselves enough to uniquely
    /// identify a shadow cascade: we need the camera that the cascade is
    /// associated with as well. This entity stores that camera.
    ///
    /// If not present, this will be `MainEntity(Entity::PLACEHOLDER)`.
    pub auxiliary_entity: MainEntity,

    /// The index of the view corresponding to the entity.
    ///
    /// For example, for point lights that cast shadows, this is the index of
    /// the cubemap face (0 through 5 inclusive). For directional lights, this
    /// is the index of the cascade.
    pub subview_index: u32,
}

impl RetainedViewEntity {
    /// Creates a new [`RetainedViewEntity`] from the given main world entity,
    /// auxiliary main world entity, and subview index.
    ///
    /// See [`RetainedViewEntity::subview_index`] for an explanation of what
    /// `auxiliary_entity` and `subview_index` are.
    pub fn new(
        main_entity: MainEntity,
        auxiliary_entity: Option<MainEntity>,
        subview_index: u32,
    ) -> Self {
        Self {
            main_entity,
            auxiliary_entity: auxiliary_entity.unwrap_or(Entity::PLACEHOLDER.into()),
            subview_index,
        }
    }
}

/// Describes a camera in the render world.
///
/// Each entity in the main world can potentially extract to multiple subviews,
/// each of which has a [`RetainedViewEntity::subview_index`]. For instance, 3D
/// cameras extract to both a 3D camera subview with index 0 and a special UI
/// subview with index 1. Likewise, point lights with shadows extract to 6
/// subviews, one for each side of the shadow cubemap.
#[derive(Component)]
pub struct ExtractedView {
    /// The entity in the main world corresponding to this render world view.
    pub retained_view_entity: RetainedViewEntity,
    /// Typically a column-major right-handed projection matrix, one of either:
    ///
    /// Perspective (infinite reverse z)
    /// ```text
    /// f = 1 / tan(fov_y_radians / 2)
    ///
    /// ⎡ f / aspect  0   0     0 ⎤
    /// ⎢          0  f   0     0 ⎥
    /// ⎢          0  0   0  near ⎥
    /// ⎣          0  0  -1     0 ⎦
    /// ```
    ///
    /// Orthographic
    /// ```text
    /// w = right - left
    /// h = top - bottom
    /// d = far - near
    /// cw = -right - left
    /// ch = -top - bottom
    ///
    /// ⎡ 2 / w      0      0   cw / w ⎤
    /// ⎢     0  2 / h      0   ch / h ⎥
    /// ⎢     0      0  1 / d  far / d ⎥
    /// ⎣     0      0      0        1 ⎦
    /// ```
    ///
    /// `clip_from_view[3][3] == 1.0` is the standard way to check if a projection is orthographic
    ///
    /// Glam matrices are column major, so for example getting the near plane of a perspective projection is `clip_from_view[3][2]`
    ///
    /// Custom projections are also possible however.
    pub clip_from_view: Mat4,
    pub world_from_view: GlobalTransform,
    // The view-projection matrix. When provided it is used instead of deriving it from
    // `projection` and `transform` fields, which can be helpful in cases where numerical
    // stability matters and there is a more direct way to derive the view-projection matrix.
    pub clip_from_world: Option<Mat4>,
    pub hdr: bool,
    // uvec4(origin.x, origin.y, width, height)
    pub viewport: UVec4,
    pub color_grading: ColorGrading,

    /// Whether to switch culling mode so that materials that request backface
    /// culling cull front faces, and vice versa.
    ///
    /// This is typically used for cameras that mirror the world that they
    /// render across a plane, because doing that flips the winding of each
    /// polygon.
    ///
    /// This setting doesn't affect materials that disable backface culling.
    pub invert_culling: bool,
}

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

impl ColorGrading {
    /// Creates a new [`ColorGrading`] instance in which shadows, midtones, and
    /// highlights all have the same set of color grading values.
    pub fn with_identical_sections(
        global: ColorGradingGlobal,
        section: ColorGradingSection,
    ) -> ColorGrading {
        ColorGrading {
            global,
            highlights: section,
            midtones: section,
            shadows: section,
        }
    }

    /// Returns an iterator that visits the shadows, midtones, and highlights
    /// sections, in that order.
    pub fn all_sections(&self) -> impl Iterator<Item = &ColorGradingSection> {
        [&self.shadows, &self.midtones, &self.highlights].into_iter()
    }

    /// Applies the given mutating function to the shadows, midtones, and
    /// highlights sections, in that order.
    ///
    /// Returns an array composed of the results of such evaluation, in that
    /// order.
    pub fn all_sections_mut(&mut self) -> impl Iterator<Item = &mut ColorGradingSection> {
        [&mut self.shadows, &mut self.midtones, &mut self.highlights].into_iter()
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
    pub const TEXTURE_FORMAT_HDR: TextureFormat = TextureFormat::Rgba16Float;

    /// Retrieve this target's main texture's color attachment.
    pub fn get_color_attachment(&self) -> RenderPassColorAttachment<'_> {
        if self.main_texture.load(Ordering::SeqCst) == 0 {
            self.main_textures.a.get_attachment()
        } else {
            self.main_textures.b.get_attachment()
        }
    }

    /// Retrieve this target's "unsampled" main texture's color attachment.
    pub fn get_unsampled_color_attachment(&self) -> RenderPassColorAttachment<'_> {
        if self.main_texture.load(Ordering::SeqCst) == 0 {
            self.main_textures.a.get_unsampled_attachment()
        } else {
            self.main_textures.b.get_unsampled_attachment()
        }
    }

    /// The "main" unsampled texture.
    pub fn main_texture(&self) -> &Texture {
        if self.main_texture.load(Ordering::SeqCst) == 0 {
            &self.main_textures.a.texture.texture
        } else {
            &self.main_textures.b.texture.texture
        }
    }

    /// The _other_ "main" unsampled texture.
    /// In most cases you should use [`Self::main_texture`] instead and never this.
    /// The textures will naturally be swapped when [`Self::post_process_write`] is called.
    ///
    /// A use case for this is to be able to prepare a bind group for all main textures
    /// ahead of time.
    pub fn main_texture_other(&self) -> &Texture {
        if self.main_texture.load(Ordering::SeqCst) == 0 {
            &self.main_textures.b.texture.texture
        } else {
            &self.main_textures.a.texture.texture
        }
    }

    /// The "main" unsampled texture.
    pub fn main_texture_view(&self) -> &TextureView {
        if self.main_texture.load(Ordering::SeqCst) == 0 {
            &self.main_textures.a.texture.default_view
        } else {
            &self.main_textures.b.texture.default_view
        }
    }

    /// The _other_ "main" unsampled texture view.
    /// In most cases you should use [`Self::main_texture_view`] instead and never this.
    /// The textures will naturally be swapped when [`Self::post_process_write`] is called.
    ///
    /// A use case for this is to be able to prepare a bind group for all main textures
    /// ahead of time.
    pub fn main_texture_other_view(&self) -> &TextureView {
        if self.main_texture.load(Ordering::SeqCst) == 0 {
            &self.main_textures.b.texture.default_view
        } else {
            &self.main_textures.a.texture.default_view
        }
    }

    /// The "main" sampled texture.
    pub fn sampled_main_texture(&self) -> Option<&Texture> {
        self.main_textures
            .a
            .resolve_target
            .as_ref()
            .map(|sampled| &sampled.texture)
    }

    /// The "main" sampled texture view.
    pub fn sampled_main_texture_view(&self) -> Option<&TextureView> {
        self.main_textures
            .a
            .resolve_target
            .as_ref()
            .map(|sampled| &sampled.default_view)
    }

    #[inline]
    pub fn main_texture_format(&self) -> TextureFormat {
        self.main_texture_format
    }

    /// Returns `true` if and only if the main texture is [`Self::TEXTURE_FORMAT_HDR`]
    #[inline]
    pub fn is_hdr(&self) -> bool {
        self.main_texture_format == ViewTarget::TEXTURE_FORMAT_HDR
    }

    /// The final texture this view will render to.
    #[inline]
    pub fn out_texture(&self) -> &TextureView {
        &self.out_texture.view
    }

    pub fn out_texture_color_attachment(
        &self,
        clear_color: Option<LinearRgba>,
    ) -> RenderPassColorAttachment<'_> {
        self.out_texture.get_attachment(clear_color)
    }

    /// Whether the final texture this view will render to needs to be presented.
    pub fn needs_present(&self) -> bool {
        self.out_texture.needs_present()
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
