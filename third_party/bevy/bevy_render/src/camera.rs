use crate::{
    extract_component::{ExtractComponent, ExtractComponentPlugin},
    extract_resource::{ExtractResource, ExtractResourcePlugin},
    render_graph::{InternedRenderSubGraph, RenderSubGraph},
    sync_world::SyncToRenderWorld,
    texture::ManualTextureViews,
    view::{ColorGrading, Msaa},
};
use bevy_app::{App, Plugin, PostStartup, PostUpdate};
use bevy_asset::{AssetEvent, AssetEventSystems, AssetId, Assets};
use bevy_camera::{
    visibility::{self},
    Camera, Camera2d, Camera3d, CameraMainTextureUsages, CameraOutputMode, CameraUpdateSystems,
    ClearColor, ClearColorConfig, Exposure, ManualTextureViewHandle, MsaaWriteback,
    NormalizedRenderTarget, Projection, RenderTarget, RenderTargetInfo, Viewport,
};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    change_detection::DetectChanges,
    component::Component,
    entity::{ContainsEntity, Entity},
    error::BevyError,
    lifecycle::HookContext,
    message::MessageReader,
    prelude::With,
    query::QueryItem,
    schedule::IntoScheduleConfigs,
    system::{Query, Res},
    world::DeferredWorld,
};
use bevy_image::Image;
use bevy_math::{uvec2, UVec2};
use bevy_platform::collections::HashSet;
use bevy_window::{PrimaryWindow, Window, WindowCreated, WindowResized, WindowScaleFactorChanged};
use tracing::warn;

#[derive(Default)]
pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.register_required_components::<Camera, Msaa>()
            .register_required_components::<Camera, SyncToRenderWorld>()
            .register_required_components::<Camera3d, ColorGrading>()
            .register_required_components::<Camera3d, Exposure>()
            .add_plugins((
                ExtractResourcePlugin::<ClearColor>::default(),
                ExtractComponentPlugin::<CameraMainTextureUsages>::default(),
            ))
            .add_systems(PostStartup, camera_system.in_set(CameraUpdateSystems))
            .add_systems(
                PostUpdate,
                camera_system
                    .in_set(CameraUpdateSystems)
                    .before(AssetEventSystems)
                    .before(visibility::update_frusta),
            );
        app.world_mut()
            .register_component_hooks::<Camera>()
            .on_add(warn_on_no_render_graph);
    }
}

fn warn_on_no_render_graph(world: DeferredWorld, HookContext { entity, caller, .. }: HookContext) {
    if !world.entity(entity).contains::<CameraRenderGraph>() {
        warn!("{}Entity {entity} has a `Camera` component, but it doesn't have a render graph configured. Usually, adding a `Camera2d` or `Camera3d` component will work.
        However, you may instead need to enable `bevy_core_pipeline`, or may want to manually add a `CameraRenderGraph` component to create a custom render graph.", caller.map(|location|format!("{location}: ")).unwrap_or_default());
    }
}

impl ExtractResource for ClearColor {
    type Source = Self;

    fn extract_resource(source: &Self::Source) -> Self {
        source.clone()
    }
}
impl ExtractComponent for CameraMainTextureUsages {
    type QueryData = &'static Self;
    type QueryFilter = ();
    type Out = Self;

    fn extract_component(item: QueryItem<Self::QueryData>) -> Option<Self::Out> {
        Some(*item)
    }
}
impl ExtractComponent for Camera2d {
    type QueryData = &'static Self;
    type QueryFilter = With<Camera>;
    type Out = Self;

    fn extract_component(item: QueryItem<Self::QueryData>) -> Option<Self::Out> {
        Some(item.clone())
    }
}
impl ExtractComponent for Camera3d {
    type QueryData = &'static Self;
    type QueryFilter = With<Camera>;
    type Out = Self;

    fn extract_component(item: QueryItem<Self::QueryData>) -> Option<Self::Out> {
        Some(item.clone())
    }
}

/// Configures the [`RenderGraph`] name assigned to be run for a given [`Camera`] entity.
#[derive(Component, Debug, Deref, DerefMut, Clone)]
pub struct CameraRenderGraph(InternedRenderSubGraph);

impl CameraRenderGraph {
    /// Creates a new [`CameraRenderGraph`] from any string-like type.
    #[inline]
    pub fn new<T: RenderSubGraph>(name: T) -> Self {
        Self(name.intern())
    }
}

pub trait NormalizedRenderTargetExt {
    fn get_render_target_info<'a>(
        &self,
        resolutions: impl IntoIterator<Item = (Entity, &'a Window)>,
        images: &Assets<Image>,
        manual_texture_views: &ManualTextureViews,
    ) -> Result<RenderTargetInfo, MissingRenderTargetInfoError>;

    // Check if this render target is contained in the given changed windows or images.
    fn is_changed(
        &self,
        changed_window_ids: &HashSet<Entity>,
        changed_image_handles: &HashSet<&AssetId<Image>>,
    ) -> bool;
}

impl NormalizedRenderTargetExt for NormalizedRenderTarget {
    fn get_render_target_info<'a>(
        &self,
        resolutions: impl IntoIterator<Item = (Entity, &'a Window)>,
        images: &Assets<Image>,
        manual_texture_views: &ManualTextureViews,
    ) -> Result<RenderTargetInfo, MissingRenderTargetInfoError> {
        match self {
            NormalizedRenderTarget::Window(window_ref) => resolutions
                .into_iter()
                .find(|(entity, _)| *entity == window_ref.entity())
                .map(|(_, window)| RenderTargetInfo {
                    physical_size: window.physical_size(),
                    scale_factor: window.resolution.scale_factor(),
                })
                .ok_or(MissingRenderTargetInfoError::Window {
                    window: window_ref.entity(),
                }),
            NormalizedRenderTarget::Image(image_target) => images
                .get(&image_target.handle)
                .map(|image| RenderTargetInfo {
                    physical_size: image.size(),
                    scale_factor: image_target.scale_factor,
                })
                .ok_or(MissingRenderTargetInfoError::Image {
                    image: image_target.handle.id(),
                }),
            NormalizedRenderTarget::TextureView(id) => manual_texture_views
                .get(id)
                .map(|tex| RenderTargetInfo {
                    physical_size: tex.size,
                    scale_factor: 1.0,
                })
                .ok_or(MissingRenderTargetInfoError::TextureView { texture_view: *id }),
            NormalizedRenderTarget::None { width, height } => Ok(RenderTargetInfo {
                physical_size: uvec2(*width, *height),
                scale_factor: 1.0,
            }),
        }
    }

    // Check if this render target is contained in the given changed windows or images.
    fn is_changed(
        &self,
        changed_window_ids: &HashSet<Entity>,
        changed_image_handles: &HashSet<&AssetId<Image>>,
    ) -> bool {
        match self {
            NormalizedRenderTarget::Window(window_ref) => {
                changed_window_ids.contains(&window_ref.entity())
            }
            NormalizedRenderTarget::Image(image_target) => {
                changed_image_handles.contains(&image_target.handle.id())
            }
            NormalizedRenderTarget::TextureView(_) => true,
            NormalizedRenderTarget::None { .. } => false,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MissingRenderTargetInfoError {
    #[error("RenderTarget::Window missing ({window:?}): Make sure the provided entity has a Window component.")]
    Window { window: Entity },
    #[error("RenderTarget::Image missing ({image:?}): Make sure the Image's usages include RenderAssetUsages::MAIN_WORLD.")]
    Image { image: AssetId<Image> },
    #[error("RenderTarget::TextureView missing ({texture_view:?}): make sure the texture view handle was not removed.")]
    TextureView {
        texture_view: ManualTextureViewHandle,
    },
}

/// System in charge of updating a [`Camera`] when its window or projection changes.
///
/// The system detects window creation, resize, and scale factor change events to update the camera
/// [`Projection`] if needed.
///
/// ## World Resources
///
/// [`Res<Assets<Image>>`](Assets<Image>) -- For cameras that render to an image, this resource is used to
/// inspect information about the render target. This system will not access any other image assets.
///
/// [`OrthographicProjection`]: bevy_camera::OrthographicProjection
/// [`PerspectiveProjection`]: bevy_camera::PerspectiveProjection
pub fn camera_system(
    mut window_resized_reader: MessageReader<WindowResized>,
    mut window_created_reader: MessageReader<WindowCreated>,
    mut window_scale_factor_changed_reader: MessageReader<WindowScaleFactorChanged>,
    mut image_asset_event_reader: MessageReader<AssetEvent<Image>>,
    primary_window: Query<Entity, With<PrimaryWindow>>,
    windows: Query<(Entity, &Window)>,
    images: Res<Assets<Image>>,
    manual_texture_views: Res<ManualTextureViews>,
    mut cameras: Query<(&mut Camera, &RenderTarget, &mut Projection)>,
) -> Result<(), BevyError> {
    let primary_window = primary_window.iter().next();

    let mut changed_window_ids = <HashSet<_>>::default();
    changed_window_ids.extend(window_created_reader.read().map(|event| event.window));
    changed_window_ids.extend(window_resized_reader.read().map(|event| event.window));
    let scale_factor_changed_window_ids: HashSet<_> = window_scale_factor_changed_reader
        .read()
        .map(|event| event.window)
        .collect();
    changed_window_ids.extend(scale_factor_changed_window_ids.clone());

    let changed_image_handles: HashSet<&AssetId<Image>> = image_asset_event_reader
        .read()
        .filter_map(|event| match event {
            AssetEvent::Modified { id } | AssetEvent::Added { id } => Some(id),
            _ => None,
        })
        .collect();

    for (mut camera, render_target, mut camera_projection) in &mut cameras {
        let mut viewport_size = camera
            .viewport
            .as_ref()
            .map(|viewport| viewport.physical_size);

        if let Some(normalized_target) = render_target.normalize(primary_window)
            && (normalized_target.is_changed(&changed_window_ids, &changed_image_handles)
                || camera.is_added()
                || camera_projection.is_changed()
                || camera.computed.old_viewport_size != viewport_size
                || camera.computed.old_sub_camera_view != camera.sub_camera_view)
        {
            let new_computed_target_info = normalized_target.get_render_target_info(
                windows,
                &images,
                &manual_texture_views,
            )?;
            // Check for the scale factor changing, and resize the viewport if needed.
            // This can happen when the window is moved between monitors with different DPIs.
            // Without this, the viewport will take a smaller portion of the window moved to
            // a higher DPI monitor.
            if normalized_target.is_changed(&scale_factor_changed_window_ids, &HashSet::default())
                && let Some(old_scale_factor) = camera
                    .computed
                    .target_info
                    .as_ref()
                    .map(|info| info.scale_factor)
            {
                let resize_factor = new_computed_target_info.scale_factor / old_scale_factor;
                if let Some(ref mut viewport) = camera.viewport {
                    let resize = |vec: UVec2| (vec.as_vec2() * resize_factor).as_uvec2();
                    viewport.physical_position = resize(viewport.physical_position);
                    viewport.physical_size = resize(viewport.physical_size);
                    viewport_size = Some(viewport.physical_size);
                }
            }
            // This check is needed because when changing WindowMode to Fullscreen, the viewport may have invalid
            // arguments due to a sudden change on the window size to a lower value.
            // If the size of the window is lower, the viewport will match that lower value.
            if let Some(viewport) = &mut camera.viewport {
                viewport.clamp_to_size(new_computed_target_info.physical_size);
            }
            camera.computed.target_info = Some(new_computed_target_info);
            if let Some(size) = camera.logical_viewport_size()
                && size.x != 0.0
                && size.y != 0.0
            {
                camera_projection.update(size.x, size.y);
                camera.computed.clip_from_view = match &camera.sub_camera_view {
                    Some(sub_view) => camera_projection.get_clip_from_view_for_sub(sub_view),
                    None => camera_projection.get_clip_from_view(),
                }
            }
        }

        if camera.computed.old_viewport_size != viewport_size {
            camera.computed.old_viewport_size = viewport_size;
        }

        if camera.computed.old_sub_camera_view != camera.sub_camera_view {
            camera.computed.old_sub_camera_view = camera.sub_camera_view;
        }
    }
    Ok(())
}

#[derive(Component, Debug)]
pub struct ExtractedCamera {
    pub target: Option<NormalizedRenderTarget>,
    pub physical_viewport_size: Option<UVec2>,
    pub physical_target_size: Option<UVec2>,
    pub viewport: Option<Viewport>,
    pub render_graph: InternedRenderSubGraph,
    pub order: isize,
    pub output_mode: CameraOutputMode,
    pub msaa_writeback: MsaaWriteback,
    pub clear_color: ClearColorConfig,
    pub sorted_camera_index_for_target: usize,
    pub exposure: f32,
    pub hdr: bool,
}

/// Camera component specifying a mip bias to apply when sampling from material textures.
///
/// Often used in conjunction with antialiasing post-process effects to reduce textures blurriness.
#[derive(Component, Clone)]
pub struct MipBias(pub f32);

impl Default for MipBias {
    fn default() -> Self {
        Self(-1.0)
    }
}
