//! Contains code related to container attributes for reflected types.
//!
//! A container attribute is an attribute which applies to an entire struct or enum
//! as opposed to a particular field or variant. An example of such an attribute is
//! the derive helper attribute for `Reflect`, which looks like:
//! `#[reflect(PartialEq, Default, ...)]`.

use crate::derive_data::ReflectTraitToImpl;
use bevy_macro_utils::{
    fq_std::{FQAny, FQClone, FQOption, FQResult},
    terminated_parser,
};
use proc_macro2::{Ident, Span};
use quote::quote_spanned;
use syn::{
    ext::IdentExt, parse::ParseStream, spanned::Spanned, token, Expr, LitBool, MetaList,
    MetaNameValue, Path, Token,
};

mod kw {
    syn::custom_keyword!(from_reflect);
    syn::custom_keyword!(Debug);
    syn::custom_keyword!(PartialEq);
    syn::custom_keyword!(Hash);
    syn::custom_keyword!(Clone);
    syn::custom_keyword!(opaque);
}

// The "special" trait idents that are used internally for reflection.
// Received via attributes like `#[reflect(PartialEq, Hash, ...)]`
const DEBUG_ATTR: &str = "Debug";
const PARTIAL_EQ_ATTR: &str = "PartialEq";
const HASH_ATTR: &str = "Hash";

// The traits listed below are not considered "special" (i.e. they use the `ReflectMyTrait` syntax)
// but useful to know exist nonetheless
pub(crate) const REFLECT_DEFAULT: &str = "ReflectDefault";

// Attributes for `FromReflect` implementation
const FROM_REFLECT_ATTR: &str = "from_reflect";

// The error message to show when a trait/type is specified multiple times
const CONFLICTING_TYPE_DATA_MESSAGE: &str = "conflicting type data registration";

/// A marker for trait implementations registered via the `Reflect` derive macro.
#[derive(Clone, Default)]
pub(crate) enum TraitImpl {
    /// The trait is not registered as implemented.
    #[default]
    NotImplemented,

    /// The trait is registered as implemented.
    Implemented(Span),
}

/// A collection of attributes used for deriving `FromReflect`.
#[derive(Clone, Default)]
pub(crate) struct FromReflectAttrs {
    auto_derive: Option<LitBool>,
}

impl FromReflectAttrs {
    /// Returns true if `FromReflect` should be automatically derived as part of the `Reflect` derive.
    pub fn should_auto_derive(&self) -> bool {
        self.auto_derive.as_ref().is_none_or(LitBool::value)
    }
}

/// A collection of traits that have been registered for a reflected type.
///
/// This keeps track of a few traits that are utilized internally for reflection
/// (we'll call these traits _special traits_ within this context), but it
/// will also keep track of all registered traits. Traits are registered as part of the
/// `Reflect` derive macro using the helper attribute: `#[reflect(...)]`.
///
/// The list of special traits are as follows:
/// * `Debug`
/// * `Hash`
/// * `PartialEq`
///
/// When registering a trait, there are a few things to keep in mind:
/// * Traits must have a valid `Reflect{}` struct in scope. For example, `Default`
///   needs `bevy_reflect::prelude::ReflectDefault` in scope.
/// * Traits must be single path identifiers. This means you _must_ use `Default`
///   instead of `std::default::Default` (otherwise it will try to register `Reflectstd`!)
///
/// # Example
///
/// Registering the `Default` implementation:
///
/// ```ignore (bevy_reflect is not accessible from this crate)
/// // Import ReflectDefault so it's accessible by the derive macro
/// use bevy_reflect::prelude::ReflectDefault;
///
/// #[derive(Reflect, Default)]
/// #[reflect(Default)]
/// struct Foo;
/// ```
///
/// Registering the `Hash` implementation:
///
/// ```ignore (bevy_reflect is not accessible from this crate)
/// // `Hash` is a "special trait" and does not need (nor have) a ReflectHash struct
///
/// #[derive(Reflect, Hash)]
/// #[reflect(Hash)]
/// struct Foo;
/// ```
#[derive(Default, Clone)]
pub(crate) struct ContainerAttributes {
    clone: TraitImpl,
    debug: TraitImpl,
    hash: TraitImpl,
    partial_eq: TraitImpl,
    from_reflect_attrs: FromReflectAttrs,
    is_opaque: bool,
    idents: Vec<Ident>,
}

impl ContainerAttributes {
    /// Parse a comma-separated list of container attributes.
    ///
    /// # Example
    /// - `Hash, Debug, MyTrait`
    pub fn parse_terminated(
        &mut self,
        input: ParseStream,
        trait_: ReflectTraitToImpl,
    ) -> syn::Result<()> {
        terminated_parser(Token![,], |stream| {
            self.parse_container_attribute(stream, trait_)
        })(input)?;

        Ok(())
    }

    /// Parse the contents of a `#[reflect(...)]` attribute into a [`ContainerAttributes`] instance.
    ///
    /// # Example
    /// - `#[reflect(Hash, Debug, MyTrait)]`
    pub fn parse_meta_list(
        &mut self,
        meta: &MetaList,
        trait_: ReflectTraitToImpl,
    ) -> syn::Result<()> {
        meta.parse_args_with(|stream: ParseStream| self.parse_terminated(stream, trait_))
    }

    /// Parse a single container attribute.
    fn parse_container_attribute(
        &mut self,
        input: ParseStream,
        trait_: ReflectTraitToImpl,
    ) -> syn::Result<()> {
        let lookahead = input.lookahead1();
        if lookahead.peek(kw::from_reflect) {
            self.parse_from_reflect(input, trait_)
        } else if lookahead.peek(kw::opaque) {
            self.parse_opaque(input)
        } else if lookahead.peek(kw::Clone) {
            self.parse_clone(input)
        } else if lookahead.peek(kw::Debug) {
            self.parse_debug(input)
        } else if lookahead.peek(kw::Hash) {
            self.parse_hash(input)
        } else if lookahead.peek(kw::PartialEq) {
            self.parse_partial_eq(input)
        } else if lookahead.peek(Ident::peek_any) {
            self.parse_ident(input)
        } else {
            Err(lookahead.error())
        }
    }

    /// Parse an ident (for registration).
    ///
    /// Examples:
    /// - `#[reflect(MyTrait)]` (registers `ReflectMyTrait`)
    fn parse_ident(&mut self, input: ParseStream) -> syn::Result<()> {
        let ident = input.parse::<Ident>()?;

        if input.peek(token::Paren) {
            return Err(syn::Error::new(ident.span(), format!(
                "only [{DEBUG_ATTR:?}, {PARTIAL_EQ_ATTR:?}, {HASH_ATTR:?}] may specify custom functions",
            )));
        }

        let ident_name = ident.to_string();

        // Create the reflect ident
        let mut reflect_ident = crate::ident::get_reflect_ident(&ident_name);
        // We set the span to the old ident so any compile errors point to that ident instead
        reflect_ident.set_span(ident.span());

        add_unique_ident(&mut self.idents, reflect_ident)?;

        Ok(())
    }

    /// Parse `clone` attribute.
    ///
    /// Examples:
    /// - `#[reflect(Clone)]`
    fn parse_clone(&mut self, input: ParseStream) -> syn::Result<()> {
        let ident = input.parse::<kw::Clone>()?;
        self.clone = TraitImpl::Implemented(ident.span);
        Ok(())
    }

    /// Parse special `Debug` registration.
    ///
    /// Examples:
    /// - `#[reflect(Debug)]`
    fn parse_debug(&mut self, input: ParseStream) -> syn::Result<()> {
        let ident = input.parse::<kw::Debug>()?;
        self.debug = TraitImpl::Implemented(ident.span);
        Ok(())
    }

    /// Parse special `PartialEq` registration.
    ///
    /// Examples:
    /// - `#[reflect(PartialEq)]`
    fn parse_partial_eq(&mut self, input: ParseStream) -> syn::Result<()> {
        let ident = input.parse::<kw::PartialEq>()?;
        self.partial_eq = TraitImpl::Implemented(ident.span);
        Ok(())
    }

    /// Parse special `Hash` registration.
    ///
    /// Examples:
    /// - `#[reflect(Hash)]`
    fn parse_hash(&mut self, input: ParseStream) -> syn::Result<()> {
        let ident = input.parse::<kw::Hash>()?;
        self.hash = TraitImpl::Implemented(ident.span);
        Ok(())
    }

    /// Parse `opaque` attribute.
    ///
    /// Examples:
    /// - `#[reflect(opaque)]`
    fn parse_opaque(&mut self, input: ParseStream) -> syn::Result<()> {
        input.parse::<kw::opaque>()?;
        self.is_opaque = true;
        Ok(())
    }

    /// Parse `from_reflect` attribute.
    ///
    /// Examples:
    /// - `#[reflect(from_reflect = false)]`
    fn parse_from_reflect(
        &mut self,
        input: ParseStream,
        trait_: ReflectTraitToImpl,
    ) -> syn::Result<()> {
        let pair = input.parse::<MetaNameValue>()?;
        let extracted_bool = extract_bool(&pair.value, |lit| {
            // Override `lit` if this is a `FromReflect` derive.
            // This typically means a user is opting out of the default implementation
            // from the `Reflect` derive and using the `FromReflect` derive directly instead.
            if trait_ == ReflectTraitToImpl::FromReflect {
                LitBool::new(true, Span::call_site())
            } else {
                lit.clone()
            }
        })?;

        if let Some(existing) = &self.from_reflect_attrs.auto_derive {
            if existing.value() != extracted_bool.value() {
                return Err(syn::Error::new(
                    extracted_bool.span(),
                    format!("`{FROM_REFLECT_ATTR}` already set to {}", existing.value()),
                ));
            }
        } else {
            self.from_reflect_attrs.auto_derive = Some(extracted_bool);
        }

        Ok(())
    }

    /// Returns true if the given reflected trait name (i.e. `ReflectDefault` for `Default`)
    /// is registered for this type.
    pub fn contains(&self, name: &str) -> bool {
        self.idents.iter().any(|ident| ident == name)
    }

    /// The list of reflected traits by their reflected ident (i.e. `ReflectDefault` for `Default`).
    pub fn idents(&self) -> &[Ident] {
        &self.idents
    }

    /// The `FromReflect` configuration found within `#[reflect(...)]` attributes on this type.
    #[expect(
        clippy::wrong_self_convention,
        reason = "Method returns `FromReflectAttrs`, does not actually convert data."
    )]
    pub fn from_reflect_attrs(&self) -> &FromReflectAttrs {
        &self.from_reflect_attrs
    }

    /// Returns the implementation of `PartialReflect::reflect_hash` as a `TokenStream`.
    ///
    /// If `Hash` was not registered, returns `None`.
    pub fn get_hash_impl(&self, bevy_reflect_path: &Path) -> Option<proc_macro2::TokenStream> {
        match &self.hash {
            &TraitImpl::Implemented(span) => Some(quote_spanned! {span=>
                fn reflect_hash(&self) -> #FQOption<u64> {
                    use ::core::hash::{Hash, Hasher};
                    let mut hasher = #bevy_reflect_path::utility::reflect_hasher();
                    Hash::hash(&#FQAny::type_id(self), &mut hasher);
                    Hash::hash(self, &mut hasher);
                    #FQOption::Some(Hasher::finish(&hasher))
                }
            }),
            TraitImpl::NotImplemented => None,
        }
    }

    /// Returns the implementation of `PartialReflect::reflect_partial_eq` as a `TokenStream`.
    ///
    /// If `PartialEq` was not registered, returns `None`.
    pub fn get_partial_eq_impl(
        &self,
        bevy_reflect_path: &Path,
    ) -> Option<proc_macro2::TokenStream> {
        match &self.partial_eq {
            &TraitImpl::Implemented(span) => Some(quote_spanned! {span=>
                fn reflect_partial_eq(&self, value: &dyn #bevy_reflect_path::PartialReflect) -> #FQOption<bool> {
                    let value = <dyn #bevy_reflect_path::PartialReflect>::try_downcast_ref::<Self>(value);
                    if let #FQOption::Some(value) = value {
                        #FQOption::Some(::core::cmp::PartialEq::eq(self, value))
                    } else {
                        #FQOption::Some(false)
                    }
                }
            }),
            TraitImpl::NotImplemented => None,
        }
    }

    /// Returns the implementation of `PartialReflect::debug` as a `TokenStream`.
    ///
    /// If `Debug` was not registered, returns `None`.
    pub fn get_debug_impl(&self) -> Option<proc_macro2::TokenStream> {
        match &self.debug {
            &TraitImpl::Implemented(span) => Some(quote_spanned! {span=>
                fn debug(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    ::core::fmt::Debug::fmt(self, f)
                }
            }),
            TraitImpl::NotImplemented => None,
        }
    }

    pub fn get_clone_impl(&self, bevy_reflect_path: &Path) -> Option<proc_macro2::TokenStream> {
        match &self.clone {
            &TraitImpl::Implemented(span) => Some(quote_spanned! {span=>
                #[inline]
                fn reflect_clone(&self) -> #FQResult<#bevy_reflect_path::__macro_exports::alloc_utils::Box<dyn #bevy_reflect_path::Reflect>, #bevy_reflect_path::ReflectCloneError> {
                    #FQResult::Ok(#bevy_reflect_path::__macro_exports::alloc_utils::Box::new(#FQClone::clone(self)))
                }
            }),
            TraitImpl::NotImplemented => None,
        }
    }

    /// Returns true if the `opaque` attribute was found on this type.
    pub fn is_opaque(&self) -> bool {
        self.is_opaque
    }
}

/// Adds an identifier to a vector of identifiers if it is not already present.
///
/// Returns an error if the identifier already exists in the list.
fn add_unique_ident(idents: &mut Vec<Ident>, ident: Ident) -> Result<(), syn::Error> {
    let ident_name = ident.to_string();
    if idents.iter().any(|i| i == ident_name.as_str()) {
        return Err(syn::Error::new(ident.span(), CONFLICTING_TYPE_DATA_MESSAGE));
    }

    idents.push(ident);
    Ok(())
}

/// Extract a boolean value from an expression.
///
/// The mapper exists so that the caller can conditionally choose to use the given
/// value or supply their own.
fn extract_bool(
    value: &Expr,
    mut mapper: impl FnMut(&LitBool) -> LitBool,
) -> Result<LitBool, syn::Error> {
    match value {
        Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Bool(lit),
            ..
        }) => Ok(mapper(lit)),
        _ => Err(syn::Error::new(value.span(), "Expected a boolean value")),
    }
}
