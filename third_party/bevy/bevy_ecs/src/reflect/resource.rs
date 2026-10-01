//! Definitions for [`Resource`] reflection.
//!
//! # Architecture
//!
//! See the module doc for [`reflect::component`](`crate::reflect::component`).

use crate::{resource::Resource, world::World};
use bevy_reflect::{FromReflect, FromType, PartialReflect, TypePath, TypeRegistry};

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
/// > Usually a [`ReflectResource`] is created for a type by deriving [`Reflect`](bevy_reflect::Reflect)
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
    /// Function pointer implementing [`ReflectResource::apply_or_insert()`].
    pub apply_or_insert: fn(&mut World, &dyn PartialReflect, &TypeRegistry),
    /// Function pointer implementing [`ReflectResource::copy()`].
    pub copy: fn(&World, &mut World, &TypeRegistry),
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
            apply_or_insert: |world, reflected_resource, registry| {
                if let Some(mut resource) = world.get_resource_mut::<R>() {
                    resource.apply(reflected_resource);
                } else {
                    let resource =
                        from_reflect_with_fallback::<R>(reflected_resource, world, registry);
                    world.insert_resource(resource);
                }
            },
            copy: |source_world, destination_world, registry| {
                let source_resource = source_world.resource::<R>();
                let destination_resource =
                    from_reflect_with_fallback::<R>(source_resource, destination_world, registry);
                destination_world.insert_resource(destination_resource);
            },
        })
    }
}
