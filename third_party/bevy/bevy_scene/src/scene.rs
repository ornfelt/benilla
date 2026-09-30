use core::any::TypeId;

use crate::reflect_utils::clone_reflect_value;
use crate::SceneSpawnError;
use bevy_asset::Asset;
use bevy_ecs::{
    component::ComponentCloneBehavior,
    entity::{Entity, EntityHashMap, SceneEntityMapper},
    entity_disabling::DefaultQueryFilters,
    reflect::{AppTypeRegistry, ReflectComponent, ReflectResource},
    relationship::RelationshipHookMode,
    world::World,
};
use bevy_reflect::TypePath;

/// A composition of [`World`] objects.
///
/// To spawn a scene, add the [`SceneRoot`](crate::components::SceneRoot) component to an entity.
#[derive(Asset, TypePath, Debug)]
pub struct Scene {
    /// The world of the scene, containing its entities and resources.
    pub world: World,
}

impl Scene {
    /// Creates a new scene with the given world.
    pub fn new(world: World) -> Self {
        Self { world }
    }

    /// Write the entities and their corresponding components to the given world.
    ///
    /// This method will return a [`SceneSpawnError`] if a type either is not registered in the
    /// provided [`AppTypeRegistry`] or doesn't reflect the [`Component`](bevy_ecs::component::Component) trait.
    pub fn write_to_world_with(
        &self,
        world: &mut World,
        entity_map: &mut EntityHashMap<Entity>,
        type_registry: &AppTypeRegistry,
    ) -> Result<(), SceneSpawnError> {
        let type_registry = type_registry.read();

        let self_dqf_id = self
            .world
            .components()
            .get_resource_id(TypeId::of::<DefaultQueryFilters>());

        // Resources archetype
        for (component_id, resource_data) in self.world.storages().resources.iter() {
            if Some(component_id) == self_dqf_id {
                continue;
            }
            if !resource_data.is_present() {
                continue;
            }

            let component_info = self
                .world
                .components()
                .get_info(component_id)
                .expect("component_ids in archetypes should have ComponentInfo");

            let type_id = component_info
                .type_id()
                .expect("reflected resources must have a type_id");

            let registration =
                type_registry
                    .get(type_id)
                    .ok_or_else(|| SceneSpawnError::UnregisteredType {
                        std_type_name: component_info.name(),
                    })?;
            let reflect_resource = registration.data::<ReflectResource>().ok_or_else(|| {
                SceneSpawnError::UnregisteredResource {
                    type_path: registration.type_info().type_path().to_string(),
                }
            })?;
            reflect_resource.copy(&self.world, world, &type_registry);
        }

        // Ensure that all scene entities have been allocated in the destination
        // world before handling components that may contain references that need mapping.
        for archetype in self.world.archetypes().iter() {
            for scene_entity in archetype.entities() {
                entity_map
                    .entry(scene_entity.id())
                    .or_insert_with(|| world.spawn_empty().id());
            }
        }

        for archetype in self.world.archetypes().iter() {
            for scene_entity in archetype.entities() {
                let entity = *entity_map
                    .get(&scene_entity.id())
                    .expect("should have previously spawned an entity");

                for component_id in archetype.iter_components() {
                    let component_info = self
                        .world
                        .components()
                        .get_info(component_id)
                        .expect("component_ids in archetypes should have ComponentInfo");

                    if matches!(
                        *component_info.clone_behavior(),
                        ComponentCloneBehavior::Ignore
                    ) {
                        continue;
                    }

                    let registration = type_registry
                        .get(component_info.type_id().unwrap())
                        .ok_or_else(|| SceneSpawnError::UnregisteredType {
                            std_type_name: component_info.name(),
                        })?;
                    let reflect_component =
                        registration.data::<ReflectComponent>().ok_or_else(|| {
                            SceneSpawnError::UnregisteredComponent {
                                type_path: registration.type_info().type_path().to_string(),
                            }
                        })?;

                    let Some(component) = reflect_component
                        .reflect(self.world.entity(scene_entity.id()))
                        .map(|component| {
                            clone_reflect_value(component.as_partial_reflect(), registration)
                        })
                    else {
                        continue;
                    };

                    // If this component references entities in the scene,
                    // update them to the entities in the world.
                    SceneEntityMapper::world_scope(entity_map, world, |world, mapper| {
                        reflect_component.apply_or_insert_mapped(
                            &mut world.entity_mut(entity),
                            component.as_partial_reflect(),
                            &type_registry,
                            mapper,
                            RelationshipHookMode::Skip,
                        );
                    });
                }
            }
        }

        Ok(())
    }
}
