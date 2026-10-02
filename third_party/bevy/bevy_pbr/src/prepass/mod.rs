use crate::ShadowView;
use bevy_app::{App, Plugin, PreUpdate};
use bevy_camera::{Camera, Camera3d};
use bevy_core_pipeline::prepass::PreviousViewData;
use bevy_ecs::prelude::*;
use bevy_math::{Affine3A, Mat4};
use bevy_mesh::Mesh3d;
use bevy_transform::prelude::GlobalTransform;

/// Sets up the prepasses for a material.
pub struct PrepassPlugin;

impl Plugin for PrepassPlugin {
    fn build(&self, app: &mut App) {
        let no_prepass_plugin_loaded = app
            .world()
            .get_resource::<AnyPrepassPluginLoaded>()
            .is_none();

        // The binned phase plugins and the prepass pipelines only reached the RenderApp.
        if no_prepass_plugin_loaded {
            app.insert_resource(AnyPrepassPluginLoaded)
                // At the start of each frame, last frame's GlobalTransforms become this frame's PreviousGlobalTransforms
                // and last frame's view projection matrices become this frame's PreviousViewProjections
                .add_systems(
                    PreUpdate,
                    (
                        update_mesh_previous_global_transforms,
                        update_previous_view_data,
                    ),
                );
        }
    }
}

#[derive(Resource)]
struct AnyPrepassPluginLoaded;

pub fn update_previous_view_data(
    mut commands: Commands,
    query: Query<(Entity, &Camera, &GlobalTransform), Or<(With<Camera3d>, With<ShadowView>)>>,
) {
    for (entity, camera, camera_transform) in &query {
        let world_from_view = camera_transform.affine();
        let view_from_world = Mat4::from(world_from_view.inverse());
        let view_from_clip = camera.clip_from_view().inverse();

        commands.entity(entity).try_insert(PreviousViewData {
            view_from_world,
            clip_from_world: camera.clip_from_view() * view_from_world,
            clip_from_view: camera.clip_from_view(),
            world_from_clip: Mat4::from(world_from_view) * view_from_clip,
            view_from_clip,
        });
    }
}

#[derive(Component, PartialEq, Default)]
pub struct PreviousGlobalTransform(pub Affine3A);

type PreviousMeshFilter = With<Mesh3d>;

pub fn update_mesh_previous_global_transforms(
    mut commands: Commands,
    views: Query<&Camera, Or<(With<Camera3d>, With<ShadowView>)>>,
    new_meshes: Query<
        (Entity, &GlobalTransform),
        (PreviousMeshFilter, Without<PreviousGlobalTransform>),
    >,
    mut meshes: Query<(&GlobalTransform, &mut PreviousGlobalTransform), PreviousMeshFilter>,
) {
    let should_run = views.iter().any(|camera| camera.is_active);

    if should_run {
        for (entity, transform) in &new_meshes {
            let new_previous_transform = PreviousGlobalTransform(transform.affine());
            commands.entity(entity).try_insert(new_previous_transform);
        }
        meshes.par_iter_mut().for_each(|(transform, mut previous)| {
            previous.set_if_neq(PreviousGlobalTransform(transform.affine()));
        });
    }
}
