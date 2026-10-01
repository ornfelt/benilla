use crate::{component::Component, query::Access, world::unsafe_world_cell::UnsafeEntityCell};

use core::any::TypeId;

/// Provides read-only access to a single entity and some of its components defined by the contained [`Access`].
#[derive(Clone, Copy)]
pub struct FilteredEntityRef<'w, 's> {
    entity: UnsafeEntityCell<'w>,
    access: &'s Access,
}

impl<'w, 's> FilteredEntityRef<'w, 's> {
    /// # Safety
    /// - No `&mut World` can exist from the underlying `UnsafeWorldCell`
    /// - If `access` takes read access to a component no mutable reference to that
    ///   component can exist at the same time as the returned [`FilteredEntityRef`]
    /// - If `access` takes any access for a component `entity` must have that component.
    #[inline]
    pub(crate) unsafe fn new(entity: UnsafeEntityCell<'w>, access: &'s Access) -> Self {
        Self { entity, access }
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
}
