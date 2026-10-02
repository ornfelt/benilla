//! Contains code related to field attributes for reflected types.
//!
//! A field attribute is an attribute which applies to particular field or variant
//! as opposed to an entire struct or enum. An example of such an attribute is
//! the derive helper attribute for `Reflect`, which looks like: `#[reflect(ignore)]`.

use crate::REFLECT_ATTRIBUTE_NAME;
use bevy_macro_utils::terminated_parser;
use syn::{parse::ParseStream, Attribute, Meta, Token};

mod kw {
    syn::custom_keyword!(ignore);
}

pub(crate) const IGNORE_ALL_ATTR: &str = "ignore";

/// Stores data about if the field should be visible via the Reflect interface
///
/// The `is_reflected` predicate is provided as `self.is_active()`
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReflectIgnoreBehavior {
    /// Don't ignore, appear to all systems
    #[default]
    None,
    /// Ignore when reflecting
    IgnoreAlways,
}

impl ReflectIgnoreBehavior {
    /// Returns `true` if the ignoring behavior implies member is included in the reflection API, and false otherwise.
    pub fn is_active(self) -> bool {
        match self {
            ReflectIgnoreBehavior::None => true,
            ReflectIgnoreBehavior::IgnoreAlways => false,
        }
    }

    /// The exact logical opposite of `self.is_active()` returns true iff this member is not part of the reflection API whatsoever
    pub fn is_ignored(self) -> bool {
        !self.is_active()
    }
}

/// A container for attributes defined on a reflected type's field.
#[derive(Default, Clone)]
pub(crate) struct FieldAttributes {
    /// Determines how this field should be ignored if at all.
    pub ignore: ReflectIgnoreBehavior,
}

impl FieldAttributes {
    /// Parse all field attributes marked "reflect" (such as `#[reflect(ignore)]`).
    pub fn parse_attributes(attrs: &[Attribute]) -> syn::Result<Self> {
        let mut args = FieldAttributes::default();

        attrs
            .iter()
            .filter_map(|attr| {
                if !attr.path().is_ident(REFLECT_ATTRIBUTE_NAME) {
                    // Not a reflect attribute -> skip
                    return None;
                }

                let Meta::List(meta) = &attr.meta else {
                    return Some(syn::Error::new_spanned(attr, "expected meta list"));
                };

                // Parse all attributes inside the list, collecting any errors
                meta.parse_args_with(terminated_parser(Token![,], |stream| {
                    args.parse_field_attribute(stream)
                }))
                .err()
            })
            .reduce(|mut acc, err| {
                acc.combine(err);
                acc
            })
            .map_or(Ok(args), Err)
    }

    /// Parses a single field attribute.
    fn parse_field_attribute(&mut self, input: ParseStream) -> syn::Result<()> {
        let lookahead = input.lookahead1();
        if lookahead.peek(kw::ignore) {
            self.parse_ignore(input)
        } else {
            Err(lookahead.error())
        }
    }

    /// Parse `ignore` attribute.
    ///
    /// Examples:
    /// - `#[reflect(ignore)]`
    fn parse_ignore(&mut self, input: ParseStream) -> syn::Result<()> {
        if self.ignore != ReflectIgnoreBehavior::None {
            return Err(input.error(format!("only one of {:?} is allowed", [IGNORE_ALL_ATTR])));
        }

        input.parse::<kw::ignore>()?;
        self.ignore = ReflectIgnoreBehavior::IgnoreAlways;
        Ok(())
    }
}
