//! Contains the definition of the [`Command`] trait,
//! as well as the blanket implementation of the trait for closures.
//!
//! It also contains functions that return closures for use with
//! [`Commands`](crate::system::Commands).

use crate::{
    change_detection::MaybeLocation,
    error::Result,
    event::Event,
    message::{Message, Messages},
    resource::Resource,
    system::{IntoSystem, SystemId, SystemInput},
    world::{FromWorld, World},
};

/// A [`World`] mutation.
///
/// Should be used with [`Commands::queue`](crate::system::Commands::queue).
///
/// The `Out` generic parameter is the returned "output" of the command.
///
/// # Usage
///
/// ```
/// # use bevy_ecs::prelude::*;
/// // Our world resource
/// #[derive(Resource, Default)]
/// struct Counter(u64);
///
/// // Our custom command
/// struct AddToCounter(u64);
///
/// impl Command for AddToCounter {
///     fn apply(self, world: &mut World) {
///         let mut counter = world.get_resource_or_insert_with(Counter::default);
///         counter.0 += self.0;
///     }
/// }
///
/// fn some_system(mut commands: Commands) {
///     commands.queue(AddToCounter(42));
/// }
/// ```
pub trait Command<Out = ()>: Send + 'static {
    /// Applies this command, causing it to mutate the provided `world`.
    ///
    /// This method is used to define what a command "does" when it is ultimately applied.
    /// Because this method takes `self`, you can store data or settings on the type that implements this trait.
    /// This data is set by the system or other source of the command, and then ultimately read in this method.
    fn apply(self, world: &mut World) -> Out;
}

impl<F, Out> Command<Out> for F
where
    F: FnOnce(&mut World) -> Out + Send + 'static,
{
    fn apply(self, world: &mut World) -> Out {
        self(world)
    }
}

/// A [`Command`] that inserts a [`Resource`] into the world using a value
/// created with the [`FromWorld`] trait.
#[track_caller]
pub fn init_resource<R: Resource + FromWorld>() -> impl Command {
    move |world: &mut World| {
        world.init_resource::<R>();
    }
}

/// A [`Command`] that inserts a [`Resource`] into the world.
#[track_caller]
pub fn insert_resource<R: Resource>(resource: R) -> impl Command {
    let caller = MaybeLocation::caller();
    move |world: &mut World| {
        world.insert_resource_with_caller(resource, caller);
    }
}

/// A [`Command`] that removes a [`Resource`] from the world.
pub fn remove_resource<R: Resource>() -> impl Command {
    move |world: &mut World| {
        world.remove_resource::<R>();
    }
}

/// A [`Command`] that runs the system corresponding to the given [`SystemId`].
pub fn run_system<O: 'static>(id: SystemId<(), O>) -> impl Command<Result> {
    move |world: &mut World| -> Result {
        world.run_system(id)?;
        Ok(())
    }
}

/// A [`Command`] that runs the system corresponding to the given [`SystemId`]
/// and provides the given input value.
pub fn run_system_with<I>(id: SystemId<I>, input: I::Inner<'static>) -> impl Command<Result>
where
    I: SystemInput<Inner<'static>: Send> + 'static,
{
    move |world: &mut World| -> Result {
        world.run_system_with(id, input)?;
        Ok(())
    }
}

/// A [`Command`] that runs the given system,
/// caching its [`SystemId`] in a [`CachedSystemId`](crate::system::CachedSystemId) resource.
pub fn run_system_cached<M, S>(system: S) -> impl Command<Result>
where
    M: 'static,
    S: IntoSystem<(), (), M> + Send + 'static,
{
    move |world: &mut World| -> Result {
        world.run_system_cached(system)?;
        Ok(())
    }
}

/// A [`Command`] that runs the given system with the given input value,
/// caching its [`SystemId`] in a [`CachedSystemId`](crate::system::CachedSystemId) resource.
pub fn run_system_cached_with<I, M, S>(system: S, input: I::Inner<'static>) -> impl Command<Result>
where
    I: SystemInput<Inner<'static>: Send> + Send + 'static,
    M: 'static,
    S: IntoSystem<I, (), M> + Send + 'static,
{
    move |world: &mut World| -> Result {
        world.run_system_cached_with(system, input)?;
        Ok(())
    }
}

/// A [`Command`] that removes a system previously registered with one of the following:
/// - [`Commands::run_system_cached`](crate::system::Commands::run_system_cached)
/// - [`World::run_system_cached`]
/// - [`World::register_system_cached`]
pub fn unregister_system_cached<I, O, M, S>(system: S) -> impl Command<Result>
where
    I: SystemInput + Send + 'static,
    O: 'static,
    M: 'static,
    S: IntoSystem<I, O, M> + Send + 'static,
{
    move |world: &mut World| -> Result {
        world.unregister_system_cached(system)?;
        Ok(())
    }
}

/// Triggers the given [`Event`], which will run any [`Observer`]s watching for it.
///
/// [`Observer`]: crate::observer::Observer
#[track_caller]
pub fn trigger<'a, E: Event<Trigger<'a>: Default>>(mut event: E) -> impl Command {
    let caller = MaybeLocation::caller();
    move |world: &mut World| {
        world.trigger_ref_with_caller(
            &mut event,
            &mut <E::Trigger<'_> as Default>::default(),
            caller,
        );
    }
}

/// A [`Command`] that writes an arbitrary [`Message`].
#[track_caller]
pub fn write_message<M: Message>(message: M) -> impl Command {
    let caller = MaybeLocation::caller();
    move |world: &mut World| {
        let mut messages = world.resource_mut::<Messages<M>>();
        messages.write_with_caller(message, caller);
    }
}
