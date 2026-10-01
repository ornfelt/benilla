use crate::{
    bundle::Bundle,
    component::{Component, ComponentId, Components, Mutable},
    entity::Entity,
    world::{unsafe_world_cell::UnsafeEntityCell, Mut, Ref},
};

use core::marker::PhantomData;

/// Provides read-only access to a single entity and all its components, save
/// for an explicitly-enumerated set.
pub struct EntityRefExcept<'w, 's, B>
where
    B: Bundle,
{
    entity: UnsafeEntityCell<'w>,
    phantom: PhantomData<(&'s (), B)>,
}

impl<'w, 's, B> EntityRefExcept<'w, 's, B>
where
    B: Bundle,
{
    /// # Safety
    /// Other users of `UnsafeEntityCell` must only have mutable access to the components in `B`.
    pub(crate) unsafe fn new(entity: UnsafeEntityCell<'w>) -> Self {
        Self {
            entity,
            phantom: PhantomData,
        }
    }

    /// Gets access to the component of type `C` for the current entity. Returns
    /// `None` if the component doesn't have a component of that type or if the
    /// type is one of the excluded components.
    #[inline]
    pub fn get<C>(&self) -> Option<&'w C>
    where
        C: Component,
    {
        let components = self.entity.world().components();
        let id = components.valid_component_id::<C>()?;
        if bundle_contains_component::<B>(components, id) {
            None
        } else {
            // SAFETY: We have read access for all components that weren't
            // covered by the `contains` check above.
            unsafe { self.entity.get() }
        }
    }

    /// Gets access to the component of type `C` for the current entity,
    /// including change detection information. Returns `None` if the component
    /// doesn't have a component of that type or if the type is one of the
    /// excluded components.
    #[inline]
    pub fn get_ref<C>(&self) -> Option<Ref<'w, C>>
    where
        C: Component,
    {
        let components = self.entity.world().components();
        let id = components.valid_component_id::<C>()?;
        if bundle_contains_component::<B>(components, id) {
            None
        } else {
            // SAFETY: We have read access for all components that weren't
            // covered by the `contains` check above.
            unsafe { self.entity.get_ref() }
        }
    }
}

impl<B: Bundle> Clone for EntityRefExcept<'_, '_, B> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<B: Bundle> Copy for EntityRefExcept<'_, '_, B> {}

/// Provides mutable access to all components of an entity, with the exception
/// of an explicit set.
///
/// This is a rather niche type that should only be used if you need access to
/// *all* components of an entity, while still allowing you to consult other
/// queries that might match entities that this query also matches. If you don't
/// need access to all components, prefer a standard query with a
/// [`Without`](`crate::query::Without`) filter.
pub struct EntityMutExcept<'w, 's, B>
where
    B: Bundle,
{
    entity: UnsafeEntityCell<'w>,
    phantom: PhantomData<(&'s (), B)>,
}

impl<'w, 's, B> EntityMutExcept<'w, 's, B>
where
    B: Bundle,
{
    /// # Safety
    /// Other users of `UnsafeEntityCell` must not have access to any components not in `B`.
    pub(crate) unsafe fn new(entity: UnsafeEntityCell<'w>) -> Self {
        Self {
            entity,
            phantom: PhantomData,
        }
    }

    /// Returns the [ID](Entity) of the current entity.
    #[inline]
    #[must_use = "Omit the .id() call if you do not need to store the `Entity` identifier."]
    pub fn id(&self) -> Entity {
        self.entity.id()
    }

    /// Returns a new instance with a shorter lifetime.
    ///
    /// This is useful if you have `&mut EntityMutExcept`, but you need
    /// `EntityMutExcept`.
    pub fn reborrow(&mut self) -> EntityMutExcept<'_, 's, B> {
        // SAFETY: We have exclusive access to the entire entity and the
        // applicable components.
        unsafe { Self::new(self.entity) }
    }

    /// Gets read-only access to all of the entity's components, except for the
    /// ones in `CL`.
    #[inline]
    pub fn as_readonly(&self) -> EntityRefExcept<'_, 's, B> {
        EntityRefExcept::from(self)
    }

    /// Gets access to the component of type `C` for the current entity. Returns
    /// `None` if the component doesn't have a component of that type or if the
    /// type is one of the excluded components.
    #[inline]
    pub fn get<C>(&self) -> Option<&'_ C>
    where
        C: Component,
    {
        self.as_readonly().get()
    }

    /// Gets access to the component of type `C` for the current entity,
    /// including change detection information. Returns `None` if the component
    /// doesn't have a component of that type or if the type is one of the
    /// excluded components.
    #[inline]
    pub fn get_ref<C>(&self) -> Option<Ref<'_, C>>
    where
        C: Component,
    {
        self.as_readonly().get_ref()
    }

    /// Gets mutable access to the component of type `C` for the current entity.
    /// Returns `None` if the component doesn't have a component of that type or
    /// if the type is one of the excluded components.
    #[inline]
    pub fn get_mut<C>(&mut self) -> Option<Mut<'_, C>>
    where
        C: Component<Mutability = Mutable>,
    {
        let components = self.entity.world().components();
        let id = components.valid_component_id::<C>()?;
        if bundle_contains_component::<B>(components, id) {
            None
        } else {
            // SAFETY: We have write access for all components that weren't
            // covered by the `contains` check above.
            unsafe { self.entity.get_mut() }
        }
    }
}

impl<'w, 's, B> From<&'w EntityMutExcept<'_, 's, B>> for EntityRefExcept<'w, 's, B>
where
    B: Bundle,
{
    fn from(entity: &'w EntityMutExcept<'_, 's, B>) -> Self {
        // SAFETY: All accesses that `EntityRefExcept` provides are also
        // accesses that `EntityMutExcept` provides.
        unsafe { EntityRefExcept::new(entity.entity) }
    }
}

fn bundle_contains_component<B>(components: &Components, query_id: ComponentId) -> bool
where
    B: Bundle,
{
    let mut found = false;
    for id in B::get_component_ids(components).flatten() {
        found = found || id == query_id;
    }
    found
}
