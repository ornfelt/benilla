//! System parameters for working with observers.

use crate::{
    bundle::Bundle,
    change_detection::MaybeLocation,
    event::{Event, EventKey},
    prelude::*,
};
use core::{
    fmt::Debug,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

/// A [system parameter] used by an observer to process events. See [`Observer`] and [`Event`] for examples.
///
/// `On` contains the triggered [`Event`] data for a given run of an `Observer`. It also provides access to the
/// [`Trigger`](crate::event::Trigger).
///
/// The generic `B: Bundle` is used to further specialize the events that this observer is interested in.
/// The entity involved *does not* have to have these components, but the observer will only be
/// triggered if the event matches the components in `B`.
///
/// This is used to avoid providing a generic argument in your event, as is done for [`Add`]
/// and the other lifecycle events.
///
/// Providing multiple components in this bundle will cause this event to be triggered by any
/// matching component in the bundle,
/// [rather than requiring all of them to be present](https://github.com/bevyengine/bevy/issues/15325).
///
/// [system parameter]: crate::system::SystemParam
// SAFETY WARNING!
// this type must _never_ expose anything with the 'w lifetime
// See the safety discussion on `Trigger` for more details.
pub struct On<'w, 't, E: Event, B: Bundle = ()> {
    observer: Entity,
    // SAFETY WARNING: never expose this 'w lifetime
    event: &'w mut E,
    // SAFETY WARNING: never expose this 'w lifetime
    trigger: &'w mut E::Trigger<'t>,
    // SAFETY WARNING: never expose this 'w lifetime
    trigger_context: &'w TriggerContext,
    _marker: PhantomData<B>,
}

impl<'w, 't, E: Event, B: Bundle> On<'w, 't, E, B> {
    /// Creates a new instance of [`On`] for the given triggered event.
    pub fn new(
        event: &'w mut E,
        observer: Entity,
        trigger: &'w mut E::Trigger<'t>,
        trigger_context: &'w TriggerContext,
    ) -> Self {
        Self {
            event,
            observer,
            trigger,
            trigger_context,
            _marker: PhantomData,
        }
    }

    /// Returns a reference to the triggered event.
    pub fn event(&self) -> &E {
        self.event
    }

    /// Returns the [`Entity`] of the [`Observer`] of the triggered event.
    /// This allows you to despawn the observer, ceasing observation.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_ecs::prelude::*;
    ///
    /// #[derive(EntityEvent)]  
    /// struct AssertEvent {
    ///     entity: Entity,
    /// }
    ///
    /// fn assert_observer(event: On<AssertEvent>) {  
    ///     assert_eq!(event.observer(), event.entity);  
    /// }  
    ///
    /// let mut world = World::new();  
    /// let entity = world.spawn(Observer::new(assert_observer)).id();  
    ///
    /// world.trigger(AssertEvent { entity });  
    /// ```
    pub fn observer(&self) -> Entity {
        self.observer
    }

    /// Returns the source code location that triggered this observer, if the `track_location` cargo feature is enabled.
    pub fn caller(&self) -> MaybeLocation {
        self.trigger_context.caller
    }
}

impl<'w, 't, E: for<'a> Event<Trigger<'a>: Debug> + Debug, B: Bundle> Debug for On<'w, 't, E, B> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("On")
            .field("event", &self.event)
            .field("trigger", &self.trigger)
            .field("_marker", &self._marker)
            .finish()
    }
}

impl<'w, 't, E: Event, B: Bundle> Deref for On<'w, 't, E, B> {
    type Target = E;

    fn deref(&self) -> &Self::Target {
        self.event
    }
}

impl<'w, 't, E: Event, B: Bundle> DerefMut for On<'w, 't, E, B> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.event
    }
}

/// Metadata about a specific [`Event`] that triggered an observer.
///
/// This information is exposed via methods on [`On`].
pub struct TriggerContext {
    /// The [`EventKey`] the trigger targeted.
    pub event_key: EventKey,
    /// The location of the source code that triggered the observer.
    pub caller: MaybeLocation,
}
