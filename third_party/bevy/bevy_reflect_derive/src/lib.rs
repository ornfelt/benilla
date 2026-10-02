#![cfg_attr(docsrs, feature(doc_cfg))]

//! This crate contains macros used by Bevy's `Reflect` API.
//!
//! The main export of this crate is the derive macro for [`Reflect`]. This allows
//! types to easily implement `Reflect` along with other `bevy_reflect` traits,
//! such as `Struct`, `GetTypeRegistration`, and more— all with a single derive!
//!
//! Some other noteworthy exports include the derive macros for [`FromReflect`] and
//! [`TypePath`].
//!
//! [`Reflect`]: crate::derive_reflect
//! [`FromReflect`]: crate::derive_from_reflect
//! [`TypePath`]: crate::derive_type_path

extern crate proc_macro;

mod container_attributes;
mod derive_data;
mod enum_utility;
mod field_attributes;
mod from_reflect;
mod ident;
mod impls;
mod meta;
mod reflect_opaque;
mod registration;
mod string_expr;
mod struct_utility;
mod type_path;
mod where_clause_options;

use crate::derive_data::{ReflectDerive, ReflectMeta, ReflectStruct};
use container_attributes::ContainerAttributes;
use derive_data::{ReflectImplSource, ReflectProvenance, ReflectTraitToImpl, ReflectTypePath};
use proc_macro::TokenStream;
use quote::quote;
use reflect_opaque::ReflectOpaqueDef;
use syn::{parse_macro_input, DeriveInput};
use type_path::NamedTypePathDef;

pub(crate) static REFLECT_ATTRIBUTE_NAME: &str = "reflect";
pub(crate) static TYPE_PATH_ATTRIBUTE_NAME: &str = "type_path";
pub(crate) static TYPE_NAME_ATTRIBUTE_NAME: &str = "type_name";

/// Used both for [`impl_reflect`] and [`derive_reflect`].
///
/// [`impl_reflect`]: macro@impl_reflect
/// [`derive_reflect`]: derive_reflect()
fn match_reflect_impls(ast: DeriveInput, source: ReflectImplSource) -> TokenStream {
    let derive_data = match ReflectDerive::from_input(
        &ast,
        ReflectProvenance {
            source,
            trait_: ReflectTraitToImpl::Reflect,
        },
    ) {
        Ok(data) => data,
        Err(err) => return err.into_compile_error().into(),
    };

    let (reflect_impls, from_reflect_impl) = match derive_data {
        ReflectDerive::Struct(struct_data) | ReflectDerive::UnitStruct(struct_data) => (
            impls::impl_struct(&struct_data),
            if struct_data.meta().from_reflect().should_auto_derive() {
                Some(from_reflect::impl_struct(&struct_data))
            } else {
                None
            },
        ),
        ReflectDerive::TupleStruct(struct_data) => (
            impls::impl_tuple_struct(&struct_data),
            if struct_data.meta().from_reflect().should_auto_derive() {
                Some(from_reflect::impl_tuple_struct(&struct_data))
            } else {
                None
            },
        ),
        ReflectDerive::Enum(enum_data) => (
            impls::impl_enum(&enum_data),
            if enum_data.meta().from_reflect().should_auto_derive() {
                Some(from_reflect::impl_enum(&enum_data))
            } else {
                None
            },
        ),
        ReflectDerive::Opaque(meta) => (
            impls::impl_opaque(&meta),
            if meta.from_reflect().should_auto_derive() {
                Some(from_reflect::impl_opaque(&meta))
            } else {
                None
            },
        ),
    };

    TokenStream::from(quote! {
        const _: () = {
            #reflect_impls

            #from_reflect_impl
        };
    })
}

/// The main derive macro used by `bevy_reflect` for deriving its `Reflect` trait.
///
/// This macro can be used on all structs and enums (unions are not supported).
/// It will automatically generate implementations for `Reflect`, `Typed`, `GetTypeRegistration`, and `FromReflect`.
/// And, depending on the item's structure, will either implement `Struct`, `TupleStruct`, or `Enum`.
///
/// See the [`FromReflect`] derive macro for more information on how to customize the `FromReflect` implementation.
///
/// # Container Attributes
///
/// This macro comes with some helper attributes that can be added to the container item
/// in order to provide additional functionality or alter the generated implementations.
///
/// In addition to those listed, this macro can also use the attributes for [`TypePath`] derives.
///
/// ## `#[reflect(Ident)]`
///
/// The `#[reflect(Ident)]` attribute is used to add type data registrations to the `GetTypeRegistration`
/// implementation corresponding to the given identifier, prepended by `Reflect`.
///
/// For example, `#[reflect(Foo, Bar)]` would add two registrations:
/// one for `ReflectFoo` and another for `ReflectBar`.
/// This assumes these types are indeed in-scope wherever this macro is called.
///
/// ### Default Registrations
///
/// The following types are automatically registered when deriving `Reflect`:
///
/// * `ReflectFromReflect` (unless opting out of `FromReflect`)
/// * `ReflectFromPtr`
///
/// ### Special Identifiers
///
/// There are a few "special" identifiers that work a bit differently:
///
/// * `#[reflect(Clone)]` will force the implementation of `Reflect::reflect_clone` to rely on
///   the type's [`Clone`] implementation.
/// * `#[reflect(Debug)]` will force the implementation of `Reflect::reflect_debug` to rely on
///   the type's [`Debug`] implementation.
/// * `#[reflect(PartialEq)]` will force the implementation of `Reflect::reflect_partial_eq` to rely on
///   the type's [`PartialEq`] implementation.
/// * `#[reflect(Hash)]` will force the implementation of `Reflect::reflect_hash` to rely on
///   the type's [`Hash`] implementation.
/// * `#[reflect(Default)]` will register the `ReflectDefault` type data as normal.
///   However, it will also affect how certain other operations are performed in order
///   to improve performance and/or robustness.
///   An example of where this is used is in the [`FromReflect`] derive macro,
///   where adding this attribute will cause the `FromReflect` implementation to create
///   a base value using its [`Default`] implementation avoiding issues with ignored fields
///   (for structs and tuple structs only).
///
/// ## `#[reflect(opaque)]`
///
/// The `#[reflect(opaque)]` attribute denotes that the item should implement `Reflect` as an opaque type,
/// hiding its structure and fields from the reflection API.
/// This means that it will forgo implementing `Struct`, `TupleStruct`, or `Enum`.
///
/// Furthermore, it requires that the type implements [`Clone`].
///
/// ## `#[reflect(from_reflect = false)]`
///
/// This attribute will opt-out of the default `FromReflect` implementation.
///
/// This is useful for when a type can't or shouldn't implement `FromReflect`,
/// or if a manual implementation is desired.
///
/// Note that in the latter case, `ReflectFromReflect` will no longer be automatically registered.
///
/// # Field Attributes
///
/// Along with the container attributes, this macro comes with some attributes that may be applied
/// to the contained fields themselves.
///
/// ## `#[reflect(ignore)]`
///
/// This attribute simply marks a field to be ignored by the reflection API.
///
/// This allows fields to completely opt-out of reflection,
/// which may be useful for maintaining invariants, keeping certain data private,
/// or allowing the use of types that do not implement `Reflect` within the container.
#[proc_macro_derive(Reflect, attributes(reflect, type_path, type_name))]
pub fn derive_reflect(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    match_reflect_impls(ast, ReflectImplSource::DeriveLocalType)
}

/// Derives the `FromReflect` trait.
///
/// # Field Attributes
///
/// ## `#[reflect(ignore)]`
///
/// The `#[reflect(ignore)]` attribute is shared with the [`#[derive(Reflect)]`](Reflect) macro and has much of the same
/// functionality in that it denotes that a field will be ignored by the reflection API.
///
/// The only major difference is that using it with this derive requires that the field implements [`Default`].
/// Without this requirement, there would be no way for `FromReflect` to automatically construct missing fields
/// that have been ignored.
#[proc_macro_derive(FromReflect, attributes(reflect))]
pub fn derive_from_reflect(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    let derive_data = match ReflectDerive::from_input(
        &ast,
        ReflectProvenance {
            source: ReflectImplSource::DeriveLocalType,
            trait_: ReflectTraitToImpl::FromReflect,
        },
    ) {
        Ok(data) => data,
        Err(err) => return err.into_compile_error().into(),
    };

    let from_reflect_impl = match derive_data {
        ReflectDerive::Struct(struct_data) | ReflectDerive::UnitStruct(struct_data) => {
            from_reflect::impl_struct(&struct_data)
        }
        ReflectDerive::TupleStruct(struct_data) => from_reflect::impl_tuple_struct(&struct_data),
        ReflectDerive::Enum(meta) => from_reflect::impl_enum(&meta),
        ReflectDerive::Opaque(meta) => from_reflect::impl_opaque(&meta),
    };

    TokenStream::from(quote! {
        const _: () = {
            #from_reflect_impl
        };
    })
}

/// Derives the `TypePath` trait, providing a stable alternative to [`std::any::type_name`].
///
/// # Container Attributes
///
/// ## `#[type_path = "my_crate::foo"]`
///
/// Optionally specifies a custom module path to use instead of [`module_path`].
///
/// This path does not include the final identifier.
///
/// ## `#[type_name = "RenamedType"]`
///
/// Optionally specifies a new terminating identifier for `TypePath`.
///
/// To use this attribute, `#[type_path = "..."]` must also be specified.
#[proc_macro_derive(TypePath, attributes(type_path, type_name))]
pub fn derive_type_path(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let derive_data = match ReflectDerive::from_input(
        &ast,
        ReflectProvenance {
            source: ReflectImplSource::DeriveLocalType,
            trait_: ReflectTraitToImpl::TypePath,
        },
    ) {
        Ok(data) => data,
        Err(err) => return err.into_compile_error().into(),
    };

    let type_path_impl = impls::impl_type_path(derive_data.meta());

    TokenStream::from(quote! {
        const _: () = {
            #type_path_impl
        };
    })
}

/// A macro used to generate reflection trait implementations for the given type.
///
/// This is functionally the same as [deriving `Reflect`] using the `#[reflect(opaque)]` container attribute.
///
/// The only reason for this macro's existence is so that `bevy_reflect` can easily implement the reflection traits
/// on primitives and other opaque types internally.
///
/// Since this macro also implements `TypePath`, the type path must be explicit.
/// See [`impl_type_path!`] for the exact syntax.
///
/// # Examples
///
/// Types can be passed with or without registering type data:
///
/// ```ignore (bevy_reflect is not accessible from this crate)
/// impl_reflect_opaque!(my_crate::Foo);
/// impl_reflect_opaque!(my_crate::Bar(Debug, Default));
/// ```
///
/// Generic types can also specify their parameters and bounds:
///
/// ```ignore (bevy_reflect is not accessible from this crate)
/// impl_reflect_opaque!(my_crate::Foo<T1, T2: Baz> where T1: Bar (Default));
/// ```
///
/// Custom type paths can be specified:
///
/// ```ignore (bevy_reflect is not accessible from this crate)
/// impl_reflect_opaque!((in not_my_crate as NotFoo) Foo(Debug, Default));
/// ```
///
/// [deriving `Reflect`]: Reflect
#[proc_macro]
pub fn impl_reflect_opaque(input: TokenStream) -> TokenStream {
    let def = parse_macro_input!(input with ReflectOpaqueDef::parse_reflect);

    let default_name = &def.type_path.segments.last().unwrap().ident;
    let type_path = if def.type_path.leading_colon.is_none() && def.custom_path.is_none() {
        ReflectTypePath::Primitive(default_name)
    } else {
        ReflectTypePath::External {
            path: &def.type_path,
            custom_path: def.custom_path.map(|path| path.into_path(default_name)),
            generics: &def.generics,
        }
    };

    let meta = ReflectMeta::new(type_path, def.traits.unwrap_or_default());

    let reflect_impls = impls::impl_opaque(&meta);
    let from_reflect_impl = from_reflect::impl_opaque(&meta);

    TokenStream::from(quote! {
        const _: () = {
            #reflect_impls
            #from_reflect_impl
        };
    })
}

/// A replacement for `#[derive(Reflect)]` to be used with foreign types which
/// the definitions of cannot be altered.
///
/// This macro is an alternative to [`impl_reflect_opaque!`],
/// which implements foreign types as Opaque types. Note that there is no `impl_from_reflect`,
/// as this macro will do the job of both. This macro implements them using one of the reflect
/// variant traits (`bevy_reflect::{Struct, TupleStruct, Enum}`, etc.),
/// which have greater functionality. The type being reflected must be in scope, as you cannot
/// qualify it in the macro as e.g. `bevy::prelude::Vec3`.
///
/// It is necessary to add a `#[type_path = "my_crate::foo"]` attribute to all types.
///
/// It may be necessary to add `#[reflect(Default)]` for some types, specifically non-constructible
/// foreign types. Without `Default` reflected for such types, you will usually get an arcane
/// error message and fail to compile. If the type does not implement `Default`, it may not
/// be possible to reflect without extending the macro.
///
///
/// # Example
/// Implementing `Reflect` for `bevy::prelude::Vec3` as a struct type:
/// ```ignore (bevy_reflect is not accessible from this crate)
/// use bevy::prelude::Vec3;
///
/// impl_reflect!(
///     #[reflect(PartialEq, Default)]
///     #[type_path = "bevy::prelude"]
///     struct Vec3 {
///         x: f32,
///         y: f32,
///         z: f32
///     }
/// );
/// ```
#[proc_macro]
pub fn impl_reflect(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    match_reflect_impls(ast, ReflectImplSource::ImplRemoteType)
}

/// A replacement for [deriving `TypePath`] for use on foreign types.
///
/// Since (unlike the derive) this macro may be invoked in a different module to where the type is defined,
/// it requires an 'absolute' path definition.
///
/// Specifically, a leading `::` denoting a global path must be specified
/// or a preceding `(in my_crate::foo)` to specify the custom path must be used.
///
/// # Examples
///
/// Implementing `TypePath` on a foreign type:
/// ```ignore (bevy_reflect is not accessible from this crate)
/// impl_type_path!(::foreign_crate::foo::bar::Baz);
/// ```
///
/// On a generic type (this can also accept trait bounds):
/// ```ignore (bevy_reflect is not accessible from this crate)
/// impl_type_path!(::foreign_crate::Foo<T>);
/// impl_type_path!(::foreign_crate::Goo<T: ?Sized>);
/// ```
///
/// On a primitive (note this will not compile for a non-primitive type):
/// ```ignore (bevy_reflect is not accessible from this crate)
/// impl_type_path!(bool);
/// ```
///
/// With a custom type path:
/// ```ignore (bevy_reflect is not accessible from this crate)
/// impl_type_path!((in other_crate::foo::bar) Baz);
/// ```
///
/// With a custom type path and a custom type name:
/// ```ignore (bevy_reflect is not accessible from this crate)
/// impl_type_path!((in other_crate::foo as Baz) Bar);
/// ```
///
/// [deriving `TypePath`]: TypePath
#[proc_macro]
pub fn impl_type_path(input: TokenStream) -> TokenStream {
    let def = parse_macro_input!(input as NamedTypePathDef);

    let type_path = match def {
        NamedTypePathDef::External {
            ref path,
            custom_path,
            ref generics,
        } => {
            let default_name = &path.segments.last().unwrap().ident;

            ReflectTypePath::External {
                path,
                custom_path: custom_path.map(|path| path.into_path(default_name)),
                generics,
            }
        }
        NamedTypePathDef::Primitive(ref ident) => ReflectTypePath::Primitive(ident),
    };

    let meta = ReflectMeta::new(type_path, ContainerAttributes::default());

    let type_path_impl = impls::impl_type_path(&meta);

    TokenStream::from(quote! {
        const _: () = {
            #type_path_impl
        };
    })
}
