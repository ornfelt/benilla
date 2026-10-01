use core::marker::PhantomData;

use crate::{
    dynamics::{
        joints::EntityConstraint,
        solver::{
            islands::{BodyIslandNode, IslandId, PhysicsIslands},
            joint_graph::{JointGraph, JointGraphEdge},
        },
    },
    prelude::{ContactGraph, PhysicsSchedule, PhysicsStepSystems, WakeIslands},
};
use bevy::{
    ecs::{
        component::ComponentId, entity_disabling::Disabled, lifecycle::HookContext,
        query::QueryFilter, world::DeferredWorld,
    },
    prelude::*,
};

/// A plugin that manages the [`JointGraph`] for a specific [joint] type.
///
/// [joint]: crate::dynamics::joints
pub struct JointGraphPlugin<T: Component + EntityConstraint<2>>(PhantomData<T>);

impl<T: Component + EntityConstraint<2>> Default for JointGraphPlugin<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

/// A component that holds the [`ComponentId`] of the [joint] component on this entity, if any.
///
/// [joint]: crate::dynamics::joints
#[derive(Component, Clone, Debug, Default, PartialEq, Reflect)]
pub struct JointComponentId(Option<ComponentId>);

#[derive(Resource, Default)]
struct JointGraphPluginInitialized;

impl<T: Component + EntityConstraint<2>> Plugin for JointGraphPlugin<T> {
    fn build(&self, app: &mut App) {
        let already_initialized = app
            .world()
            .is_resource_added::<JointGraphPluginInitialized>();

        app.init_resource::<JointGraph>();
        app.init_resource::<JointGraphPluginInitialized>();

        // Automatically add the `JointComponentId` component when the joint is added.
        app.register_required_components::<T, JointComponentId>();

        // Register hooks for adding and removing joints.
        app.world_mut()
            .register_component_hooks::<T>()
            .on_add(on_add_joint)
            .on_remove(on_remove_joint);

        // Add the joint to the joint graph when it is added.
        app.add_observer(add_joint_to_graph::<T, Add, T, With<JointComponentId>>);

        // Remove the joint from the joint graph when it is removed.
        app.add_observer(remove_joint_from_graph::<Remove, T>);

        if !already_initialized {
            // Remove the joint from the joint graph when it is disabled.
            app.add_observer(remove_joint_from_graph::<Add, Disabled>);
        }

        // Add the joint back to the joint graph when `Disabled` is removed.
        app.add_observer(
            add_joint_to_graph::<
                T,
                Remove,
                Disabled,
                (
                    With<JointComponentId>,
                    Or<(With<Disabled>, Without<Disabled>)>,
                ),
            >,
        );

        app.add_systems(
            PhysicsSchedule,
            on_change_joint_entities::<T>
                .in_set(PhysicsStepSystems::First)
                .ambiguous_with(PhysicsStepSystems::First),
        );
    }
}

fn add_joint_to_graph<
    T: Component + EntityConstraint<2>,
    E: EntityEvent,
    B: Bundle,
    F: QueryFilter,
>(
    trigger: On<E, B>,
    query: Query<&T, F>,
    mut commands: Commands,
    mut body_islands: Query<&mut BodyIslandNode, Or<(With<Disabled>, Without<Disabled>)>>,
    mut contact_graph: ResMut<ContactGraph>,
    mut joint_graph: ResMut<JointGraph>,
    mut islands: Option<ResMut<PhysicsIslands>>,
) {
    let entity = trigger.event_target();

    let Ok(joint) = query.get(entity) else {
        return;
    };

    let [body1, body2] = joint.entities();

    // Add the joint to the joint graph.
    let joint_edge = JointGraphEdge::new(entity, body1, body2, false);
    let joint_id = joint_graph.add_joint(body1, body2, joint_edge);

    // Link the joint to an island.
    if let Some(islands) = &mut islands {
        let island = islands.add_joint(
            joint_id,
            &mut body_islands,
            &mut contact_graph,
            &mut joint_graph,
        );

        // Wake up the island if it was sleeping.
        if let Some(island) = island
            && island.is_sleeping
        {
            commands.queue(WakeIslands(vec![island.id]));
        }
    }
}

fn remove_joint_from_graph<E: EntityEvent, B: Bundle>(
    trigger: On<E, B>,
    mut commands: Commands,
    mut body_islands: Query<&mut BodyIslandNode, Or<(With<Disabled>, Without<Disabled>)>>,
    contact_graph: ResMut<ContactGraph>,
    mut joint_graph: ResMut<JointGraph>,
    mut islands: Option<ResMut<PhysicsIslands>>,
) {
    let entity = trigger.event_target();

    let Some(joint) = joint_graph.get(entity) else {
        return;
    };

    // Remove the joint from the island.
    if let Some(islands) = &mut islands
        && let Some(island) = islands.remove_joint(
            joint.id,
            &mut body_islands,
            &contact_graph,
            &mut joint_graph,
        )
    {
        // Wake up the island if it was sleeping.
        if island.is_sleeping {
            commands.queue(WakeIslands(vec![island.id]));
        }
    }

    // Remove the joint from the joint graph.
    joint_graph.remove_joint(entity);
}

fn on_add_joint(mut world: DeferredWorld, ctx: HookContext) {
    let entity = ctx.entity;
    let component_id = ctx.component_id;

    let mut joint = world.get_mut::<JointComponentId>(entity).unwrap();
    let old_joint = joint.0;

    // Update the joint component with the new component ID.
    joint.0 = Some(component_id);

    if let Some(old_joint) = old_joint {
        // Joint already exists, remove the old one.
        world.commands().entity(entity).remove_by_id(old_joint);
    }
}

fn on_remove_joint(mut world: DeferredWorld, ctx: HookContext) {
    let entity = ctx.entity;
    let component_id = ctx.component_id;

    // Remove the `JointComponentId` from the entity unless the component ID
    // was changed, implying that the joint is being replaced by another one.
    if let Some(mut joint) = world.get_mut::<JointComponentId>(entity)
        && joint.0 == Some(component_id)
    {
        joint.0 = None;

        // Remove the joint component.
        world
            .commands()
            .entity(entity)
            .try_remove::<JointComponentId>();
    }
}

/// Update the joint graph when the entities of a joint change.
fn on_change_joint_entities<T: Component + EntityConstraint<2>>(
    query: Query<(Entity, &T), Changed<T>>,
    mut commands: Commands,
    mut body_islands: Query<&mut BodyIslandNode, Or<(With<Disabled>, Without<Disabled>)>>,
    mut joint_graph: ResMut<JointGraph>,
    mut contact_graph: ResMut<ContactGraph>,
    mut islands: Option<ResMut<PhysicsIslands>>,
) {
    let mut islands_to_wake: Vec<IslandId> = Vec::new();

    for (entity, joint) in &query {
        let [body1, body2] = joint.entities();
        let Some(old_edge) = joint_graph.get(entity) else {
            continue;
        };

        if body1 != old_edge.body1 || body2 != old_edge.body2 {
            // Remove the joint from the island.
            if let Some(islands) = &mut islands
                && let Some(island) = islands.remove_joint(
                    old_edge.id,
                    &mut body_islands,
                    &contact_graph,
                    &mut joint_graph,
                )
            {
                // Wake up the island if it was sleeping.
                if island.is_sleeping {
                    islands_to_wake.push(island.id);
                }
            }

            // Remove the old joint edge.
            if let Some(mut edge) = joint_graph.remove_joint(entity) {
                // Update the edge with the new bodies.
                edge.body1 = body1;
                edge.body2 = body2;

                // Add the joint edge.
                let joint_id = joint_graph.add_joint(body1, body2, edge);

                // Link the joint to an island.
                if let Some(islands) = &mut islands {
                    islands.add_joint(
                        joint_id,
                        &mut body_islands,
                        &mut contact_graph,
                        &mut joint_graph,
                    );
                }
            }
        }
    }

    if !islands_to_wake.is_empty() {
        islands_to_wake.sort_unstable();
        islands_to_wake.dedup();

        // Wake up the islands that were previously sleeping.
        commands.queue(WakeIslands(islands_to_wake));
    }
}

// TODO: Tests
