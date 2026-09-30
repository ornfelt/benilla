//! Handles generic collider backend logic, like initializing colliders and AABBs and updating related components.
//!
//! See [`ColliderBackendPlugin`].

use core::marker::PhantomData;

use crate::{
    collision::collider::EnlargedAabb,
    physics_transform::{PhysicsTransformConfig, PhysicsTransformSystems, init_physics_transform},
    prelude::*,
};
use bevy::{
    ecs::{intern::Interned, schedule::ScheduleLabel},
    prelude::*,
};
use mass_properties::{MassPropertySystems, components::RecomputeMassProperties};

/// A plugin for handling generic collider backend logic.
///
/// - Initializes colliders.
/// - Updates [`ColliderAabb`]s.
/// - Updates collider scale based on `Transform` scale.
/// - Updates [`ColliderMassProperties`].
///
/// This plugin should typically be used together with the [`ColliderHierarchyPlugin`].
///
/// # Custom Collision Backends
///
/// By default, [`PhysicsPlugins`] adds this plugin for the [`Collider`] component.
/// You can also create custom collider backends by implementing the [`AnyCollider`]
/// and [`ScalableCollider`] traits for a type.
///
/// To use a custom collider backend, simply add the [`ColliderBackendPlugin`] with your collider type:
///
/// ```no_run
/// use avian3d::prelude::*;
/// use bevy::prelude::*;
/// #
/// # type MyCollider = Collider;
///
/// fn main() {
///     App::new()
///         .add_plugins((
///             DefaultPlugins,
///             PhysicsPlugins::default(),
///             // MyCollider must implement AnyCollider and ScalableCollider.
///             ColliderBackendPlugin::<MyCollider>::default(),
///             // To enable collision detection for the collider,
///             // we also need to add the NarrowPhasePlugin for it.
///             NarrowPhasePlugin::<MyCollider>::default(),
///         ))
///         // ...your other plugins, systems and resources
///         .run();
/// }
/// ```
///
/// Assuming you have implemented the required traits correctly,
/// it should now work with the rest of the engine just like normal [`Collider`]s!
///
/// **Note**: [Spatial queries](spatial_query) are not supported for custom colliders yet.
pub struct ColliderBackendPlugin<C: ScalableCollider> {
    schedule: Interned<dyn ScheduleLabel>,
    _phantom: PhantomData<C>,
}

impl<C: ScalableCollider> ColliderBackendPlugin<C> {
    /// Creates a [`ColliderBackendPlugin`] with the schedule that is used for running the [`PhysicsSchedule`].
    ///
    /// The default schedule is `FixedPostUpdate`.
    pub fn new(schedule: impl ScheduleLabel) -> Self {
        Self {
            schedule: schedule.intern(),
            _phantom: PhantomData,
        }
    }
}

impl<C: ScalableCollider> Default for ColliderBackendPlugin<C> {
    fn default() -> Self {
        Self {
            schedule: FixedPostUpdate.intern(),
            _phantom: PhantomData,
        }
    }
}

impl<C: ScalableCollider> Plugin for ColliderBackendPlugin<C> {
    fn build(&self, app: &mut App) {
        // Register required components for the collider type.
        let _ = app.try_register_required_components_with::<C, Position>(|| Position::PLACEHOLDER);
        let _ = app.try_register_required_components_with::<C, Rotation>(|| Rotation::PLACEHOLDER);
        let _ = app.try_register_required_components::<C, ColliderMarker>();
        let _ = app.try_register_required_components::<C, ColliderAabb>();
        let _ = app.try_register_required_components::<C, EnlargedAabb>();
        let _ = app.try_register_required_components::<C, CollisionLayers>();
        let _ = app.try_register_required_components::<C, ColliderDensity>();
        let _ = app.try_register_required_components::<C, ColliderMassProperties>();

        // Make sure the necessary resources are initialized.
        app.init_resource::<PhysicsTransformConfig>();
        app.init_resource::<NarrowPhaseConfig>();
        app.init_resource::<PhysicsLengthUnit>();

        let hooks = app.world_mut().register_component_hooks::<C>();

        // Initialize missing components for colliders.
        hooks
            .on_add(|mut world, ctx| {
                // Initialize the global physics transform for the collider.
                // Avoid doing this twice for rigid bodies added at the same time.
                // TODO: The special case for rigid bodies is a bit of a hack here.
                if !world.entity(ctx.entity).contains::<RigidBody>() {
                    init_physics_transform(&mut world, &ctx);
                }
            })
            .on_insert(|mut world, ctx| {
                let scale = world
                    .entity(ctx.entity)
                    .get::<GlobalTransform>()
                    .map(|gt| gt.scale())
                    .unwrap_or_default();

                let mut entity_mut = world.entity_mut(ctx.entity);

                // Make sure the collider is initialized with the correct scale.
                // This overwrites the scale set by the constructor, but that one is
                // meant to be only changed after initialization.
                entity_mut
                    .get_mut::<C>()
                    .unwrap()
                    .set_scale(scale.adjust_precision(), 10);

                let collider = entity_mut.get::<C>().unwrap();

                let density = entity_mut
                    .get::<ColliderDensity>()
                    .copied()
                    .unwrap_or_default();

                let mass_properties = if entity_mut.get::<Sensor>().is_some() {
                    MassProperties::ZERO
                } else {
                    collider.mass_properties(density.0)
                };

                if let Some(mut collider_mass_properties) =
                    entity_mut.get_mut::<ColliderMassProperties>()
                {
                    *collider_mass_properties = ColliderMassProperties::from(mass_properties);
                }
            });

        // Register a component hook that removes `ColliderMarker` components
        // and updates rigid bodies when their collider is removed.
        app.world_mut()
            .register_component_hooks::<C>()
            .on_remove(|mut world, ctx| {
                // Remove the `ColliderMarker` associated with the collider.
                // TODO: If the same entity had multiple *different* types of colliders, this would
                //       get removed even if just one collider was removed. This is a very niche edge case though.
                world
                    .commands()
                    .entity(ctx.entity)
                    .try_remove::<ColliderMarker>();

                let entity_ref = world.entity_mut(ctx.entity);

                // Get the rigid body entity that the collider is attached to.
                let Some(ColliderOf { body }) = entity_ref.get::<ColliderOf>().copied() else {
                    return;
                };

                // Queue the rigid body for a mass property update.
                world
                    .commands()
                    .entity(body)
                    .try_insert(RecomputeMassProperties);
            });

        // When the `Sensor` component is added to a collider, queue its rigid body for a mass property update.
        app.add_observer(
            |trigger: On<Add, Sensor>,
             mut commands: Commands,
             query: Query<(&ColliderMassProperties, &ColliderOf)>| {
                if let Ok((collider_mass_properties, &ColliderOf { body })) =
                    query.get(trigger.entity)
                {
                    // If the collider mass properties are zero, there is nothing to subtract.
                    if *collider_mass_properties == ColliderMassProperties::ZERO {
                        return;
                    }

                    // Queue the rigid body for a mass property update.
                    if let Ok(mut entity_commands) = commands.get_entity(body) {
                        entity_commands.insert(RecomputeMassProperties);
                    }
                }
            },
        );

        // When the `Sensor` component is removed from a collider, update its mass properties.
        app.add_observer(
            |trigger: On<Remove, Sensor>,
             mut collider_query: Query<(
                Ref<C>,
                &ColliderDensity,
                &mut ColliderMassProperties,
            )>| {
                if let Ok((collider, density, mut collider_mass_properties)) =
                    collider_query.get_mut(trigger.entity)
                {
                    // Update collider mass props.
                    *collider_mass_properties =
                        ColliderMassProperties::from(collider.mass_properties(density.0));
                }
            },
        );

        app.add_systems(
            self.schedule,
            (
                update_collider_scale::<C>
                    .in_set(PhysicsSystems::Prepare)
                    .after(PhysicsTransformSystems::TransformToPosition),
                update_collider_mass_properties::<C>
                    .in_set(MassPropertySystems::UpdateColliderMassProperties),
            )
                .chain(),
        );

        app.add_systems(
            Update,
            (
                init_collider_constructors,
                init_collider_constructor_hierarchies,
            ),
        );
    }
}

/// A marker component for colliders. Inserted and removed automatically.
///
/// This is useful for filtering collider entities regardless of the [collider backend](ColliderBackendPlugin).
#[derive(Reflect, Component, Clone, Copy, Debug, Default)]
#[reflect(Component, Debug, Default)]
pub struct ColliderMarker;

// Stand-in for the system that built each `ColliderConstructor`'s collider; its `Commands` keeps
// the sync point after it.
fn init_collider_constructors(_commands: Commands) {}

// Stand-in for the system that built the colliders of each `ColliderConstructorHierarchy`; its
// `Commands` keeps the sync point after it.
fn init_collider_constructor_hierarchies(_commands: Commands) {}

/// Updates the scale of colliders based on [`Transform`] scale.
#[allow(clippy::type_complexity)]
pub fn update_collider_scale<C: ScalableCollider>(
    mut colliders: ParamSet<(
        // Root bodies
        Query<(&Transform, &mut C), (Without<ChildOf>, Or<(Changed<Transform>, Changed<C>)>)>,
        // Child colliders
        Query<
            (&ColliderTransform, &mut C),
            (With<ChildOf>, Or<(Changed<ColliderTransform>, Changed<C>)>),
        >,
    )>,
    config: Res<PhysicsTransformConfig>,
) {
    if config.transform_to_collider_scale {
        // Update collider scale for root bodies
        for (transform, mut collider) in &mut colliders.p0() {
            let scale = transform.scale.adjust_precision();
            if scale != collider.scale() {
                // TODO: Support configurable subdivision count for shapes that
                //       can't be represented without approximations after scaling.
                collider.set_scale(scale, 10);
            }
        }
    }
    // Update collider scale for child colliders
    for (collider_transform, mut collider) in &mut colliders.p1() {
        if collider_transform.scale != collider.scale() {
            // TODO: Support configurable subdivision count for shapes that
            //       can't be represented without approximations after scaling.
            collider.set_scale(collider_transform.scale, 10);
        }
    }
}

/// Updates the mass properties of [`Collider`].
#[allow(clippy::type_complexity)]
pub(crate) fn update_collider_mass_properties<C: AnyCollider>(
    mut query: Query<
        (Ref<C>, &ColliderDensity, &mut ColliderMassProperties),
        (Or<(Changed<C>, Changed<ColliderDensity>)>, Without<Sensor>),
    >,
) {
    for (collider, density, mut collider_mass_properties) in &mut query {
        // Update the collider's mass properties.
        *collider_mass_properties =
            ColliderMassProperties::from(collider.mass_properties(density.0));
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unnecessary_cast)]

    use super::*;

    #[test]
    fn sensor_mass_properties() {
        let mut app = App::new();

        app.init_schedule(PhysicsSchedule)
            .init_schedule(SubstepSchedule);

        app.add_plugins((
            MassPropertyPlugin::new(FixedPostUpdate),
            ColliderHierarchyPlugin,
            ColliderTransformPlugin::default(),
            ColliderBackendPlugin::<Collider>::new(FixedPostUpdate),
        ));

        let collider = Collider::capsule(0.5, 2.0);
        let mass_properties = MassPropertiesBundle::from_shape(&collider, 1.0);

        let parent = app
            .world_mut()
            .spawn((
                RigidBody::Dynamic,
                mass_properties.clone(),
                Transform::default(),
            ))
            .id();

        let child = app
            .world_mut()
            .spawn((
                collider,
                Transform::from_xyz(1.0, 0.0, 0.0),
                ChildOf(parent),
            ))
            .id();

        app.world_mut().run_schedule(FixedPostUpdate);

        assert_eq!(
            app.world()
                .entity(parent)
                .get::<ComputedMass>()
                .expect("rigid body should have mass")
                .value() as f32,
            2.0 * mass_properties.mass.0,
        );
        assert!(
            app.world()
                .entity(parent)
                .get::<ComputedCenterOfMass>()
                .expect("rigid body should have a center of mass")
                .x
                > 0.0,
        );

        // Mark the collider as a sensor. It should no longer contribute to the mass properties of the rigid body.
        let mut entity_mut = app.world_mut().entity_mut(child);
        entity_mut.insert(Sensor);

        app.world_mut().run_schedule(FixedPostUpdate);

        assert_eq!(
            app.world()
                .entity(parent)
                .get::<ComputedMass>()
                .expect("rigid body should have mass")
                .value() as f32,
            mass_properties.mass.0,
        );
        assert!(
            app.world()
                .entity(parent)
                .get::<ComputedCenterOfMass>()
                .expect("rigid body should have a center of mass")
                .x
                == 0.0,
        );

        // Remove the sensor component. The collider should contribute to the mass properties of the rigid body again.
        let mut entity_mut = app.world_mut().entity_mut(child);
        entity_mut.remove::<Sensor>();

        app.world_mut().run_schedule(FixedPostUpdate);

        assert_eq!(
            app.world()
                .entity(parent)
                .get::<ComputedMass>()
                .expect("rigid body should have mass")
                .value() as f32,
            2.0 * mass_properties.mass.0,
        );
        assert!(
            app.world()
                .entity(parent)
                .get::<ComputedCenterOfMass>()
                .expect("rigid body should have a center of mass")
                .x
                > 0.0,
        );
    }
}
