//! Run a prepass before the main pass to generate depth, normals, and/or motion vectors textures, sometimes called a thin g-buffer.
//! These textures are useful for various screen-space effects and reducing overdraw in the main pass.
//!
//! The prepass only runs for opaque meshes or meshes with an alpha mask. Transparent meshes are ignored.
//!
//! To enable the prepass, you need to add a prepass component to a [`bevy_camera::Camera3d`].
//!
//! [`DepthPrepass`]
//! [`NormalPrepass`]
//! [`MotionVectorPrepass`]
//!
//! The textures are automatically added to the default mesh view bindings. You can also get the raw textures
//! by querying the [`ViewPrepassTextures`] component on any camera with a prepass component.
//!
//! The depth prepass will always run and generate the depth buffer as a side effect, but it won't copy it
//! to a separate texture unless the [`DepthPrepass`] is activated. This means that if any prepass component is present
//! it will always create a depth buffer that will be used by the main pass.
//!
//! When using the default mesh view bindings you should be able to use `prepass_depth()`,
//! `prepass_normal()`, and `prepass_motion_vector()` to load the related textures.
//! These functions are defined in `bevy_pbr::prepass_utils`. See the `shader_prepass` example that shows how to use them.
//!
//! The prepass runs for each `Material`. You can control if the prepass should run per-material by setting the `prepass_enabled`
//! flag on the `MaterialPlugin`.
//!
//! Currently only works for 3D.

use bevy_ecs::prelude::*;
use bevy_math::Mat4;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::render_resource::ShaderType;

/// If added to a [`bevy_camera::Camera3d`] then depth values will be copied to a separate texture available to the main pass.
#[derive(Component, Default, Reflect, Clone)]
#[reflect(Component, Default, Clone)]
pub struct DepthPrepass;

/// If added to a [`bevy_camera::Camera3d`] then vertex world normals will be copied to a separate texture available to the main pass.
/// Normals will have normal map textures already applied.
#[derive(Component, Default, Reflect, Clone)]
#[reflect(Component, Default, Clone)]
pub struct NormalPrepass;

/// If added to a [`bevy_camera::Camera3d`] then screen space motion vectors will be copied to a separate texture available to the main pass.
///
/// Motion vectors are stored in the range -1,1, with +x right and +y down.
/// A value of (1.0,1.0) indicates a pixel moved from the top left corner to the bottom right corner of the screen.
#[derive(Component, Default, Reflect, Clone)]
#[reflect(Component, Default, Clone)]
pub struct MotionVectorPrepass;

/// If added to a [`bevy_camera::Camera3d`] then deferred materials will be rendered to the deferred gbuffer texture and will be available to subsequent passes.
/// Note the default deferred lighting plugin also requires `DepthPrepass` to work correctly.
#[derive(Component, Default, Reflect)]
#[reflect(Component, Default)]
pub struct DeferredPrepass;

/// Allows querying the previous frame's [`DepthPrepass`].
#[derive(Component, Default, Reflect, Clone)]
#[reflect(Component, Default, Clone)]
#[require(DepthPrepass)]
pub struct DepthPrepassDoubleBuffer;

/// Allows querying the previous frame's [`DeferredPrepass`].
#[derive(Component, Default, Reflect, Clone)]
#[reflect(Component, Default, Clone)]
#[require(DeferredPrepass)]
pub struct DeferredPrepassDoubleBuffer;

/// View matrices from the previous frame.
///
/// Useful for temporal rendering techniques that need access to last frame's camera data.
#[derive(Component, ShaderType, Clone)]
pub struct PreviousViewData {
    pub view_from_world: Mat4,
    pub clip_from_world: Mat4,
    pub clip_from_view: Mat4,
    pub world_from_clip: Mat4,
    pub view_from_clip: Mat4,
}
