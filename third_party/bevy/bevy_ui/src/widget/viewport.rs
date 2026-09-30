use crate::{ComputedNode, Node};
use bevy_asset::Assets;
use bevy_camera::RenderTarget;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::{Changed, Or},
    reflect::ReflectComponent,
    system::{Query, ResMut},
};
use bevy_image::{Image, ToExtents};
use bevy_math::UVec2;
use bevy_reflect::Reflect;

/// Component used to render a [`RenderTarget`]  to a node.
///
/// # See Also
///
/// [`update_viewport_render_target_size`]
#[derive(Component, Debug, Clone, Copy, Reflect)]
#[reflect(Component, Debug)]
#[require(Node)]
pub struct ViewportNode {
    /// The entity representing the [`Camera`] associated with this viewport.
    ///
    /// Note: Removing the [`ViewportNode`] component will not despawn this
    /// entity.
    ///
    /// Note: Despawning the camera entity will leave a viewport node with an
    /// invalid camera.
    pub camera: Entity,
}

impl ViewportNode {
    /// Creates a new [`ViewportNode`] with a given `camera`.
    #[inline]
    pub const fn new(camera: Entity) -> Self {
        Self { camera }
    }
}

/// Updates the size of the associated render target for viewports when the node size changes.
pub fn update_viewport_render_target_size(
    viewport_query: Query<
        (&ViewportNode, &ComputedNode),
        Or<(Changed<ComputedNode>, Changed<ViewportNode>)>,
    >,
    camera_query: Query<&RenderTarget>,
    mut images: ResMut<Assets<Image>>,
) {
    for (viewport, computed_node) in &viewport_query {
        let Ok(render_target) = camera_query.get(viewport.camera) else {
            continue;
        };
        let size = computed_node.size();

        let Some(image_handle) = render_target.as_image() else {
            continue;
        };
        let size = size.as_uvec2().max(UVec2::ONE).to_extents();
        images.get_mut(image_handle).unwrap().resize(size);
    }
}
