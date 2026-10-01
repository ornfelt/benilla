//! Contains the definition of the [`EntityCommand`] trait,
//! as well as the blanket implementation of the trait for closures.
//!
//! It also contains functions that return closures for use with
//! [`EntityCommands`](crate::system::EntityCommands).

use crate::{
    bundle::{Bundle, InsertMode},
    change_detection::MaybeLocation,
    component::{Component, ComponentId},
    event::EntityEvent,
    relationship::RelationshipHookMode,
    system::IntoObserverSystem,
    world::{error::EntityMutableFetchError, EntityWorldMut, FromWorld},
};
use bevy_ptr::move_as_ptr;

/// A command which gets executed for a given [`Entity`](crate::entity::Entity).
///
/// Should be used with [`EntityCommands::queue`](crate::system::EntityCommands::queue).
///
/// The `Out` generic parameter is the returned "output" of the command.
///
/// # Examples
///
/// ```
/// # use std::collections::HashSet;
/// # use bevy_ecs::prelude::*;
/// use bevy_ecs::system::EntityCommand;
/// #
/// # #[derive(Component, PartialEq)]
/// # struct Name(String);
/// # impl Name {
/// #   fn new(s: String) -> Self { Name(s) }
/// #   fn as_str(&self) -> &str { &self.0 }
/// # }
///
/// #[derive(Resource, Default)]
/// struct Counter(i64);
///
/// /// A `Command` which names an entity based on a global counter.
/// fn count_name(mut entity: EntityWorldMut) {
///     // Get the current value of the counter, and increment it for next time.
///     let i = {
///         let mut counter = entity.resource_mut::<Counter>();
///         let i = counter.0;
///         counter.0 += 1;
///         i
///     };
///     // Name the entity after the value of the counter.
///     entity.insert(Name::new(format!("Entity #{i}")));
/// }
///
/// // App creation boilerplate omitted...
/// # let mut world = World::new();
/// # world.init_resource::<Counter>();
/// #
/// # let mut setup_schedule = Schedule::default();
/// # setup_schedule.add_systems(setup);
/// # let mut assert_schedule = Schedule::default();
/// # assert_schedule.add_systems(assert_names);
/// #
/// # setup_schedule.run(&mut world);
/// # assert_schedule.run(&mut world);
///
/// fn setup(mut commands: Commands) {
///     commands.spawn_empty().queue(count_name);
///     commands.spawn_empty().queue(count_name);
/// }
///
/// fn assert_names(named: Query<&Name>) {
///     // We use a HashSet because we do not care about the order.
///     let names: HashSet<_> = named.iter().map(Name::as_str).collect();
///     assert_eq!(names, HashSet::from_iter(["Entity #0", "Entity #1"]));
/// }
/// ```
pub trait EntityCommand<Out = ()>: Send + 'static {
    /// Executes this command for the given [`Entity`](crate::entity::Entity).
    fn apply(self, entity: EntityWorldMut) -> Out;
}

/// An error that occurs when running an [`EntityCommand`] on a specific entity.
#[derive(thiserror::Error, Debug)]
pub enum EntityCommandError<E> {
    /// The entity this [`EntityCommand`] tried to run on could not be fetched.
    #[error(transparent)]
    EntityFetchError(#[from] EntityMutableFetchError),
    /// An error that occurred while running the [`EntityCommand`].
    #[error("{0}")]
    CommandFailed(E),
}

impl<Out, F> EntityCommand<Out> for F
where
    F: FnOnce(EntityWorldMut) -> Out + Send + 'static,
{
    fn apply(self, entity: EntityWorldMut) -> Out {
        self(entity)
    }
}

/// An [`EntityCommand`] that adds the components in a [`Bundle`] to an entity.
#[track_caller]
pub fn insert(bundle: impl Bundle, mode: InsertMode) -> impl EntityCommand {
    let caller = MaybeLocation::caller();
    move |mut entity: EntityWorldMut| {
        move_as_ptr!(bundle);
        entity.insert_with_caller(bundle, mode, caller, RelationshipHookMode::Run);
    }
}

/// An [`EntityCommand`] that adds a component to an entity using
/// the component's [`FromWorld`] implementation.
///
/// `T::from_world` will only be invoked if the component will actually be inserted.
/// In other words, `T::from_world` will *not* be invoked if `mode` is [`InsertMode::Keep`]
/// and the entity already has the component.
#[track_caller]
pub fn insert_from_world<T: Component + FromWorld>(mode: InsertMode) -> impl EntityCommand {
    let caller = MaybeLocation::caller();
    move |mut entity: EntityWorldMut| {
        if !(mode == InsertMode::Keep && entity.contains::<T>()) {
            let value = entity.world_scope(|world| T::from_world(world));
            move_as_ptr!(value);
            entity.insert_with_caller(value, mode, caller, RelationshipHookMode::Run);
        }
    }
}

/// An [`EntityCommand`] that adds a component to an entity using
/// some function that returns the component.
///
/// The function will only be invoked if the component will actually be inserted.
/// In other words, the function will *not* be invoked if `mode` is [`InsertMode::Keep`]
/// and the entity already has the component.
#[track_caller]
pub fn insert_with<T: Component, F>(component_fn: F, mode: InsertMode) -> impl EntityCommand
where
    F: FnOnce() -> T + Send + 'static,
{
    let caller = MaybeLocation::caller();
    move |mut entity: EntityWorldMut| {
        if !(mode == InsertMode::Keep && entity.contains::<T>()) {
            let bundle = component_fn();
            move_as_ptr!(bundle);
            entity.insert_with_caller(bundle, mode, caller, RelationshipHookMode::Run);
        }
    }
}

/// An [`EntityCommand`] that removes the components in a [`Bundle`] from an entity.
#[track_caller]
pub fn remove<T: Bundle>() -> impl EntityCommand {
    let caller = MaybeLocation::caller();
    move |mut entity: EntityWorldMut| {
        entity.remove_with_caller::<T>(caller);
    }
}

/// An [`EntityCommand`] that removes the components in a [`Bundle`] from an entity,
/// as well as the required components for each component removed.
#[track_caller]
pub fn remove_with_requires<T: Bundle>() -> impl EntityCommand {
    let caller = MaybeLocation::caller();
    move |mut entity: EntityWorldMut| {
        entity.remove_with_requires_with_caller::<T>(caller);
    }
}

/// An [`EntityCommand`] that removes a dynamic component from an entity.
#[track_caller]
pub fn remove_by_id(component_id: ComponentId) -> impl EntityCommand {
    let caller = MaybeLocation::caller();
    move |mut entity: EntityWorldMut| {
        entity.remove_by_id_with_caller(component_id, caller);
    }
}

/// An [`EntityCommand`] that removes all components from an entity.
#[track_caller]
pub fn clear() -> impl EntityCommand {
    let caller = MaybeLocation::caller();
    move |mut entity: EntityWorldMut| {
        entity.clear_with_caller(caller);
    }
}

/// An [`EntityCommand`] that despawns an entity.
///
/// # Note
///
/// This will also despawn the entities in any [`RelationshipTarget`](crate::relationship::RelationshipTarget)
/// that is configured to despawn descendants.
///
/// For example, this will recursively despawn [`Children`](crate::hierarchy::Children).
#[track_caller]
pub fn despawn() -> impl EntityCommand {
    let caller = MaybeLocation::caller();
    move |entity: EntityWorldMut| {
        entity.despawn_with_caller(caller);
    }
}

/// An [`EntityCommand`] that creates an [`Observer`](crate::observer::Observer)
/// watching for an [`EntityEvent`] of type `E` whose [`EntityEvent::event_target`]
/// targets this entity.
#[track_caller]
pub fn observe<E: EntityEvent, B: Bundle, M>(
    observer: impl IntoObserverSystem<E, B, M>,
) -> impl EntityCommand {
    let caller = MaybeLocation::caller();
    move |mut entity: EntityWorldMut| {
        entity.observe_with_caller(observer, caller);
    }
}
