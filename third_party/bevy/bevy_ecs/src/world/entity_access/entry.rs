use crate::{
    component::{Component, Mutable},
    world::{EntityWorldMut, Mut},
};

use core::marker::PhantomData;

/// A view into a single entity and component in a world, which may either be vacant or occupied.
///
/// This `enum` can only be constructed from the [`entry`] method on [`EntityWorldMut`].
///
/// [`entry`]: EntityWorldMut::entry
pub enum ComponentEntry<'w, 'a, T: Component> {
    /// An occupied entry.
    Occupied(OccupiedComponentEntry<'w, 'a, T>),
    /// A vacant entry.
    Vacant(VacantComponentEntry<'w, 'a, T>),
}

impl<'w, 'a, T: Component + Default> ComponentEntry<'w, 'a, T> {
    /// Ensures the entry has this component by inserting the default value if empty, and
    /// returns a mutable reference to this component in the entry.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #[derive(Component, Default, Clone, Copy, Debug, PartialEq)]
    /// struct Comp(u32);
    ///
    /// # let mut world = World::new();
    /// let mut entity = world.spawn_empty();
    ///
    /// entity.entry::<Comp>().or_default();
    /// assert_eq!(world.query::<&Comp>().single(&world).unwrap().0, 0);
    /// ```
    #[inline]
    pub fn or_default(self) -> OccupiedComponentEntry<'w, 'a, T> {
        match self {
            ComponentEntry::Occupied(entry) => entry,
            ComponentEntry::Vacant(entry) => entry.insert(Default::default()),
        }
    }
}

/// A view into an occupied entry in a [`EntityWorldMut`]. It is part of the [`OccupiedComponentEntry`] enum.
///
/// The contained entity must have the component type parameter if we have this struct.
pub struct OccupiedComponentEntry<'w, 'a, T: Component> {
    pub(crate) entity_world: &'a mut EntityWorldMut<'w>,
    pub(crate) _marker: PhantomData<T>,
}

impl<'w, 'a, T: Component<Mutability = Mutable>> OccupiedComponentEntry<'w, 'a, T> {
    /// Converts the [`OccupiedComponentEntry`] into a mutable reference to the value in the entry with
    /// a lifetime bound to the `EntityWorldMut`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_ecs::{prelude::*, world::ComponentEntry};
    /// #[derive(Component, Default, Clone, Copy, Debug, PartialEq)]
    /// struct Comp(u32);
    ///
    /// # let mut world = World::new();
    /// let mut entity = world.spawn(Comp(5));
    ///
    /// if let ComponentEntry::Occupied(o) = entity.entry::<Comp>() {
    ///     o.into_mut().0 += 10;
    /// }
    ///
    /// assert_eq!(world.query::<&Comp>().single(&world).unwrap().0, 15);
    /// ```
    #[inline]
    pub fn into_mut(self) -> Mut<'a, T> {
        // This shouldn't panic because if we have an OccupiedComponentEntry the component must exist.
        self.entity_world.get_mut().unwrap()
    }
}

/// A view into a vacant entry in a [`EntityWorldMut`]. It is part of the [`ComponentEntry`] enum.
pub struct VacantComponentEntry<'w, 'a, T: Component> {
    pub(crate) entity_world: &'a mut EntityWorldMut<'w>,
    pub(crate) _marker: PhantomData<T>,
}

impl<'w, 'a, T: Component> VacantComponentEntry<'w, 'a, T> {
    /// Inserts the component into the [`VacantComponentEntry`] and returns an [`OccupiedComponentEntry`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_ecs::{prelude::*, world::ComponentEntry};
    /// #[derive(Component, Default, Clone, Copy, Debug, PartialEq)]
    /// struct Comp(u32);
    ///
    /// # let mut world = World::new();
    /// let mut entity = world.spawn_empty();
    ///
    /// if let ComponentEntry::Vacant(v) = entity.entry::<Comp>() {
    ///     v.insert(Comp(10));
    /// }
    ///
    /// assert_eq!(world.query::<&Comp>().single(&world).unwrap().0, 10);
    /// ```
    #[inline]
    pub fn insert(self, component: T) -> OccupiedComponentEntry<'w, 'a, T> {
        self.entity_world.insert(component);
        OccupiedComponentEntry {
            entity_world: self.entity_world,
            _marker: PhantomData,
        }
    }
}
