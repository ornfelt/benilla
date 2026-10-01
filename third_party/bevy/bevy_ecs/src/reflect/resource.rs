//! Definitions for [`Resource`] reflection.
//!
//! # Architecture
//!
//! See the module doc for [`reflect::component`](`crate::reflect::component`).

use crate::{
    change_detection::Mut,
    component::ComponentId,
    resource::Resource,
    world::{
        error::ResourceFetchError, unsafe_world_cell::UnsafeWorldCell, FilteredResources,
        FilteredResourcesMut, World,
    },
};
use bevy_reflect::{FromReflect, FromType, PartialReflect, Reflect, TypePath, TypeRegistry};

use super::from_reflect_with_fallback;

/// A struct used to operate on reflected [`Resource`] of a type.
///
/// A [`ReflectResource`] for type `T` can be obtained via
/// [`bevy_reflect::TypeRegistration::data`].
#[derive(Clone)]
pub struct ReflectResource(ReflectResourceFns);

/// The raw function pointers needed to make up a [`ReflectResource`].
///
/// > **Note:**
/// > Creating custom implementations of [`ReflectResource`] is an advanced feature that most users
/// > will not need.
/// > Usually a [`ReflectResource`] is created for a type by deriving [`Reflect`]
/// > and adding the `#[reflect(Resource)]` attribute.
/// > After adding the component to the [`TypeRegistry`],
/// > its [`ReflectResource`] can then be retrieved when needed.
///
/// Creating a custom [`ReflectResource`] may be useful if you need to create new resource types at
/// runtime, for example, for scripting implementations.
///
/// By creating a custom [`ReflectResource`] and inserting it into a type's
/// [`TypeRegistration`][bevy_reflect::TypeRegistration],
/// you can modify the way that reflected resources of that type will be inserted into the bevy
/// world.
#[derive(Clone)]
pub struct ReflectResourceFns {
    /// Function pointer implementing `ReflectResource::insert()`.
    pub insert: fn(&mut World, &dyn PartialReflect, &TypeRegistry),
    /// Function pointer implementing `ReflectResource::apply()`.
    pub apply: fn(&mut World, &dyn PartialReflect),
    /// Function pointer implementing [`ReflectResource::apply_or_insert()`].
    pub apply_or_insert: fn(&mut World, &dyn PartialReflect, &TypeRegistry),
    /// Function pointer implementing `ReflectResource::remove()`.
    pub remove: fn(&mut World),
    /// Function pointer implementing [`ReflectResource::reflect()`].
    pub reflect:
        for<'w> fn(FilteredResources<'w, '_>) -> Result<&'w dyn Reflect, ResourceFetchError>,
    /// Function pointer implementing `ReflectResource::reflect_mut()`.
    pub reflect_mut: for<'w> fn(
        FilteredResourcesMut<'w, '_>,
    ) -> Result<Mut<'w, dyn Reflect>, ResourceFetchError>,
    /// Function pointer implementing `ReflectResource::reflect_unchecked_mut()`.
    ///
    /// # Safety
    /// The function may only be called with an [`UnsafeWorldCell`] that can be used to mutably access the relevant resource.
    pub reflect_unchecked_mut: unsafe fn(UnsafeWorldCell<'_>) -> Option<Mut<'_, dyn Reflect>>,
    /// Function pointer implementing [`ReflectResource::copy()`].
    pub copy: fn(&World, &mut World, &TypeRegistry),
    /// Function pointer implementing `ReflectResource::register_resource()`.
    pub register_resource: fn(&mut World) -> ComponentId,
}

impl ReflectResource {
    /// Uses reflection to set the value of this [`Resource`] type in the world to the given value or insert a new one if it does not exist.
    pub fn apply_or_insert(
        &self,
        world: &mut World,
        resource: &dyn PartialReflect,
        registry: &TypeRegistry,
    ) {
        (self.0.apply_or_insert)(world, resource, registry);
    }

    /// Gets the value of this [`Resource`] type from the world as a reflected reference.
    ///
    /// Note that [`&World`](World) is a valid type for `resources`.
    pub fn reflect<'w, 's>(
        &self,
        resources: impl Into<FilteredResources<'w, 's>>,
    ) -> Result<&'w dyn Reflect, ResourceFetchError> {
        (self.0.reflect)(resources.into())
    }

    /// Gets the value of this [`Resource`] type from `source_world` and applies it to the value of this [`Resource`] type in `destination_world`.
    ///
    /// # Panics
    ///
    /// Panics if there is no [`Resource`] of the given type.
    pub fn copy(
        &self,
        source_world: &World,
        destination_world: &mut World,
        registry: &TypeRegistry,
    ) {
        (self.0.copy)(source_world, destination_world, registry);
    }
}

impl<R: Resource + FromReflect + TypePath> FromType<R> for ReflectResource {
    fn from_type() -> Self {
        ReflectResource(ReflectResourceFns {
            insert: |world, reflected_resource, registry| {
                let resource = from_reflect_with_fallback::<R>(reflected_resource, world, registry);
                world.insert_resource(resource);
            },
            apply: |world, reflected_resource| {
                let mut resource = world.resource_mut::<R>();
                resource.apply(reflected_resource);
            },
            apply_or_insert: |world, reflected_resource, registry| {
                if let Some(mut resource) = world.get_resource_mut::<R>() {
                    resource.apply(reflected_resource);
                } else {
                    let resource =
                        from_reflect_with_fallback::<R>(reflected_resource, world, registry);
                    world.insert_resource(resource);
                }
            },
            remove: |world| {
                world.remove_resource::<R>();
            },
            reflect: |world| world.get::<R>().map(|res| res.into_inner() as &dyn Reflect),
            reflect_mut: |world| {
                world
                    .into_mut::<R>()
                    .map(|res| res.map_unchanged(|value| value as &mut dyn Reflect))
            },
            reflect_unchecked_mut: |world| {
                // SAFETY: all usages of `reflect_unchecked_mut` guarantee that there is either a single mutable
                // reference or multiple immutable ones alive at any given point
                let res = unsafe { world.get_resource_mut::<R>() };
                res.map(|res| res.map_unchanged(|value| value as &mut dyn Reflect))
            },
            copy: |source_world, destination_world, registry| {
                let source_resource = source_world.resource::<R>();
                let destination_resource =
                    from_reflect_with_fallback::<R>(source_resource, destination_world, registry);
                destination_world.insert_resource(destination_resource);
            },

            register_resource: |world: &mut World| -> ComponentId {
                world.register_resource::<R>()
            },
        })
    }
}
