use crate::{EntitiesNeedingSpecialization, Material, MeshMaterial3d};
use bevy_ecs::prelude::*;
use bevy_light::NotShadowCaster;
use bevy_render::texture::DepthAttachment;

// These will be extracted in the material extraction, which will also clear the needs_specialization
// collection.
pub fn check_light_entities_needing_specialization<M: Material>(
    needs_specialization: Query<Entity, (With<MeshMaterial3d<M>>, Changed<NotShadowCaster>)>,
    mesh_materials: Query<Entity, With<MeshMaterial3d<M>>>,
    mut entities_needing_specialization: ResMut<EntitiesNeedingSpecialization<M>>,
    mut removed_components: RemovedComponents<NotShadowCaster>,
) {
    for entity in &needs_specialization {
        entities_needing_specialization.push(entity);
    }

    for removed in removed_components.read() {
        // Only require specialization if the entity still exists.
        if mesh_materials.contains(removed) {
            entities_needing_specialization.entities.push(removed);
        }
    }
}

#[derive(Component)]
pub struct ShadowView {
    pub depth_attachment: DepthAttachment,
    pub pass_name: String,
}
