//! Definitions for [`FromWorld`] reflection.
//! This allows creating instances of types that are known only at runtime and
//! require an `&mut World` to be initialized.
//!
//! This module exports two types: [`ReflectFromWorldFns`] and [`ReflectFromWorld`].
//!
//! Same as [`component`](`super::component`), but for [`FromWorld`].

use alloc::boxed::Box;
use bevy_reflect::{FromType, Reflect};

use crate::world::{FromWorld, World};

/// A struct used to operate on the reflected [`FromWorld`] trait of a type.
///
/// A [`ReflectFromWorld`] for type `T` can be obtained via
/// [`bevy_reflect::TypeRegistration::data`].
#[derive(Clone)]
pub struct ReflectFromWorld(ReflectFromWorldFns);

/// The raw function pointers needed to make up a [`ReflectFromWorld`].
#[derive(Clone)]
pub struct ReflectFromWorldFns {
    /// Function pointer implementing [`ReflectFromWorld::from_world()`].
    pub from_world: fn(&mut World) -> Box<dyn Reflect>,
}

impl ReflectFromWorld {
    /// Constructs default reflected [`FromWorld`] from world using [`from_world()`](FromWorld::from_world).
    pub fn from_world(&self, world: &mut World) -> Box<dyn Reflect> {
        (self.0.from_world)(world)
    }
}

impl<B: Reflect + FromWorld> FromType<B> for ReflectFromWorld {
    fn from_type() -> Self {
        ReflectFromWorld(ReflectFromWorldFns {
            from_world: |world| Box::new(B::from_world(world)),
        })
    }
}
