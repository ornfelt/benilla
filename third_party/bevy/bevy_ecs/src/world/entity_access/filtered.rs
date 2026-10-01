use crate::{
    change_detection::{ComponentTicks, MutUntyped},
    component::{Component, ComponentId, Mutable},
    entity::{ContainsEntity, Entity, EntityEquivalent},
    query::Access,
    world::{unsafe_world_cell::UnsafeEntityCell, EntityMut, EntityRef, Mut, Ref},
};

use bevy_ptr::Ptr;
use core::{
    any::TypeId,
    cmp::Ordering,
    hash::{Hash, Hasher},
};
use thiserror::Error;

/// Provides read-only access to a single entity and some of its components defined by the contained [`Access`].
///
/// To define the access when used as a [`QueryData`](crate::query::QueryData),
/// use a [`QueryBuilder`](crate::query::QueryBuilder) or [`QueryParamBuilder`](crate::system::QueryParamBuilder).
/// The [`FilteredEntityRef`] must be the entire [`QueryData`](crate::query::QueryData), and not nested inside a tuple with other data.
///
/// ```
/// # use bevy_ecs::{prelude::*, world::FilteredEntityRef};
/// #
/// # #[derive(Component)]
/// # struct A;
/// #
/// # let mut world = World::new();
/// # world.spawn(A);
/// #
/// // This gives the `FilteredEntityRef` access to `&A`.
/// let mut query = QueryBuilder::<FilteredEntityRef>::new(&mut world)
///     .data::<&A>()
///     .build();
///
/// let filtered_entity: FilteredEntityRef = query.single(&mut world).unwrap();
/// let component: &A = filtered_entity.get().unwrap();
/// ```
#[derive(Clone, Copy)]
pub struct FilteredEntityRef<'w, 's> {
    entity: UnsafeEntityCell<'w>,
    access: &'s Access,
}

impl<'w, 's> FilteredEntityRef<'w, 's> {
    /// # Safety
    /// - No `&mut World` can exist from the underlying `UnsafeWorldCell`
    /// - If `access` takes read access to a component no mutable reference to that
    ///   component can exist at the same time as the returned [`FilteredEntityMut`]
    /// - If `access` takes any access for a component `entity` must have that component.
    #[inline]
    pub(crate) unsafe fn new(entity: UnsafeEntityCell<'w>, access: &'s Access) -> Self {
        Self { entity, access }
    }

    /// Returns the [ID](Entity) of the current entity.
    #[inline]
    #[must_use = "Omit the .id() call if you do not need to store the `Entity` identifier."]
    pub fn id(&self) -> Entity {
        self.entity.id()
    }

    /// Returns `true` if the current entity has a component of type `T`.
    /// Otherwise, this returns `false`.
    ///
    /// ## Notes
    ///
    /// If you do not know the concrete type of a component, consider using
    /// [`Self::contains_type_id`].
    #[inline]
    pub fn contains<T: Component>(&self) -> bool {
        self.contains_type_id(TypeId::of::<T>())
    }

    /// Returns `true` if the current entity has a component with the type identified by `type_id`.
    /// Otherwise, this returns false.
    ///
    /// ## Notes
    ///
    /// - If you know the concrete type of the component, you should prefer [`Self::contains`].
    #[inline]
    pub fn contains_type_id(&self, type_id: TypeId) -> bool {
        self.entity.contains_type_id(type_id)
    }

    /// Gets access to the component of type `T` for the current entity.
    /// Returns `None` if the entity does not have a component of type `T`.
    #[inline]
    pub fn get<T: Component>(&self) -> Option<&'w T> {
        let id = self
            .entity
            .world()
            .components()
            .get_valid_id(TypeId::of::<T>())?;
        self.access
            .has_component_read(id)
            // SAFETY: We have read access
            .then(|| unsafe { self.entity.get() })
            .flatten()
    }

    /// Gets access to the component of type `T` for the current entity,
    /// including change detection information as a [`Ref`].
    ///
    /// Returns `None` if the entity does not have a component of type `T`.
    #[inline]
    pub fn get_ref<T: Component>(&self) -> Option<Ref<'w, T>> {
        let id = self
            .entity
            .world()
            .components()
            .get_valid_id(TypeId::of::<T>())?;
        self.access
            .has_component_read(id)
            // SAFETY: We have read access
            .then(|| unsafe { self.entity.get_ref() })
            .flatten()
    }

    /// Retrieves the change ticks for the given component. This can be useful for implementing change
    /// detection in custom runtimes.
    #[inline]
    pub fn get_change_ticks<T: Component>(&self) -> Option<ComponentTicks> {
        let id = self
            .entity
            .world()
            .components()
            .get_valid_id(TypeId::of::<T>())?;
        self.access
            .has_component_read(id)
            // SAFETY: We have read access
            .then(|| unsafe { self.entity.get_change_ticks::<T>() })
            .flatten()
    }

    /// Retrieves the change ticks for the given [`ComponentId`]. This can be useful for implementing change
    /// detection in custom runtimes.
    ///
    /// **You should prefer to use the typed API [`Self::get_change_ticks`] where possible and only
    /// use this in cases where the actual component types are not known at
    /// compile time.**
    #[inline]
    pub fn get_change_ticks_by_id(&self, component_id: ComponentId) -> Option<ComponentTicks> {
        self.access
            .has_component_read(component_id)
            // SAFETY: We have read access
            .then(|| unsafe { self.entity.get_change_ticks_by_id(component_id) })
            .flatten()
    }

    /// Gets the component of the given [`ComponentId`] from the entity.
    ///
    /// **You should prefer to use the typed API [`Self::get`] where possible and only
    /// use this in cases where the actual component types are not known at
    /// compile time.**
    ///
    /// Unlike [`FilteredEntityRef::get`], this returns a raw pointer to the component,
    /// which is only valid while the [`FilteredEntityRef`] is alive.
    #[inline]
    pub fn get_by_id(&self, component_id: ComponentId) -> Option<Ptr<'w>> {
        self.access
            .has_component_read(component_id)
            // SAFETY: We have read access
            .then(|| unsafe { self.entity.get_by_id(component_id) })
            .flatten()
    }
}

impl<'a> TryFrom<FilteredEntityRef<'a, '_>> for EntityRef<'a> {
    type Error = TryFromFilteredError;

    fn try_from(entity: FilteredEntityRef<'a, '_>) -> Result<Self, Self::Error> {
        if !entity.access.has_read_all() {
            Err(TryFromFilteredError::MissingReadAllAccess)
        } else {
            // SAFETY: check above guarantees read-only access to all components of the entity.
            Ok(unsafe { EntityRef::new(entity.entity) })
        }
    }
}

impl<'a> TryFrom<&'a FilteredEntityRef<'_, '_>> for EntityRef<'a> {
    type Error = TryFromFilteredError;

    fn try_from(entity: &'a FilteredEntityRef<'_, '_>) -> Result<Self, Self::Error> {
        if !entity.access.has_read_all() {
            Err(TryFromFilteredError::MissingReadAllAccess)
        } else {
            // SAFETY: check above guarantees read-only access to all components of the entity.
            Ok(unsafe { EntityRef::new(entity.entity) })
        }
    }
}

impl PartialEq for FilteredEntityRef<'_, '_> {
    fn eq(&self, other: &Self) -> bool {
        self.entity() == other.entity()
    }
}

impl Eq for FilteredEntityRef<'_, '_> {}

impl PartialOrd for FilteredEntityRef<'_, '_> {
    /// [`FilteredEntityRef`]'s comparison trait implementations match the underlying [`Entity`],
    /// and cannot discern between different worlds.
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FilteredEntityRef<'_, '_> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.entity().cmp(&other.entity())
    }
}

impl Hash for FilteredEntityRef<'_, '_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.entity().hash(state);
    }
}

impl ContainsEntity for FilteredEntityRef<'_, '_> {
    fn entity(&self) -> Entity {
        self.id()
    }
}

// SAFETY: This type represents one Entity. We implement the comparison traits based on that Entity.
unsafe impl EntityEquivalent for FilteredEntityRef<'_, '_> {}

/// Provides mutable access to a single entity and some of its components defined by the contained [`Access`].
///
/// To define the access when used as a [`QueryData`](crate::query::QueryData),
/// use a [`QueryBuilder`](crate::query::QueryBuilder) or [`QueryParamBuilder`](crate::system::QueryParamBuilder).
/// The `FilteredEntityMut` must be the entire `QueryData`, and not nested inside a tuple with other data.
///
/// ```
/// # use bevy_ecs::{prelude::*, world::FilteredEntityMut};
/// #
/// # #[derive(Component)]
/// # struct A;
/// #
/// # let mut world = World::new();
/// # world.spawn(A);
/// #
/// // This gives the `FilteredEntityMut` access to `&mut A`.
/// let mut query = QueryBuilder::<FilteredEntityMut>::new(&mut world)
///     .data::<&mut A>()
///     .build();
///
/// let mut filtered_entity: FilteredEntityMut = query.single_mut(&mut world).unwrap();
/// let component: Mut<A> = filtered_entity.get_mut().unwrap();
/// ```
pub struct FilteredEntityMut<'w, 's> {
    entity: UnsafeEntityCell<'w>,
    access: &'s Access,
}

impl<'w, 's> FilteredEntityMut<'w, 's> {
    /// # Safety
    /// - No `&mut World` can exist from the underlying `UnsafeWorldCell`
    /// - If `access` takes read access to a component no mutable reference to that
    ///   component can exist at the same time as the returned [`FilteredEntityMut`]
    /// - If `access` takes write access to a component, no reference to that component
    ///   may exist at the same time as the returned [`FilteredEntityMut`]
    /// - If `access` takes any access for a component `entity` must have that component.
    #[inline]
    pub(crate) unsafe fn new(entity: UnsafeEntityCell<'w>, access: &'s Access) -> Self {
        Self { entity, access }
    }

    /// Gets read-only access to all of the entity's components.
    #[inline]
    pub fn as_readonly(&self) -> FilteredEntityRef<'_, 's> {
        FilteredEntityRef::from(self)
    }

    /// Returns the [ID](Entity) of the current entity.
    #[inline]
    #[must_use = "Omit the .id() call if you do not need to store the `Entity` identifier."]
    pub fn id(&self) -> Entity {
        self.entity.id()
    }

    /// Gets access to the component of type `T` for the current entity.
    /// Returns `None` if the entity does not have a component of type `T`.
    #[inline]
    pub fn get<T: Component>(&self) -> Option<&'_ T> {
        self.as_readonly().get()
    }

    /// Gets access to the component of type `T` for the current entity,
    /// including change detection information as a [`Ref`].
    ///
    /// Returns `None` if the entity does not have a component of type `T`.
    #[inline]
    pub fn get_ref<T: Component>(&self) -> Option<Ref<'_, T>> {
        self.as_readonly().get_ref()
    }

    /// Gets mutable access to the component of type `T` for the current entity.
    /// Returns `None` if the entity does not have a component of type `T` or if
    /// the access does not include write access to `T`.
    #[inline]
    pub fn get_mut<T: Component<Mutability = Mutable>>(&mut self) -> Option<Mut<'_, T>> {
        // SAFETY: we use a mutable reference to self, so we cannot use the `FilteredEntityMut` to access
        // another component
        unsafe { self.get_mut_unchecked() }
    }

    /// Gets mutable access to the component of type `T` for the current entity.
    /// Returns `None` if the entity does not have a component of type `T` or if
    /// the access does not include write access to `T`.
    ///
    /// This only requires `&self`, and so may be used to get mutable access to multiple components.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::{prelude::*, world::FilteredEntityMut};
    /// #
    /// #[derive(Component)]
    /// struct X(usize);
    /// #[derive(Component)]
    /// struct Y(usize);
    ///
    /// # let mut world = World::default();
    /// let mut entity = world.spawn((X(0), Y(0))).into_mutable();
    ///
    /// // This gives the `FilteredEntityMut` access to `&mut X` and `&mut Y`.
    /// let mut query = QueryBuilder::<FilteredEntityMut>::new(&mut world)
    ///     .data::<(&mut X, &mut Y)>()
    ///     .build();
    ///
    /// let mut filtered_entity: FilteredEntityMut = query.single_mut(&mut world).unwrap();
    ///
    /// // Get mutable access to two components at once
    /// // SAFETY: We don't take any other references to `X` from this entity
    /// let mut x = unsafe { filtered_entity.get_mut_unchecked::<X>() }.unwrap();
    /// // SAFETY: We don't take any other references to `Y` from this entity
    /// let mut y = unsafe { filtered_entity.get_mut_unchecked::<Y>() }.unwrap();
    /// *x = X(1);
    /// *y = Y(1);
    /// ```
    ///
    /// # Safety
    ///
    /// No other references to the same component may exist at the same time as the returned reference.
    ///
    /// # See also
    ///
    /// - [`get_mut`](Self::get_mut) for the safe version.
    #[inline]
    pub unsafe fn get_mut_unchecked<T: Component<Mutability = Mutable>>(
        &self,
    ) -> Option<Mut<'_, T>> {
        let id = self
            .entity
            .world()
            .components()
            .get_valid_id(TypeId::of::<T>())?;
        self.access
            .has_component_write(id)
            // SAFETY: We have permission to access the component mutable
            // and we promise to not create other references to the same component
            .then(|| unsafe { self.entity.get_mut() })
            .flatten()
    }

    /// Consumes self and gets mutable access to the component of type `T`
    /// with the world `'w` lifetime for the current entity.
    /// Returns `None` if the entity does not have a component of type `T`.
    ///
    /// # Safety
    ///
    /// - `T` must be a mutable component
    #[inline]
    pub unsafe fn into_mut_assume_mutable<T: Component>(self) -> Option<Mut<'w, T>> {
        let id = self
            .entity
            .world()
            .components()
            .get_valid_id(TypeId::of::<T>())?;
        self.access
            .has_component_write(id)
            // SAFETY:
            // - We have write access
            // - Caller ensures `T` is a mutable component
            .then(|| unsafe { self.entity.get_mut_assume_mutable() })
            .flatten()
    }

    /// Retrieves the change ticks for the given component. This can be useful for implementing change
    /// detection in custom runtimes.
    #[inline]
    pub fn get_change_ticks<T: Component>(&self) -> Option<ComponentTicks> {
        self.as_readonly().get_change_ticks::<T>()
    }

    /// Retrieves the change ticks for the given [`ComponentId`]. This can be useful for implementing change
    /// detection in custom runtimes.
    ///
    /// **You should prefer to use the typed API [`Self::get_change_ticks`] where possible and only
    /// use this in cases where the actual component types are not known at
    /// compile time.**
    #[inline]
    pub fn get_change_ticks_by_id(&self, component_id: ComponentId) -> Option<ComponentTicks> {
        self.as_readonly().get_change_ticks_by_id(component_id)
    }

    /// Gets the component of the given [`ComponentId`] from the entity.
    ///
    /// **You should prefer to use the typed API [`Self::get`] where possible and only
    /// use this in cases where the actual component types are not known at
    /// compile time.**
    ///
    /// Unlike [`FilteredEntityMut::get`], this returns a raw pointer to the component,
    /// which is only valid while the [`FilteredEntityMut`] is alive.
    #[inline]
    pub fn get_by_id(&self, component_id: ComponentId) -> Option<Ptr<'_>> {
        self.as_readonly().get_by_id(component_id)
    }

    /// Gets a [`MutUntyped`] of the component of the given [`ComponentId`] from the entity.
    ///
    /// **You should prefer to use the typed API [`Self::get_mut`] where possible and only
    /// use this in cases where the actual component types are not known at
    /// compile time.**
    ///
    /// Unlike [`FilteredEntityMut::get_mut`], this returns a raw pointer to the component,
    /// which is only valid while the [`FilteredEntityMut`] is alive.
    #[inline]
    pub fn get_mut_by_id(&mut self, component_id: ComponentId) -> Option<MutUntyped<'_>> {
        // SAFETY: we use a mutable reference to self, so we cannot use the `FilteredEntityMut` to access
        // another component
        unsafe { self.get_mut_by_id_unchecked(component_id) }
    }

    /// Gets a [`MutUntyped`] of the component of the given [`ComponentId`] from the entity.
    ///
    /// **You should prefer to use the typed API [`Self::get_mut`] where possible and only
    /// use this in cases where the actual component types are not known at
    /// compile time.**
    ///
    /// Unlike [`FilteredEntityMut::get_mut`], this returns a raw pointer to the component,
    /// which is only valid while the [`FilteredEntityMut`] is alive.
    ///
    /// This only requires `&self`, and so may be used to get mutable access to multiple components.
    ///
    /// # Safety
    ///
    /// No other references to the same component may exist at the same time as the returned reference.
    ///
    /// # See also
    ///
    /// - [`get_mut_by_id`](Self::get_mut_by_id) for the safe version.
    #[inline]
    pub unsafe fn get_mut_by_id_unchecked(
        &self,
        component_id: ComponentId,
    ) -> Option<MutUntyped<'_>> {
        self.access
            .has_component_write(component_id)
            // SAFETY: We have permission to access the component mutable
            // and we promise to not create other references to the same component
            .then(|| unsafe { self.entity.get_mut_by_id(component_id).ok() })
            .flatten()
    }
}

impl<'a> TryFrom<FilteredEntityMut<'a, '_>> for EntityRef<'a> {
    type Error = TryFromFilteredError;

    fn try_from(entity: FilteredEntityMut<'a, '_>) -> Result<Self, Self::Error> {
        if !entity.access.has_read_all() {
            Err(TryFromFilteredError::MissingReadAllAccess)
        } else {
            // SAFETY: check above guarantees read-only access to all components of the entity.
            Ok(unsafe { EntityRef::new(entity.entity) })
        }
    }
}

impl<'a> TryFrom<&'a FilteredEntityMut<'_, '_>> for EntityRef<'a> {
    type Error = TryFromFilteredError;

    fn try_from(entity: &'a FilteredEntityMut<'_, '_>) -> Result<Self, Self::Error> {
        if !entity.access.has_read_all() {
            Err(TryFromFilteredError::MissingReadAllAccess)
        } else {
            // SAFETY: check above guarantees read-only access to all components of the entity.
            Ok(unsafe { EntityRef::new(entity.entity) })
        }
    }
}

impl<'a> TryFrom<FilteredEntityMut<'a, '_>> for EntityMut<'a> {
    type Error = TryFromFilteredError;

    fn try_from(entity: FilteredEntityMut<'a, '_>) -> Result<Self, Self::Error> {
        if !entity.access.has_read_all() {
            Err(TryFromFilteredError::MissingReadAllAccess)
        } else if !entity.access.has_write_all() {
            Err(TryFromFilteredError::MissingWriteAllAccess)
        } else {
            // SAFETY: check above guarantees exclusive access to all components of the entity.
            Ok(unsafe { EntityMut::new(entity.entity) })
        }
    }
}

impl<'a> TryFrom<&'a mut FilteredEntityMut<'_, '_>> for EntityMut<'a> {
    type Error = TryFromFilteredError;

    fn try_from(entity: &'a mut FilteredEntityMut<'_, '_>) -> Result<Self, Self::Error> {
        if !entity.access.has_read_all() {
            Err(TryFromFilteredError::MissingReadAllAccess)
        } else if !entity.access.has_write_all() {
            Err(TryFromFilteredError::MissingWriteAllAccess)
        } else {
            // SAFETY: check above guarantees exclusive access to all components of the entity.
            Ok(unsafe { EntityMut::new(entity.entity) })
        }
    }
}

impl<'w, 's> From<FilteredEntityMut<'w, 's>> for FilteredEntityRef<'w, 's> {
    #[inline]
    fn from(entity: FilteredEntityMut<'w, 's>) -> Self {
        // SAFETY:
        // - `FilteredEntityMut` guarantees exclusive access to all components in the new `FilteredEntityRef`.
        unsafe { FilteredEntityRef::new(entity.entity, entity.access) }
    }
}

impl<'w, 's> From<&'w FilteredEntityMut<'_, 's>> for FilteredEntityRef<'w, 's> {
    #[inline]
    fn from(entity: &'w FilteredEntityMut<'_, 's>) -> Self {
        // SAFETY:
        // - `FilteredEntityMut` guarantees exclusive access to all components in the new `FilteredEntityRef`.
        unsafe { FilteredEntityRef::new(entity.entity, entity.access) }
    }
}

impl PartialEq for FilteredEntityMut<'_, '_> {
    fn eq(&self, other: &Self) -> bool {
        self.entity() == other.entity()
    }
}

impl Eq for FilteredEntityMut<'_, '_> {}

impl PartialOrd for FilteredEntityMut<'_, '_> {
    /// [`FilteredEntityMut`]'s comparison trait implementations match the underlying [`Entity`],
    /// and cannot discern between different worlds.
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FilteredEntityMut<'_, '_> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.entity().cmp(&other.entity())
    }
}

impl Hash for FilteredEntityMut<'_, '_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.entity().hash(state);
    }
}

impl ContainsEntity for FilteredEntityMut<'_, '_> {
    fn entity(&self) -> Entity {
        self.id()
    }
}

// SAFETY: This type represents one Entity. We implement the comparison traits based on that Entity.
unsafe impl EntityEquivalent for FilteredEntityMut<'_, '_> {}

/// Error type returned by [`TryFrom`] conversions from filtered entity types
/// ([`FilteredEntityRef`]/[`FilteredEntityMut`]) to full-access entity types
/// ([`EntityRef`]/[`EntityMut`]).
#[derive(Error, Debug)]
pub enum TryFromFilteredError {
    /// Error indicating that the filtered entity does not have read access to
    /// all components.
    #[error("Conversion failed, filtered entity ref does not have read access to all components")]
    MissingReadAllAccess,
    /// Error indicating that the filtered entity does not have write access to
    /// all components.
    #[error("Conversion failed, filtered entity ref does not have write access to all components")]
    MissingWriteAllAccess,
}
