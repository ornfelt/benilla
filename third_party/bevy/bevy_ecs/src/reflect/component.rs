//! Definitions for [`Component`] reflection.
//! This allows inserting, updating, removing and generally interacting with components
//! whose types are only known at runtime.
//!
//! This module exports two types: [`ReflectComponentFns`] and [`ReflectComponent`].
//!
//! # Architecture
//!
//! [`ReflectComponent`] wraps a [`ReflectComponentFns`]. In fact, each method on
//! [`ReflectComponent`] wraps a call to a function pointer field in `ReflectComponentFns`.
//!
//! ## Who creates `ReflectComponent`s?
//!
//! When a user adds the `#[reflect(Component)]` attribute to their `#[derive(Reflect)]`
//! type, it tells the derive macro for `Reflect` to add the following single line to its
//! [`get_type_registration`] method (see the relevant code[^1]).
//!
//! ```
//! # use bevy_reflect::{FromType, Reflect};
//! # use bevy_ecs::prelude::{ReflectComponent, Component};
//! # #[derive(Default, Reflect, Component)]
//! # struct A;
//! # impl A {
//! #   fn foo() {
//! # let mut registration = bevy_reflect::TypeRegistration::of::<A>();
//! registration.insert::<ReflectComponent>(FromType::<Self>::from_type());
//! #   }
//! # }
//! ```
//!
//! This line adds a `ReflectComponent` to the registration data for the type in question.
//! The user can access the `ReflectComponent` for type `T` through the type registry,
//! as per the `trait_reflection.rs` example.
//!
//! The `FromType::<Self>::from_type()` in the previous line calls the `FromType<C>`
//! implementation of `ReflectComponent`.
//!
//! The `FromType<C>` impl creates a function per field of [`ReflectComponentFns`].
//! In those functions, we call generic methods on [`World`] and [`EntityWorldMut`].
//!
//! The result is a `ReflectComponent` completely independent of `C`, yet capable
//! of using generic ECS methods such as `entity.get::<C>()` to get `&dyn Reflect`
//! with underlying type `C`, without the `C` appearing in the type signature.
//!
//! ## A note on code generation
//!
//! A downside of this approach is that monomorphized code (ie: concrete code
//! for generics) is generated **unconditionally**, regardless of whether it ends
//! up used or not.
//!
//! Adding `N` fields on `ReflectComponentFns` will generate `N × M` additional
//! functions, where `M` is how many types derive `#[reflect(Component)]`.
//!
//! Those functions will increase the size of the final app binary.
//!
//! [^1]: `crates/bevy_reflect/bevy_reflect_derive/src/registration.rs`
//!
//! [`get_type_registration`]: bevy_reflect::GetTypeRegistration::get_type_registration

use super::from_reflect_with_fallback;
use crate::{
    component::{ComponentId, ComponentMutability},
    entity::EntityMapper,
    prelude::Component,
    relationship::RelationshipHookMode,
    world::{EntityWorldMut, FilteredEntityRef, World},
};
use bevy_reflect::{FromType, PartialReflect, Reflect, TypePath, TypeRegistry};

/// A struct used to operate on reflected [`Component`] trait of a type.
///
/// A [`ReflectComponent`] for type `T` can be obtained via
/// [`bevy_reflect::TypeRegistration::data`].
#[derive(Clone)]
pub struct ReflectComponent(ReflectComponentFns);

/// The raw function pointers needed to make up a [`ReflectComponent`].
///
/// > **Note:**
/// > Creating custom implementations of [`ReflectComponent`] is an advanced feature that most users
/// > will not need.
/// > Usually a [`ReflectComponent`] is created for a type by deriving [`Reflect`]
/// > and adding the `#[reflect(Component)]` attribute.
/// > After adding the component to the [`TypeRegistry`],
/// > its [`ReflectComponent`] can then be retrieved when needed.
///
/// Creating a custom [`ReflectComponent`] may be useful if you need to create new component types
/// at runtime, for example, for scripting implementations.
///
/// By creating a custom [`ReflectComponent`] and inserting it into a type's
/// [`TypeRegistration`][bevy_reflect::TypeRegistration],
/// you can modify the way that reflected components of that type will be inserted into the Bevy
/// world.
#[derive(Clone)]
pub struct ReflectComponentFns {
    /// Function pointer implementing [`ReflectComponent::apply_or_insert_mapped()`].
    pub apply_or_insert_mapped: fn(
        &mut EntityWorldMut,
        &dyn PartialReflect,
        &TypeRegistry,
        &mut dyn EntityMapper,
        RelationshipHookMode,
    ),
    /// Function pointer implementing [`ReflectComponent::reflect()`].
    pub reflect: for<'w> fn(FilteredEntityRef<'w, '_>) -> Option<&'w dyn Reflect>,
    /// Function pointer implementing [`ReflectComponent::register_component()`].
    pub register_component: fn(&mut World) -> ComponentId,
}

impl ReflectComponent {
    /// Uses reflection to set the value of this [`Component`] type in the entity to the given value or insert a new one if it does not exist.
    ///
    /// # Panics
    ///
    /// Panics if [`Component`] is immutable.
    pub fn apply_or_insert_mapped(
        &self,
        entity: &mut EntityWorldMut,
        component: &dyn PartialReflect,
        registry: &TypeRegistry,
        map: &mut dyn EntityMapper,
        relationship_hook_mode: RelationshipHookMode,
    ) {
        (self.0.apply_or_insert_mapped)(entity, component, registry, map, relationship_hook_mode);
    }

    /// Gets the value of this [`Component`] type from the entity as a reflected reference.
    pub fn reflect<'w, 's>(
        &self,
        entity: impl Into<FilteredEntityRef<'w, 's>>,
    ) -> Option<&'w dyn Reflect> {
        (self.0.reflect)(entity.into())
    }

    /// Register the type of this [`Component`] in [`World`], returning its [`ComponentId`].
    pub fn register_component(&self, world: &mut World) -> ComponentId {
        (self.0.register_component)(world)
    }
}

impl<C: Component + Reflect + TypePath> FromType<C> for ReflectComponent {
    fn from_type() -> Self {
        ReflectComponent(ReflectComponentFns {
            apply_or_insert_mapped: |entity,
                                     reflected_component,
                                     registry,
                                     mut mapper,
                                     relationship_hook_mode| {
                if C::Mutability::MUTABLE {
                    // SAFETY: guard ensures `C` is a mutable component
                    if let Some(mut component) = unsafe { entity.get_mut_assume_mutable::<C>() } {
                        component.apply(reflected_component.as_partial_reflect());
                        C::map_entities(&mut component, &mut mapper);
                    } else {
                        let mut component = entity.world_scope(|world| {
                            from_reflect_with_fallback::<C>(reflected_component, world, registry)
                        });
                        C::map_entities(&mut component, &mut mapper);
                        entity
                            .insert_with_relationship_hook_mode(component, relationship_hook_mode);
                    }
                } else {
                    let mut component = entity.world_scope(|world| {
                        from_reflect_with_fallback::<C>(reflected_component, world, registry)
                    });
                    C::map_entities(&mut component, &mut mapper);
                    entity.insert_with_relationship_hook_mode(component, relationship_hook_mode);
                }
            },
            reflect: |entity| entity.get::<C>().map(|c| c as &dyn Reflect),
            register_component: |world: &mut World| -> ComponentId {
                world.register_component::<C>()
            },
        })
    }
}
