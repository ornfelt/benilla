#![cfg_attr(docsrs, feature(doc_cfg))]
#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://bevy.org/assets/icon.png",
    html_favicon_url = "https://bevy.org/assets/icon.png"
)]

//! Provides 2D sprite functionality.

extern crate alloc;

mod text2d;
mod texture_slice;

/// The sprite prelude.
///
/// This includes the most common types in this crate, re-exported for your convenience.
pub mod prelude {
    #[cfg(feature = "bevy_text")]
    #[doc(hidden)]
    pub use crate::text2d::Text2d;
    #[doc(hidden)]
    pub use crate::texture_slice::{BorderRect, SliceScaleMode, TextureSlicer};
}

use bevy_asset::Assets;
use bevy_camera::{
    primitives::{Aabb, MeshAabb},
    visibility::NoFrustumCulling,
    visibility::VisibilitySystems,
};
use bevy_mesh::{Mesh, Mesh2d};
pub use text2d::*;
pub use texture_slice::*;

use bevy_app::prelude::*;
use bevy_asset::prelude::AssetChanged;
use bevy_camera::visibility::NoAutoAabb;
use bevy_ecs::prelude::*;
use bevy_image::TextureAtlasPlugin;

/// Adds support for 2D sprites.
#[derive(Default)]
pub struct SpritePlugin;

impl Plugin for SpritePlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<TextureAtlasPlugin>() {
            app.add_plugins(TextureAtlasPlugin);
        }
        app.add_systems(
            PostUpdate,
            calculate_bounds_2d.in_set(VisibilitySystems::CalculateBounds),
        );

        #[cfg(feature = "bevy_text")]
        app.add_systems(
            PostUpdate,
            (
                bevy_text::detect_text_needs_rerender::<Text2d>,
                update_text2d_layout
                    .after(bevy_camera::CameraUpdateSystems)
                    .after(bevy_text::free_unused_font_atlases_system),
                calculate_bounds_text2d.in_set(VisibilitySystems::CalculateBounds),
            )
                .chain()
                .in_set(bevy_text::Text2dUpdateSystems)
                .after(bevy_app::AnimationSystems),
        );
    }
}

/// System calculating and inserting an [`Aabb`] component to entities with a `Mesh2d` component
/// and without a [`NoFrustumCulling`] component (no entity has a `Sprite`, whose bounds it also
/// computed).
///
/// Used in system set [`VisibilitySystems::CalculateBounds`].
pub fn calculate_bounds_2d(
    mut commands: Commands,
    meshes: Res<Assets<Mesh>>,
    new_mesh_aabb: Query<
        (Entity, &Mesh2d),
        (
            Without<Aabb>,
            Without<NoFrustumCulling>,
            Without<NoAutoAabb>,
        ),
    >,
    mut update_mesh_aabb: Query<
        (&Mesh2d, &mut Aabb),
        (
            Or<(AssetChanged<Mesh2d>, Changed<Mesh2d>)>,
            Without<NoFrustumCulling>,
            Without<NoAutoAabb>,
        ),
    >,
) {
    // New meshes require inserting a component
    for (entity, mesh_handle) in &new_mesh_aabb {
        if let Some(mesh) = meshes.get(mesh_handle)
            && let Some(aabb) = mesh.compute_aabb()
        {
            commands.entity(entity).try_insert(aabb);
        }
    }

    // Updated meshes can take the fast path with parallel component mutation
    update_mesh_aabb
        .par_iter_mut()
        .for_each(|(mesh_handle, mut aabb)| {
            if let Some(new_aabb) = meshes.get(mesh_handle).and_then(MeshAabb::compute_aabb) {
                aabb.set_if_neq(new_aabb);
            }
        });
}
