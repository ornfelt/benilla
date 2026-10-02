#![cfg_attr(
    any(docsrs, docsrs_dep),
    expect(
        internal_features,
        reason = "rustdoc_internals is needed for fake_variadic"
    )
)]
#![cfg_attr(any(docsrs, docsrs_dep), feature(doc_cfg, rustdoc_internals))]
#![doc(
    html_logo_url = "https://bevy.org/assets/icon.png",
    html_favicon_url = "https://bevy.org/assets/icon.png"
)]

//! Reflection in Rust.
//!
//! [Reflection] is a powerful tool provided within many programming languages
//! that allows for meta-programming: using information _about_ the program to
//! _affect_ the program.
//! In other words, reflection allows us to inspect the program itself, its
//! syntax, and its type information at runtime.
//!
//! This crate adds this missing reflection functionality to Rust.
//! Though it was made with the [Bevy] game engine in mind,
//! it's a general-purpose solution that can be used in any Rust project.
//!
//! At a very high level, this crate allows you to:
//! * Dynamically interact with Rust values
//! * Access type metadata at runtime
//! * Serialize and deserialize (i.e. save and load) data
//!
//! It's important to note that because of missing features in Rust,
//! there are some [limitations] with this crate.
//!
//! # The `Reflect` and `PartialReflect` traits
//!
//! At the root of [`bevy_reflect`] is the [`PartialReflect`] trait.
//!
//! Its purpose is to allow dynamic [introspection] of values,
//! following Rust's type system through a system of [subtraits].
//!
//! Its primary purpose is to allow all implementors to be passed around
//! as a `dyn PartialReflect` trait object in one of the following forms:
//! * `&dyn PartialReflect`
//! * `&mut dyn PartialReflect`
//! * `Box<dyn PartialReflect>`
//!
//! This allows values of types implementing `PartialReflect`
//! to be operated upon completely dynamically (at a small [runtime cost]).
//!
//! Building on `PartialReflect` is the [`Reflect`] trait.
//!
//! `PartialReflect` is a supertrait of `Reflect`
//! so any type implementing `Reflect` implements `PartialReflect` by definition.
//! `dyn Reflect` trait objects can be used similarly to `dyn PartialReflect`,
//! but `Reflect` is also often used in trait bounds (like `T: Reflect`).
//!
//! The distinction between `PartialReflect` and `Reflect` is summarized in the following:
//! * `PartialReflect` is a trait for interacting with values under `bevy_reflect`'s data model.
//!   This means values implementing `PartialReflect` can be dynamically constructed and introspected.
//! * The `Reflect` trait, however, ensures that the interface exposed by `PartialReflect`
//!   on types which additionally implement `Reflect` mirrors the structure of a single Rust type.
//! * This means `dyn Reflect` trait objects can be directly downcast to concrete types,
//!   where `dyn PartialReflect` trait object cannot.
//! * `Reflect`, since it provides a stronger type-correctness guarantee,
//!   is the trait used to interact with [the type registry].
//!
//! ## Converting between `PartialReflect` and `Reflect`
//!
//! Since `T: Reflect` implies `T: PartialReflect`, conversion from a `dyn Reflect` to a `dyn PartialReflect`
//! trait object (upcasting) is infallible and can be performed with one of the following methods.
//! Note that these are temporary while [the language feature for dyn upcasting coercion] is experimental:
//! * [`PartialReflect::as_partial_reflect`] for `&dyn PartialReflect`
//! * [`PartialReflect::as_partial_reflect_mut`] for `&mut dyn PartialReflect`
//! * [`PartialReflect::into_partial_reflect`] for `Box<dyn PartialReflect>`
//!
//! For conversion in the other direction — downcasting `dyn PartialReflect` to `dyn Reflect` —
//! there are fallible methods:
//! * [`PartialReflect::try_as_reflect`] for `&dyn Reflect`
//! * [`PartialReflect::try_as_reflect_mut`] for `&mut dyn Reflect`
//! * [`PartialReflect::try_into_reflect`] for `Box<dyn Reflect>`
//!
//! Additionally, [`FromReflect::from_reflect`] can be used to convert a `dyn PartialReflect` to a concrete type
//! which implements `Reflect`.
//!
//! # Implementing `Reflect`
//!
//! Implementing `Reflect` (and `PartialReflect`) is easily done using the provided [derive macro]:
//!
//! ```
//! # use bevy_reflect::Reflect;
//! #[derive(Reflect)]
//! struct MyStruct {
//!   foo: i32
//! }
//! ```
//!
//! This will automatically generate the implementation of `Reflect` for any struct or enum.
//!
//! It will also generate other very important trait implementations used for reflection:
//! * [`GetTypeRegistration`]
//! * [`Typed`]
//! * [`Struct`], [`TupleStruct`], or [`Enum`] depending on the type
//!
//! ## Requirements
//!
//! We can implement `Reflect` on any type that satisfies _both_ of the following conditions:
//! * The type implements `Any`, `Send`, and `Sync`.
//!   For the `Any` requirement to be satisfied, the type itself must have a [`'static` lifetime].
//! * All fields and sub-elements themselves implement `Reflect`
//!   (see the [derive macro documentation] for details on how to ignore certain fields when deriving).
//!
//! Additionally, using the derive macro on enums requires a third condition to be met:
//! * All fields and sub-elements must implement [`FromReflect`]—
//!   another important reflection trait discussed in a later section.
//!
//! # The Reflection Subtraits
//!
//! Since [`PartialReflect`] is meant to cover any and every type, this crate also comes with a few
//! more traits to accompany `PartialReflect` and provide more specific interactions.
//! We refer to these traits as the _reflection subtraits_ since they all have `PartialReflect` as a supertrait.
//! The current list of reflection subtraits include:
//! * [`Tuple`]
//! * [`Array`]
//! * [`List`]
//! * [`Struct`]
//! * [`TupleStruct`]
//! * [`Enum`]
//!
//! As mentioned previously, the last three are automatically implemented by the [derive macro].
//!
//! Each of these traits come with their own methods specific to their respective category.
//! For example, we can access our struct's fields by name using the [`Struct::field`] method.
//!
//! ```
//! # use bevy_reflect::{PartialReflect, Reflect, Struct};
//! # #[derive(Reflect)]
//! # struct MyStruct {
//! #   foo: i32
//! # }
//! let my_struct: Box<dyn Struct> = Box::new(MyStruct {
//!   foo: 123
//! });
//! let foo: &dyn PartialReflect = my_struct.field("foo").unwrap();
//! assert_eq!(Some(&123), foo.try_downcast_ref::<i32>());
//! ```
//!
//! Since most data is passed around as `dyn PartialReflect` or `dyn Reflect` trait objects,
//! the `PartialReflect` trait has methods for going to and from these subtraits.
//!
//! [`PartialReflect::reflect_kind`], [`PartialReflect::reflect_ref`],
//! [`PartialReflect::reflect_mut`], and [`PartialReflect::reflect_owned`] all return
//! an enum that respectively contains zero-sized, immutable, mutable, and owned access to the type as a subtrait object.
//!
//! For example, we can get out a `dyn Tuple` from our reflected tuple type using one of these methods.
//!
//! ```
//! # use bevy_reflect::{PartialReflect, ReflectRef};
//! let my_tuple: Box<dyn PartialReflect> = Box::new((1, 2, 3));
//! let my_tuple = my_tuple.reflect_ref().as_tuple().unwrap();
//! assert_eq!(3, my_tuple.field_len());
//! ```
//!
//! And to go back to a general-purpose `dyn PartialReflect`,
//! we can just use the matching [`PartialReflect::as_partial_reflect`], [`PartialReflect::as_partial_reflect_mut`],
//! or [`PartialReflect::into_partial_reflect`] methods.
//!
//! ## Opaque Types
//!
//! Some types don't fall under a particular subtrait.
//!
//! These types hide their internal structure to reflection,
//! either because it is not possible, difficult, or not useful to reflect its internals.
//! Such types are known as _opaque_ types.
//!
//! This includes truly opaque types like `String` or `Instant`,
//! but also includes all the primitive types (e.g.  `bool`, `usize`, etc.)
//! since they can't be broken down any further.
//!
//! # Dynamic Types
//!
//! Each subtrait comes with a corresponding _dynamic_ type.
//!
//! The available dynamic types are:
//! * [`DynamicTuple`]
//! * [`DynamicArray`]
//! * [`DynamicList`]
//! * [`DynamicStruct`]
//! * [`DynamicTupleStruct`]
//! * [`DynamicEnum`]
//!
//! These dynamic types may contain any arbitrary reflected data.
//!
//! ```
//! # use bevy_reflect::{DynamicStruct, Struct};
//! let mut data = DynamicStruct::default();
//! data.insert("foo", 123_i32);
//! assert_eq!(Some(&123), data.field("foo").unwrap().try_downcast_ref::<i32>())
//! ```
//!
//! They are most commonly used as "proxies" for other types,
//! where they contain the same data as— and therefore, represent— a concrete type.
//! The [`PartialReflect::to_dynamic`] method will return a dynamic type for all non-opaque types,
//! allowing all types to essentially be "cloned" into a dynamic type.
//! And since dynamic types themselves implement [`PartialReflect`],
//! we may pass them around just like most other reflected types.
//!
//! ```
//! # use bevy_reflect::{DynamicStruct, PartialReflect, Reflect};
//! # #[derive(Reflect)]
//! # struct MyStruct {
//! #   foo: i32
//! # }
//! let original: Box<dyn Reflect> = Box::new(MyStruct {
//!   foo: 123
//! });
//!
//! // `dynamic` will be a `DynamicStruct` representing a `MyStruct`
//! let dynamic: Box<dyn PartialReflect> = original.to_dynamic();
//! assert!(dynamic.represents::<MyStruct>());
//! ```
//!
//! ## Patching
//!
//! These dynamic types come in handy when needing to apply multiple changes to another type.
//! This is known as "patching" and is done using the [`PartialReflect::apply`] and [`PartialReflect::try_apply`] methods.
//!
//! ```
//! # use bevy_reflect::{DynamicEnum, PartialReflect};
//! let mut value = Some(123_i32);
//! let patch = DynamicEnum::new("None", ());
//! value.apply(&patch);
//! assert_eq!(None, value);
//! ```
//!
//! ## `FromReflect`
//!
//! It's important to remember that dynamic types are _not_ the concrete type they may be representing.
//! A common mistake is to treat them like such when trying to cast back to the original type
//! or when trying to make use of a reflected trait which expects the actual type.
//!
//! ```should_panic
//! # use bevy_reflect::{DynamicStruct, PartialReflect, Reflect};
//! # #[derive(Reflect)]
//! # struct MyStruct {
//! #   foo: i32
//! # }
//! let original: Box<dyn Reflect> = Box::new(MyStruct {
//!   foo: 123
//! });
//!
//! let dynamic: Box<dyn PartialReflect> = original.to_dynamic();
//! let value = dynamic.try_take::<MyStruct>().unwrap(); // PANIC!
//! ```
//!
//! To resolve this issue, we'll need to convert the dynamic type to the concrete one.
//! This is where [`FromReflect`] comes in.
//!
//! `FromReflect` is a trait that allows an instance of a type to be generated from a
//! dynamic representation— even partial ones.
//! And since the [`FromReflect::from_reflect`] method takes the data by reference,
//! this can be used to effectively clone data (to an extent).
//!
//! It is automatically implemented when [deriving `Reflect`] on a type unless opted out of
//! using `#[reflect(from_reflect = false)]` on the item.
//!
//! ```
//! # use bevy_reflect::{FromReflect, PartialReflect, Reflect};
//! #[derive(Reflect)]
//! struct MyStruct {
//!   foo: i32
//! }
//! let original: Box<dyn Reflect> = Box::new(MyStruct {
//!   foo: 123
//! });
//!
//! let dynamic: Box<dyn PartialReflect> = original.to_dynamic();
//! let value = <MyStruct as FromReflect>::from_reflect(&*dynamic).unwrap(); // OK!
//! ```
//!
//! When deriving, all active fields and sub-elements must also implement `FromReflect`.
//!
//! Ignored fields must implement [`Default`].
//!
//! See the [derive macro documentation](derive@crate::FromReflect) for details.
//!
//! All primitives and simple types implement `FromReflect` by relying on their [`Default`] implementation.
//!
//! # Type Registration
//!
//! This crate also comes with a [`TypeRegistry`] that can be used to store and retrieve additional type metadata at runtime,
//! such as helper types and trait implementations.
//!
//! The [derive macro] for [`Reflect`] also generates an implementation of the [`GetTypeRegistration`] trait,
//! which is used by the registry to generate a [`TypeRegistration`] struct for that type.
//! We can then register additional [type data] we want associated with that type.
//!
//! For example, we can register [`ReflectDefault`] on our type so that its `Default` implementation
//! may be used dynamically.
//!
//! ```
//! # use bevy_reflect::{Reflect, TypeRegistry, prelude::ReflectDefault};
//! #[derive(Reflect, Default)]
//! struct MyStruct {
//!   foo: i32
//! }
//! let mut registry = TypeRegistry::empty();
//! registry.register::<MyStruct>();
//! registry.register_type_data::<MyStruct, ReflectDefault>();
//!
//! let registration = registry.get(core::any::TypeId::of::<MyStruct>()).unwrap();
//! let reflect_default = registration.data::<ReflectDefault>().unwrap();
//!
//! let new_value: Box<dyn Reflect> = reflect_default.default();
//! assert!(new_value.is::<MyStruct>());
//! ```
//!
//! Because this operation is so common, the derive macro actually has a shorthand for it.
//! By using the `#[reflect(Trait)]` attribute, the derive macro will automatically register a matching,
//! in-scope `ReflectTrait` type within the `GetTypeRegistration` implementation.
//!
//! ```
//! use bevy_reflect::prelude::{Reflect, ReflectDefault};
//!
//! #[derive(Reflect, Default)]
//! #[reflect(Default)]
//! struct MyStruct {
//!   foo: i32
//! }
//! ```
//!
//! # Limitations
//!
//! While this crate offers a lot in terms of adding reflection to Rust,
//! it does come with some limitations that don't make it as featureful as reflection
//! in other programming languages.
//!
//! ## Non-Static Lifetimes
//!
//! One of the most obvious limitations is the `'static` requirement.
//! Rust requires fields to define a lifetime for referenced data,
//! but [`Reflect`] requires all types to have a `'static` lifetime.
//! This makes it impossible to reflect any type with non-static borrowed data.
//!
//! ## Generic Function Reflection
//!
//! Another limitation is the inability to reflect over generic functions directly. It can be done, but will
//! typically require manual monomorphization (i.e. manually specifying the types the generic method can
//! take).
//!
//! # Features
//!
//! ## `bevy`
//!
//! | Default | Dependencies                                        |
//! | :-----: | :-------------------------------------------------: |
//! | ❌      | [`bevy_math`], [`glam`], [`indexmap`], [`smallvec`] |
//!
//! This feature makes it so that the appropriate reflection traits are implemented on all the types
//! necessary for the [Bevy] game engine.
//! enables the optional dependencies: [`bevy_math`], [`glam`], [`indexmap`], and [`smallvec`].
//! These dependencies are used by the [Bevy] game engine and must define their reflection implementations
//! within this crate due to Rust's [orphan rule].
//!
//! ## `debug`
//!
//! | Default | Dependencies                                  |
//! | :-----: | :-------------------------------------------: |
//! | ✅      | `debug_stack`                                 |
//!
//! This feature enables useful debug features for reflection.
//!
//! It includes the `debug_stack` feature, which no longer enables anything in this copy.
//!
//! [Reflection]: https://en.wikipedia.org/wiki/Reflective_programming
//! [Bevy]: https://bevy.org/
//! [limitations]: #limitations
//! [`bevy_reflect`]: crate
//! [introspection]: https://en.wikipedia.org/wiki/Type_introspection
//! [subtraits]: #the-reflection-subtraits
//! [the type registry]: #type-registration
//! [runtime cost]: https://doc.rust-lang.org/book/ch17-02-trait-objects.html#trait-objects-perform-dynamic-dispatch
//! [the language feature for dyn upcasting coercion]: https://github.com/rust-lang/rust/issues/65991
//! [derive macro]: derive@crate::Reflect
//! [`'static` lifetime]: https://doc.rust-lang.org/rust-by-example/scope/lifetime/static_lifetime.html#trait-bound
//! [derive macro documentation]: derive@crate::Reflect
//! [deriving `Reflect`]: derive@crate::Reflect
//! [type data]: TypeData
//! [`ReflectDefault`]: std_traits::ReflectDefault
//! [object-safe]: https://doc.rust-lang.org/reference/items/traits.html#object-safety
//! [type registry]: TypeRegistry
//! [`bevy_math`]: https://docs.rs/bevy_math/latest/bevy_math/
//! [`glam`]: https://docs.rs/glam/latest/glam/
//! [`smallvec`]: https://docs.rs/smallvec/latest/smallvec/
//! [`indexmap`]: https://docs.rs/indexmap/latest/indexmap/
//! [orphan rule]: https://doc.rust-lang.org/book/ch10-02-traits.html#implementing-a-trait-on-a-type:~:text=But%20we%20can%E2%80%99t,implementation%20to%20use.

#![no_std]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

// Required to make proc macros work in bevy itself.
extern crate self as bevy_reflect;

mod array;
mod error;
mod fields;
mod from_reflect;
mod is;
mod kind;
mod list;
mod reflect;
mod reflectable;
mod struct_trait;
mod tuple;
mod tuple_struct;
mod type_info;
mod type_path;
mod type_registry;

mod impls {
    mod alloc;
    mod core;
    mod macros;

    #[cfg(feature = "glam")]
    mod glam;
}

mod enums;
mod generics;
pub mod std_traits;
pub mod utility;

/// The reflect prelude.
///
/// This includes the most common types in this crate, re-exported for your convenience.
pub mod prelude {
    pub use crate::std_traits::*;

    #[doc(hidden)]
    pub use crate::{
        FromReflect, GetField, GetTupleStructField, PartialReflect, Reflect, ReflectFromReflect,
        Struct, TupleStruct, TypePath,
    };
}

pub use array::*;
pub use enums::*;
pub use error::*;
pub use fields::*;
pub use from_reflect::*;
pub use generics::*;
pub use is::*;
pub use kind::*;
pub use list::*;
pub use reflect::*;
pub use reflectable::*;
pub use struct_trait::*;
pub use tuple::*;
pub use tuple_struct::*;
pub use type_info::*;
pub use type_path::*;
pub use type_registry::*;

pub use bevy_reflect_derive::*;

/// Exports used by the reflection macros.
///
/// These are not meant to be used directly and are subject to breaking changes.
#[doc(hidden)]
pub mod __macro_exports {
    use crate::{
        DynamicArray, DynamicEnum, DynamicList, DynamicStruct, DynamicTuple, DynamicTupleStruct,
        GetTypeRegistration, TypeRegistry,
    };

    /// Re-exports of items from the [`alloc`] crate.
    ///
    /// This is required because in `std` environments (e.g., the `std` feature is enabled)
    /// the `alloc` crate may not have been included, making its namespace unreliable.
    pub mod alloc_utils {
        pub use ::alloc::{
            borrow::{Cow, ToOwned},
            boxed::Box,
            string::ToString,
        };
    }

    /// A wrapper trait around [`GetTypeRegistration`].
    ///
    /// This trait is used by the derive macro to recursively register all type dependencies.
    /// It's used instead of `GetTypeRegistration` directly to avoid making dynamic types also
    /// implement `GetTypeRegistration` in order to be used as active fields.
    ///
    /// This trait has a blanket implementation for all types that implement `GetTypeRegistration`
    /// and manual implementations for all dynamic types (which simply do nothing).
    #[diagnostic::on_unimplemented(
        message = "`{Self}` does not implement `GetTypeRegistration` so cannot be registered for reflection",
        note = "consider annotating `{Self}` with `#[derive(Reflect)]`"
    )]
    pub trait RegisterForReflection {
        #[expect(
            unused_variables,
            reason = "The parameters here are intentionally unused by the default implementation; however, putting underscores here will result in the underscores being copied by rust-analyzer's tab completion."
        )]
        fn __register(registry: &mut TypeRegistry) {}
    }

    impl<T: GetTypeRegistration> RegisterForReflection for T {
        fn __register(registry: &mut TypeRegistry) {
            registry.register::<T>();
        }
    }

    impl RegisterForReflection for DynamicEnum {}

    impl RegisterForReflection for DynamicTupleStruct {}

    impl RegisterForReflection for DynamicStruct {}

    impl RegisterForReflection for DynamicList {}

    impl RegisterForReflection for DynamicArray {}

    impl RegisterForReflection for DynamicTuple {}
}

#[cfg(test)]
#[expect(
    clippy::approx_constant,
    reason = "We don't need the exact value of Pi here."
)]
mod tests {
    use alloc::{
        borrow::Cow,
        boxed::Box,
        format,
        string::{String, ToString},
        vec,
        vec::Vec,
    };
    use core::{
        any::TypeId,
        fmt::{Debug, Formatter},
        hash::Hash,
        marker::PhantomData,
    };
    use static_assertions::assert_impl_all;

    use super::{prelude::*, *};

    #[test]
    fn try_apply_should_detect_kinds() {
        #[derive(Reflect, Debug)]
        struct Struct {
            a: u32,
            b: f32,
        }

        #[derive(Reflect, Debug)]
        enum Enum {
            A,
            B(u32),
        }

        let mut struct_target = Struct {
            a: 0xDEADBEEF,
            b: 3.14,
        };

        let mut enum_target = Enum::A;

        let array_src = [8, 0, 8];

        let result = struct_target.try_apply(&enum_target);
        assert!(
            matches!(
                result,
                Err(ApplyError::MismatchedKinds {
                    from_kind: ReflectKind::Enum,
                    to_kind: ReflectKind::Struct
                })
            ),
            "result was {result:?}"
        );

        let result = enum_target.try_apply(&array_src);
        assert!(
            matches!(
                result,
                Err(ApplyError::MismatchedKinds {
                    from_kind: ReflectKind::Array,
                    to_kind: ReflectKind::Enum
                })
            ),
            "result was {result:?}"
        );
    }

    #[test]
    fn reflect_struct() {
        #[derive(Reflect)]
        struct Foo {
            a: u32,
            b: f32,
            c: Bar,
        }
        #[derive(Reflect)]
        struct Bar {
            x: u32,
        }

        let mut foo = Foo {
            a: 42,
            b: 3.14,
            c: Bar { x: 1 },
        };

        let a = *foo.get_field::<u32>("a").unwrap();
        assert_eq!(a, 42);

        *foo.get_field_mut::<u32>("a").unwrap() += 1;
        assert_eq!(foo.a, 43);

        let bar = foo.get_field::<Bar>("c").unwrap();
        assert_eq!(bar.x, 1);

        // nested retrieval
        let c = foo.field("c").unwrap();
        let value = c.reflect_ref().as_struct().unwrap();
        assert_eq!(*value.get_field::<u32>("x").unwrap(), 1);

        // patch Foo with a dynamic struct
        let mut dynamic_struct = DynamicStruct::default();
        dynamic_struct.insert_boxed("a", Box::new(123u32));
        dynamic_struct.insert_boxed("should_be_ignored", Box::new(456));

        foo.apply(&dynamic_struct);
        assert_eq!(foo.a, 123);
    }

    #[test]
    fn reflect_unit_struct() {
        #[derive(Reflect)]
        struct Foo(u32, u64);

        let mut foo = Foo(1, 2);
        assert_eq!(1, *foo.get_field::<u32>(0).unwrap());
        assert_eq!(2, *foo.get_field::<u64>(1).unwrap());

        let mut patch = DynamicTupleStruct::default();
        patch.insert_boxed(Box::new(3u32));
        patch.insert_boxed(Box::new(4u64));
        assert_eq!(
            3,
            *patch.field(0).unwrap().try_downcast_ref::<u32>().unwrap()
        );
        assert_eq!(
            4,
            *patch.field(1).unwrap().try_downcast_ref::<u64>().unwrap()
        );

        foo.apply(&patch);
        assert_eq!(3, foo.0);
        assert_eq!(4, foo.1);

        let mut iter = patch.iter_fields();
        assert_eq!(3, *iter.next().unwrap().try_downcast_ref::<u32>().unwrap());
        assert_eq!(4, *iter.next().unwrap().try_downcast_ref::<u64>().unwrap());
    }

    #[test]
    fn reflect_ignore() {
        #[derive(Reflect)]
        struct Foo {
            a: u32,
            #[reflect(ignore)]
            _b: u32,
        }

        let foo = Foo { a: 1, _b: 2 };

        let values: Vec<u32> = foo
            .iter_fields()
            .map(|value| *value.try_downcast_ref::<u32>().unwrap())
            .collect();
        assert_eq!(values, vec![1]);
    }

    /// This test ensures that we are able to reflect generic types with one or more type parameters.
    ///
    /// When there is an `Add` implementation for `String`, the compiler isn't able to infer the correct
    /// type to deref to.
    /// If we don't append the strings in the `TypePath` derive correctly (i.e. explicitly specifying the type),
    /// we'll get a compilation error saying that "`&String` cannot be added to `String`".
    ///
    /// So this test just ensures that we do that correctly.
    ///
    /// This problem is a known issue and is unexpectedly expected behavior:
    /// - <https://github.com/rust-lang/rust/issues/77143>
    /// - <https://github.com/bodil/smartstring/issues/7>
    /// - <https://github.com/pola-rs/polars/issues/14666>
    #[test]
    fn should_reflect_generic() {
        struct FakeString {}

        // This implementation confuses the compiler when trying to add a `&String` to a `String`
        impl core::ops::Add<FakeString> for String {
            type Output = Self;
            fn add(self, _rhs: FakeString) -> Self::Output {
                unreachable!()
            }
        }

        #[expect(
            dead_code,
            reason = "This struct is used as a compilation test to test the derive macros, and as such is intentionally never constructed."
        )]
        #[derive(Reflect)]
        struct Foo<A>(A);

        #[expect(
            dead_code,
            reason = "This struct is used as a compilation test to test the derive macros, and as such is intentionally never constructed."
        )]
        #[derive(Reflect)]
        struct Bar<A, B>(A, B);

        #[expect(
            dead_code,
            reason = "This struct is used as a compilation test to test the derive macros, and as such is intentionally never constructed."
        )]
        #[derive(Reflect)]
        struct Baz<A, B, C>(A, B, C);
    }

    #[test]
    fn should_reflect_clone() {
        // Struct
        #[derive(Reflect, Debug, PartialEq)]
        struct Foo(usize);

        let value = Foo(123);
        let clone = value.reflect_clone().expect("should reflect_clone struct");
        assert_eq!(value, clone.take::<Foo>().unwrap());

        // Tuple
        let foo = (123, 4.56);
        let clone = foo.reflect_clone().expect("should reflect_clone tuple");
        assert_eq!(foo, clone.take::<(u32, f32)>().unwrap());
    }

    #[test]
    fn should_not_clone_ignored_fields() {
        // Tuple Struct
        #[derive(Reflect, Clone, Debug, PartialEq)]
        struct Foo(#[reflect(ignore)] usize);

        let foo = Foo(123);
        let clone = foo.reflect_clone();
        assert_eq!(
            clone.unwrap_err(),
            ReflectCloneError::FieldNotCloneable {
                field: FieldId::Unnamed(0),
                variant: None,
                container_type_path: Cow::Borrowed(Foo::type_path()),
            }
        );

        // Struct
        #[derive(Reflect, Clone, Debug, PartialEq)]
        struct Bar {
            #[reflect(ignore)]
            value: usize,
        }

        let bar = Bar { value: 123 };
        let clone = bar.reflect_clone();
        assert_eq!(
            clone.unwrap_err(),
            ReflectCloneError::FieldNotCloneable {
                field: FieldId::Named(Cow::Borrowed("value")),
                variant: None,
                container_type_path: Cow::Borrowed(Bar::type_path()),
            }
        );

        // Enum
        #[derive(Reflect, Clone, Debug, PartialEq)]
        enum Baz {
            Tuple(#[reflect(ignore)] usize),
            Struct {
                #[reflect(ignore)]
                value: usize,
            },
        }

        let baz = Baz::Tuple(123);
        let clone = baz.reflect_clone();
        assert_eq!(
            clone.unwrap_err(),
            ReflectCloneError::FieldNotCloneable {
                field: FieldId::Unnamed(0),
                variant: Some(Cow::Borrowed("Tuple")),
                container_type_path: Cow::Borrowed(Baz::type_path()),
            }
        );

        let baz = Baz::Struct { value: 123 };
        let clone = baz.reflect_clone();
        assert_eq!(
            clone.unwrap_err(),
            ReflectCloneError::FieldNotCloneable {
                field: FieldId::Named(Cow::Borrowed("value")),
                variant: Some(Cow::Borrowed("Struct")),
                container_type_path: Cow::Borrowed(Baz::type_path()),
            }
        );
    }

    #[test]
    fn should_call_from_reflect_dynamically() {
        #[derive(Reflect)]
        struct MyStruct {
            foo: usize,
        }

        // Register
        let mut registry = TypeRegistry::default();
        registry.register::<MyStruct>();

        // Get type data
        let type_id = TypeId::of::<MyStruct>();
        let rfr = registry
            .get_type_data::<ReflectFromReflect>(type_id)
            .expect("the FromReflect trait should be registered");

        // Call from_reflect
        let mut dynamic_struct = DynamicStruct::default();
        dynamic_struct.insert_boxed("foo", Box::new(123usize));
        let reflected = rfr
            .from_reflect(&dynamic_struct)
            .expect("the type should be properly reflected");

        // Assert
        let expected = MyStruct { foo: 123 };
        assert!(expected
            .reflect_partial_eq(reflected.as_partial_reflect())
            .unwrap_or_default());
        let not_expected = MyStruct { foo: 321 };
        assert!(!not_expected
            .reflect_partial_eq(reflected.as_partial_reflect())
            .unwrap_or_default());
    }

    #[test]
    fn from_reflect_should_allow_ignored_unnamed_fields() {
        #[derive(Reflect, Eq, PartialEq, Debug)]
        struct MyTupleStruct(i8, #[reflect(ignore)] i16, i32);

        let expected = MyTupleStruct(1, 0, 3);

        let mut dyn_tuple_struct = DynamicTupleStruct::default();
        dyn_tuple_struct.insert_boxed(Box::new(1_i8));
        dyn_tuple_struct.insert_boxed(Box::new(3_i32));
        let my_tuple_struct = <MyTupleStruct as FromReflect>::from_reflect(&dyn_tuple_struct);

        assert_eq!(Some(expected), my_tuple_struct);

        #[derive(Reflect, Eq, PartialEq, Debug)]
        enum MyEnum {
            Tuple(i8, #[reflect(ignore)] i16, i32),
        }

        let expected = MyEnum::Tuple(1, 0, 3);

        let mut dyn_tuple = DynamicTuple::default();
        dyn_tuple.insert_boxed(Box::new(1_i8));
        dyn_tuple.insert_boxed(Box::new(3_i32));

        let mut dyn_enum = DynamicEnum::default();
        dyn_enum.set_variant("Tuple", dyn_tuple);

        let my_enum = <MyEnum as FromReflect>::from_reflect(&dyn_enum);

        assert_eq!(Some(expected), my_enum);
    }

    #[test]
    fn from_reflect_should_use_default_container_attribute() {
        #[derive(Reflect, Eq, PartialEq, Debug)]
        #[reflect(Default)]
        struct MyStruct {
            foo: String,
            #[reflect(ignore)]
            bar: usize,
        }

        impl Default for MyStruct {
            fn default() -> Self {
                Self {
                    foo: String::from("Hello"),
                    bar: 123,
                }
            }
        }

        let expected = MyStruct {
            foo: String::from("Hello"),
            bar: 123,
        };

        let dyn_struct = DynamicStruct::default();
        let my_struct = <MyStruct as FromReflect>::from_reflect(&dyn_struct);

        assert_eq!(Some(expected), my_struct);
    }

    #[test]
    fn reflect_complex_patch() {
        #[derive(Reflect, Eq, PartialEq, Debug)]
        #[reflect(PartialEq)]
        struct Foo {
            a: u32,
            #[reflect(ignore)]
            _b: u32,
            c: Vec<isize>,
            e: Bar,
            f: (i32, Vec<isize>, Bar),
            g: Vec<(Baz,)>,
            h: [u32; 2],
        }

        #[derive(Reflect, Eq, PartialEq, Clone, Debug)]
        #[reflect(PartialEq)]
        struct Bar {
            x: u32,
        }

        #[derive(Reflect, Eq, PartialEq, Debug)]
        struct Baz(String);

        let mut foo = Foo {
            a: 1,
            _b: 1,
            c: vec![1, 2],
            e: Bar { x: 1 },
            f: (1, vec![1, 2], Bar { x: 1 }),
            g: vec![(Baz("string".to_string()),)],
            h: [2; 2],
        };

        let mut foo_patch = DynamicStruct::default();
        foo_patch.insert_boxed("a", Box::new(2u32));
        foo_patch.insert_boxed("b", Box::new(2u32)); // this should be ignored

        let mut list = DynamicList::default();
        list.push_box(Box::new(3isize));
        list.push_box(Box::new(4isize));
        list.push_box(Box::new(5isize));
        foo_patch.insert_boxed("c", Box::new(list.to_dynamic_list()));

        let mut bar_patch = DynamicStruct::default();
        bar_patch.insert_boxed("x", Box::new(2u32));
        foo_patch.insert_boxed("e", Box::new(bar_patch.to_dynamic_struct()));

        let mut tuple = DynamicTuple::default();
        tuple.insert_boxed(Box::new(2i32));
        tuple.insert_boxed(Box::new(list));
        tuple.insert_boxed(Box::new(bar_patch));
        foo_patch.insert_boxed("f", Box::new(tuple));

        let mut composite = DynamicList::default();
        composite.push_box(Box::new({
            let mut tuple = DynamicTuple::default();
            tuple.insert_boxed(Box::new({
                let mut tuple_struct = DynamicTupleStruct::default();
                tuple_struct.insert_boxed(Box::new("new_string".to_string()));
                tuple_struct
            }));
            tuple
        }));
        foo_patch.insert_boxed("g", Box::new(composite));

        let array = DynamicArray::from_iter([2u32, 2u32]);
        foo_patch.insert_boxed("h", Box::new(array));

        foo.apply(&foo_patch);

        let expected_foo = Foo {
            a: 2,
            _b: 1,
            c: vec![3, 4, 5],
            e: Bar { x: 2 },
            f: (2, vec![3, 4, 5], Bar { x: 2 }),
            g: vec![(Baz("new_string".to_string()),)],
            h: [2; 2],
        };

        assert_eq!(foo, expected_foo);

        let new_foo = Foo::from_reflect(&foo_patch)
            .expect("error while creating a concrete type from a dynamic type");

        let expected_new_foo = Foo {
            a: 2,
            _b: 0,
            c: vec![3, 4, 5],
            e: Bar { x: 2 },
            f: (2, vec![3, 4, 5], Bar { x: 2 }),
            g: vec![(Baz("new_string".to_string()),)],
            h: [2; 2],
        };

        assert_eq!(new_foo, expected_new_foo);
    }

    #[test]
    fn should_auto_register_fields() {
        #[derive(Reflect)]
        struct Foo {
            bar: Bar,
        }

        #[derive(Reflect)]
        enum Bar {
            Variant(Baz),
        }

        #[derive(Reflect)]
        struct Baz(usize);

        // === Basic === //
        let mut registry = TypeRegistry::empty();
        registry.register::<Foo>();

        assert!(
            registry.get(TypeId::of::<Bar>()).is_some(),
            "registry should contain auto-registered `Bar` from `Foo`"
        );

        // === Option === //
        let mut registry = TypeRegistry::empty();
        registry.register::<Option<Foo>>();

        assert!(
            registry.get(TypeId::of::<Bar>()).is_some(),
            "registry should contain auto-registered `Bar` from `Option<Foo>`"
        );

        // === Tuple === //
        let mut registry = TypeRegistry::empty();
        registry.register::<(Foo, Foo)>();

        assert!(
            registry.get(TypeId::of::<Bar>()).is_some(),
            "registry should contain auto-registered `Bar` from `(Foo, Foo)`"
        );

        // === Array === //
        let mut registry = TypeRegistry::empty();
        registry.register::<[Foo; 3]>();

        assert!(
            registry.get(TypeId::of::<Bar>()).is_some(),
            "registry should contain auto-registered `Bar` from `[Foo; 3]`"
        );

        // === Vec === //
        let mut registry = TypeRegistry::empty();
        registry.register::<Vec<Foo>>();

        assert!(
            registry.get(TypeId::of::<Bar>()).is_some(),
            "registry should contain auto-registered `Bar` from `Vec<Foo>`"
        );
    }

    #[test]
    fn should_allow_dynamic_fields() {
        #[derive(Reflect)]
        #[reflect(from_reflect = false)]
        struct MyStruct(
            DynamicEnum,
            DynamicTupleStruct,
            DynamicStruct,
            DynamicList,
            DynamicArray,
            DynamicTuple,
            i32,
        );

        assert_impl_all!(MyStruct: Reflect, GetTypeRegistration);

        let mut registry = TypeRegistry::empty();
        registry.register::<MyStruct>();

        assert!(registry.get(TypeId::of::<MyStruct>()).is_some());
        assert!(registry.get(TypeId::of::<i32>()).is_some());
    }

    #[test]
    fn should_not_auto_register_existing_types() {
        #[derive(Reflect)]
        struct Foo {
            bar: Bar,
        }

        #[derive(Reflect, Default)]
        struct Bar(usize);

        let mut registry = TypeRegistry::empty();
        registry.register::<Bar>();
        registry
            .get_mut(TypeId::of::<Bar>())
            .unwrap()
            .register_type_data::<ReflectDefault, Bar>();
        registry.register::<Foo>();

        assert!(
            registry
                .get_type_data::<ReflectDefault>(TypeId::of::<Bar>())
                .is_some(),
            "registry should contain existing registration for `Bar`"
        );
    }

    #[test]
    fn reflect_downcast() {
        #[derive(Reflect, Clone, Debug, PartialEq)]
        struct Bar {
            y: u8,
        }

        #[derive(Reflect, Clone, Debug, PartialEq)]
        struct Foo {
            x: i32,
            s: String,
            b: Bar,
            u: usize,
            t: ([f32; 3], String),
            v: Cow<'static, str>,
            w: Cow<'static, [u8]>,
        }

        let foo = Foo {
            x: 123,
            s: "String".to_string(),
            b: Bar { y: 255 },
            u: 1111111111111,
            t: ([3.0, 2.0, 1.0], "Tuple String".to_string()),
            v: Cow::Owned("Cow String".to_string()),
            w: Cow::Owned(vec![1, 2, 3]),
        };

        let foo2: Box<dyn Reflect> = Box::new(foo.clone());

        assert_eq!(foo, *foo2.downcast::<Foo>().unwrap());
    }

    #[test]
    fn should_drain_fields() {
        let array_value: Box<dyn Array> = Box::new([123_i32, 321_i32]);
        let fields = array_value.drain();
        assert!(fields[0].reflect_partial_eq(&123_i32).unwrap_or_default());
        assert!(fields[1].reflect_partial_eq(&321_i32).unwrap_or_default());

        let mut list_value: Box<dyn List> = Box::new(vec![123_i32, 321_i32]);
        let fields = list_value.drain();
        assert!(fields[0].reflect_partial_eq(&123_i32).unwrap_or_default());
        assert!(fields[1].reflect_partial_eq(&321_i32).unwrap_or_default());

        let tuple_value: Box<dyn Tuple> = Box::new((123_i32, 321_i32));
        let fields = tuple_value.drain();
        assert!(fields[0].reflect_partial_eq(&123_i32).unwrap_or_default());
        assert!(fields[1].reflect_partial_eq(&321_i32).unwrap_or_default());
    }

    #[test]
    fn reflect_take() {
        #[derive(Reflect, Debug, PartialEq)]
        #[reflect(PartialEq)]
        struct Bar {
            x: u32,
        }

        let x: Box<dyn Reflect> = Box::new(Bar { x: 2 });
        let y = x.take::<Bar>().unwrap();
        assert_eq!(y, Bar { x: 2 });
    }

    #[test]
    fn not_dynamic_names() {
        let list = Vec::<usize>::new();
        let dyn_list = list.to_dynamic_list();
        assert_ne!(dyn_list.reflect_type_path(), Vec::<usize>::type_path());

        let array = [b'0'; 4];
        let dyn_array = array.to_dynamic_array();
        assert_ne!(dyn_array.reflect_type_path(), <[u8; 4]>::type_path());

        let tuple = (0usize, "1".to_string(), 2.0f32);
        let mut dyn_tuple = tuple.to_dynamic_tuple();
        dyn_tuple.insert_boxed(Box::<usize>::new(3));
        assert_ne!(
            dyn_tuple.reflect_type_path(),
            <(usize, String, f32, usize)>::type_path()
        );

        #[derive(Reflect)]
        struct TestStruct {
            a: usize,
        }
        let struct_ = TestStruct { a: 0 };
        let dyn_struct = struct_.to_dynamic_struct();
        assert_ne!(dyn_struct.reflect_type_path(), TestStruct::type_path());

        #[derive(Reflect)]
        struct TestTupleStruct(usize);
        let tuple_struct = TestTupleStruct(0);
        let dyn_tuple_struct = tuple_struct.to_dynamic_tuple_struct();
        assert_ne!(
            dyn_tuple_struct.reflect_type_path(),
            TestTupleStruct::type_path()
        );
    }

    macro_rules! assert_type_paths {
        ($($ty:ty => $long:literal, $short:literal,)*) => {
            $(
                assert_eq!(<$ty as TypePath>::type_path(), $long);
                assert_eq!(<$ty as TypePath>::short_type_path(), $short);
            )*
        };
    }

    #[test]
    fn reflect_type_path() {
        #[derive(TypePath)]
        struct Param;

        #[derive(TypePath)]
        struct Derive;

        #[derive(TypePath)]
        #[type_path = "my_alias"]
        struct DerivePath;

        #[derive(TypePath)]
        #[type_path = "my_alias"]
        #[type_name = "MyDerivePathName"]
        struct DerivePathName;

        #[derive(TypePath)]
        struct DeriveG<T>(PhantomData<T>);

        #[derive(TypePath)]
        #[type_path = "my_alias"]
        struct DerivePathG<T, const N: usize>(PhantomData<T>);

        #[derive(TypePath)]
        #[type_path = "my_alias"]
        #[type_name = "MyDerivePathNameG"]
        struct DerivePathNameG<T>(PhantomData<T>);

        struct Macro;
        impl_type_path!((in my_alias) Macro);

        struct MacroName;
        impl_type_path!((in my_alias as MyMacroName) MacroName);

        struct MacroG<T, const N: usize>(PhantomData<T>);
        impl_type_path!((in my_alias) MacroG<T, const N: usize>);

        struct MacroNameG<T>(PhantomData<T>);
        impl_type_path!((in my_alias as MyMacroNameG) MacroNameG<T>);

        assert_type_paths! {
            Derive => "bevy_reflect::tests::Derive", "Derive",
            DerivePath => "my_alias::DerivePath", "DerivePath",
            DerivePathName => "my_alias::MyDerivePathName", "MyDerivePathName",
            DeriveG<Param> => "bevy_reflect::tests::DeriveG<bevy_reflect::tests::Param>", "DeriveG<Param>",
            DerivePathG<Param, 10> => "my_alias::DerivePathG<bevy_reflect::tests::Param, 10>", "DerivePathG<Param, 10>",
            DerivePathNameG<Param> => "my_alias::MyDerivePathNameG<bevy_reflect::tests::Param>", "MyDerivePathNameG<Param>",
            Macro => "my_alias::Macro", "Macro",
            MacroName => "my_alias::MyMacroName", "MyMacroName",
            MacroG<Param, 10> => "my_alias::MacroG<bevy_reflect::tests::Param, 10>", "MacroG<Param, 10>",
            MacroNameG<Param> => "my_alias::MyMacroNameG<bevy_reflect::tests::Param>", "MyMacroNameG<Param>",
        }
    }

    #[test]
    fn std_type_paths() {
        #[derive(Clone)]
        struct Type;

        impl TypePath for Type {
            fn type_path() -> &'static str {
                // for brevity in tests
                "Long"
            }

            fn short_type_path() -> &'static str {
                "Short"
            }
        }

        assert_type_paths! {
            u8 => "u8", "u8",
            Type => "Long", "Short",
            &Type => "&Long", "&Short",
            [Type] => "[Long]", "[Short]",
            &[Type] => "&[Long]", "&[Short]",
            [Type; 0] => "[Long; 0]", "[Short; 0]",
            [Type; 100] => "[Long; 100]", "[Short; 100]",
            () => "()", "()",
            (Type,) => "(Long,)", "(Short,)",
            (Type, Type) => "(Long, Long)", "(Short, Short)",
            (Type, Type, Type) => "(Long, Long, Long)", "(Short, Short, Short)",
            Cow<'static, Type> => "alloc::borrow::Cow<Long>", "Cow<Short>",
        }
    }

    #[test]
    fn get_represented_kind_info() {
        #[derive(Reflect)]
        struct SomeStruct;

        #[derive(Reflect)]
        struct SomeTupleStruct(f32);

        #[derive(Reflect)]
        enum SomeEnum {
            Foo,
            Bar,
        }

        let dyn_struct: &dyn Struct = &SomeStruct;
        let _: &StructInfo = dyn_struct.get_represented_struct_info().unwrap();

        let dyn_array: &dyn Array = &[1, 2, 3];
        let _: &ArrayInfo = dyn_array.get_represented_array_info().unwrap();

        let dyn_list: &dyn List = &vec![1, 2, 3];
        let _: &ListInfo = dyn_list.get_represented_list_info().unwrap();

        let dyn_tuple_struct: &dyn TupleStruct = &SomeTupleStruct(5.0);
        let _: &TupleStructInfo = dyn_tuple_struct
            .get_represented_tuple_struct_info()
            .unwrap();

        let dyn_enum: &dyn Enum = &SomeEnum::Foo;
        let _: &EnumInfo = dyn_enum.get_represented_enum_info().unwrap();
    }

    #[test]
    fn should_permit_higher_ranked_lifetimes() {
        #[derive(Reflect)]
        #[reflect(from_reflect = false)]
        struct TestStruct {
            #[reflect(ignore)]
            _hrl: for<'a> fn(&'a str) -> &'a str,
        }

        impl Default for TestStruct {
            fn default() -> Self {
                TestStruct {
                    _hrl: |input| input,
                }
            }
        }

        fn get_type_registration<T: GetTypeRegistration>() {}
        get_type_registration::<TestStruct>();
    }

    #[test]
    fn into_reflect() {
        trait TestTrait: Reflect {}

        #[derive(Reflect)]
        struct TestStruct;

        impl TestTrait for TestStruct {}

        let trait_object: Box<dyn TestTrait> = Box::new(TestStruct);

        // Should compile:
        let _ = trait_object.into_reflect();
    }

    #[test]
    fn as_reflect() {
        trait TestTrait: Reflect {}

        #[derive(Reflect)]
        struct TestStruct;

        impl TestTrait for TestStruct {}

        let trait_object: Box<dyn TestTrait> = Box::new(TestStruct);

        // Should compile:
        let _ = trait_object.as_reflect();
    }

    #[test]
    fn should_reflect_debug() {
        #[derive(Reflect)]
        struct Test {
            value: usize,
            list: Vec<String>,
            array: [f32; 3],
            a_struct: SomeStruct,
            a_tuple_struct: SomeTupleStruct,
            enum_unit: SomeEnum,
            enum_tuple: SomeEnum,
            enum_struct: SomeEnum,
            custom: CustomDebug,
            #[reflect(ignore)]
            #[expect(dead_code, reason = "This value is intended to not be reflected.")]
            ignored: isize,
        }

        #[derive(Reflect)]
        struct SomeStruct {
            foo: String,
        }

        #[derive(Reflect)]
        enum SomeEnum {
            A,
            B(usize),
            C { value: i32 },
        }

        #[derive(Reflect)]
        struct SomeTupleStruct(String);

        #[derive(Reflect)]
        #[reflect(Debug)]
        struct CustomDebug;
        impl Debug for CustomDebug {
            fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
                f.write_str("Cool debug!")
            }
        }

        let test = Test {
            value: 123,
            list: vec![String::from("A"), String::from("B"), String::from("C")],
            array: [1.0, 2.0, 3.0],
            a_struct: SomeStruct {
                foo: String::from("A Struct!"),
            },
            a_tuple_struct: SomeTupleStruct(String::from("A Tuple Struct!")),
            enum_unit: SomeEnum::A,
            enum_tuple: SomeEnum::B(123),
            enum_struct: SomeEnum::C { value: 321 },
            custom: CustomDebug,
            ignored: 321,
        };

        let reflected: &dyn Reflect = &test;
        let expected = r#"
bevy_reflect::tests::Test {
    value: 123,
    list: [
        "A",
        "B",
        "C",
    ],
    array: [
        1.0,
        2.0,
        3.0,
    ],
    a_struct: bevy_reflect::tests::SomeStruct {
        foo: "A Struct!",
    },
    a_tuple_struct: bevy_reflect::tests::SomeTupleStruct(
        "A Tuple Struct!",
    ),
    enum_unit: A,
    enum_tuple: B(
        123,
    ),
    enum_struct: C {
        value: 321,
    },
    custom: Cool debug!,
}"#;

        assert_eq!(expected, format!("\n{reflected:#?}"));
    }

    #[test]
    fn multiple_reflect_lists() {
        #[derive(Hash, PartialEq, Reflect)]
        #[reflect(Debug, Hash)]
        #[reflect(PartialEq)]
        struct Foo(i32);

        impl Debug for Foo {
            fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
                write!(f, "Foo")
            }
        }

        let foo = Foo(123);
        let foo: &dyn PartialReflect = &foo;

        assert!(foo.reflect_hash().is_some());
        assert_eq!(Some(true), foo.reflect_partial_eq(foo));
        assert_eq!("Foo".to_string(), format!("{foo:?}"));
    }

    #[test]
    fn should_allow_empty_enums() {
        #[derive(Reflect)]
        enum Empty {}

        assert_impl_all!(Empty: Reflect);
    }

    #[test]
    fn recursive_typed_storage_does_not_hang() {
        #[derive(Reflect)]
        struct Recurse<T>(T);

        let _ = <Recurse<Recurse<()>> as Typed>::type_info();
        let _ = <Recurse<Recurse<()>> as TypePath>::type_path();
    }

    #[test]
    fn recursive_registration_does_not_hang() {
        #[derive(Reflect)]
        struct Recurse<T>(T);

        let mut registry = TypeRegistry::empty();

        registry.register::<Recurse<Recurse<()>>>();
    }

    #[test]
    fn dynamic_types_debug_format() {
        #[derive(Debug, Reflect)]
        struct TestTupleStruct(u32);

        #[derive(Debug, Reflect)]
        enum TestEnum {
            A(u32),
            B,
        }

        #[derive(Debug, Reflect)]
        // test DynamicStruct
        struct TestStruct {
            // test DynamicTuple
            tuple: (u32, u32),
            // test DynamicTupleStruct
            tuple_struct: TestTupleStruct,
            // test DynamicList
            list: Vec<u32>,
            // test DynamicArray
            array: [u32; 3],
            // test DynamicEnum
            e: TestEnum,
            // test reflected value
            value: u32,
        }
        let mut test_struct: DynamicStruct = TestStruct {
            tuple: (0, 1),
            list: vec![2, 3, 4],
            array: [5, 6, 7],
            tuple_struct: TestTupleStruct(8),
            e: TestEnum::A(11),
            value: 12,
        }
        .to_dynamic_struct();

        // test unknown DynamicStruct
        let mut test_unknown_struct = DynamicStruct::default();
        test_unknown_struct.insert_boxed("a", Box::new(13));
        test_struct.insert_boxed("unknown_struct", Box::new(test_unknown_struct));
        // test unknown DynamicTupleStruct
        let mut test_unknown_tuple_struct = DynamicTupleStruct::default();
        test_unknown_tuple_struct.insert_boxed(Box::new(14));
        test_struct.insert_boxed("unknown_tuplestruct", Box::new(test_unknown_tuple_struct));
        assert_eq!(
            format!("{test_struct:?}"),
            "DynamicStruct(bevy_reflect::tests::TestStruct { \
                tuple: DynamicTuple((0, 1)), \
                tuple_struct: DynamicTupleStruct(bevy_reflect::tests::TestTupleStruct(8)), \
                list: DynamicList([2, 3, 4]), \
                array: DynamicArray([5, 6, 7]), \
                e: DynamicEnum(A(11)), \
                value: 12, \
                unknown_struct: DynamicStruct(_ { a: 13 }), \
                unknown_tuplestruct: DynamicTupleStruct(_(14)) \
            })"
        );
    }

    #[test]
    fn assert_impl_reflect_macro_on_all() {
        struct Struct {
            foo: (),
        }
        struct TupleStruct(());
        enum Enum {
            Foo { foo: () },
            Bar(()),
        }

        impl_reflect!(
            #[type_path = "my_crate::foo"]
            struct Struct {
                foo: (),
            }
        );

        impl_reflect!(
            #[type_path = "my_crate::foo"]
            struct TupleStruct(());
        );

        impl_reflect!(
            #[type_path = "my_crate::foo"]
            enum Enum {
                Foo { foo: () },
                Bar(()),
            }
        );

        assert_impl_all!(Struct: Reflect);
        assert_impl_all!(TupleStruct: Reflect);
        assert_impl_all!(Enum: Reflect);
    }

    #[cfg(feature = "glam")]
    mod glam {
        use super::*;
        use ::glam::vec3;

        #[test]
        fn vec3_field_access() {
            let mut v = vec3(1.0, 2.0, 3.0);

            assert_eq!(*v.get_field::<f32>("x").unwrap(), 1.0);

            *v.get_field_mut::<f32>("y").unwrap() = 6.0;

            assert_eq!(v.y, 6.0);
        }

        #[test]
        fn vec3_apply_dynamic() {
            let mut v = vec3(3.0, 3.0, 3.0);

            let mut d = DynamicStruct::default();
            d.insert_boxed("x", Box::new(4.0f32));
            d.insert_boxed("y", Box::new(2.0f32));
            d.insert_boxed("z", Box::new(1.0f32));

            v.apply(&d);

            assert_eq!(v, vec3(4.0, 2.0, 1.0));
        }
    }
}
