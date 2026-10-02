use crate::{type_info::impl_type_methods, MaybeTyped, PartialReflect, Type, TypePath};
use alloc::borrow::Cow;
use core::fmt::{Display, Formatter};

/// The named field of a reflected struct.
#[derive(Clone, Debug)]
pub struct NamedField {
    name: &'static str,
    ty: Type,
}

impl NamedField {
    /// Create a new [`NamedField`].
    pub fn new<T: PartialReflect + MaybeTyped + TypePath>(name: &'static str) -> Self {
        Self {
            name,
            ty: Type::of::<T>(),
        }
    }

    /// The name of the field.
    pub fn name(&self) -> &'static str {
        self.name
    }

    impl_type_methods!(ty);
}

/// The unnamed field of a reflected tuple or tuple struct.
#[derive(Clone, Debug)]
pub struct UnnamedField {
    ty: Type,
}

impl UnnamedField {
    /// Create a new [`UnnamedField`].
    pub fn new<T: PartialReflect + MaybeTyped + TypePath>(_index: usize) -> Self {
        Self {
            ty: Type::of::<T>(),
        }
    }

    impl_type_methods!(ty);
}

/// A representation of a field's accessor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FieldId {
    /// Access a field by name.
    Named(Cow<'static, str>),
    /// Access a field by index.
    Unnamed(usize),
}

impl Display for FieldId {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Named(name) => Display::fmt(name, f),
            Self::Unnamed(index) => Display::fmt(index, f),
        }
    }
}
