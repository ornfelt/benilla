use crate::{NamedField, UnnamedField};
use alloc::boxed::Box;
use bevy_platform::collections::HashMap;
use thiserror::Error;

/// Describes the form of an enum variant.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum VariantType {
    /// Struct enums take the form:
    ///
    /// ```
    /// enum MyEnum {
    ///   A {
    ///     foo: usize
    ///   }
    /// }
    /// ```
    Struct,
    /// Tuple enums take the form:
    ///
    /// ```
    /// enum MyEnum {
    ///   A(usize)
    /// }
    /// ```
    Tuple,
    /// Unit enums take the form:
    ///
    /// ```
    /// enum MyEnum {
    ///   A
    /// }
    /// ```
    Unit,
}

/// A [`VariantInfo`]-specific error.
#[derive(Debug, Error)]
pub enum VariantInfoError {
    /// Caused when a variant was expected to be of a certain [type], but was not.
    ///
    /// [type]: VariantType
    #[error("variant type mismatch: expected {expected:?}, received {received:?}")]
    TypeMismatch {
        /// Expected variant type.
        expected: VariantType,
        /// Received variant type.
        received: VariantType,
    },
}

/// A container for compile-time enum variant info.
#[derive(Clone, Debug)]
pub enum VariantInfo {
    /// Struct enums take the form:
    ///
    /// ```
    /// enum MyEnum {
    ///   A {
    ///     foo: usize
    ///   }
    /// }
    /// ```
    Struct(StructVariantInfo),
    /// Tuple enums take the form:
    ///
    /// ```
    /// enum MyEnum {
    ///   A(usize)
    /// }
    /// ```
    Tuple(TupleVariantInfo),
    /// Unit enums take the form:
    ///
    /// ```
    /// enum MyEnum {
    ///   A
    /// }
    /// ```
    Unit(UnitVariantInfo),
}

impl VariantInfo {
    /// The name of the enum variant.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Struct(info) => info.name(),
            Self::Tuple(info) => info.name(),
            Self::Unit(info) => info.name(),
        }
    }

    /// Returns the [type] of this variant.
    ///
    /// [type]: VariantType
    pub fn variant_type(&self) -> VariantType {
        match self {
            Self::Struct(_) => VariantType::Struct,
            Self::Tuple(_) => VariantType::Tuple,
            Self::Unit(_) => VariantType::Unit,
        }
    }
}

macro_rules! impl_cast_method {
    ($name:ident : $kind:ident => $info:ident) => {
        #[doc = concat!("Attempts a cast to [`", stringify!($info), "`].")]
        #[doc = concat!("\n\nReturns an error if `self` is not [`VariantInfo::", stringify!($kind), "`].")]
        pub fn $name(&self) -> Result<&$info, VariantInfoError> {
            match self {
                Self::$kind(info) => Ok(info),
                _ => Err(VariantInfoError::TypeMismatch {
                    expected: VariantType::$kind,
                    received: self.variant_type(),
                }),
            }
        }
    };
}

/// Conversion convenience methods for [`VariantInfo`].
impl VariantInfo {
    impl_cast_method!(as_tuple_variant: Tuple => TupleVariantInfo);
}

/// Type info for struct variants.
#[derive(Clone, Debug)]
pub struct StructVariantInfo {
    name: &'static str,
    fields: Box<[NamedField]>,
    field_indices: HashMap<&'static str, usize>,
}

impl StructVariantInfo {
    /// Create a new [`StructVariantInfo`].
    pub fn new(name: &'static str, fields: &[NamedField]) -> Self {
        let field_indices = Self::collect_field_indices(fields);
        Self {
            name,
            fields: fields.to_vec().into_boxed_slice(),
            field_indices,
        }
    }

    /// The name of this variant.
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// Get the field with the given name.
    pub fn field(&self, name: &str) -> Option<&NamedField> {
        self.field_indices
            .get(name)
            .map(|index| &self.fields[*index])
    }

    /// Get the field at the given index.
    pub fn field_at(&self, index: usize) -> Option<&NamedField> {
        self.fields.get(index)
    }

    /// The total number of fields in this variant.
    pub fn field_len(&self) -> usize {
        self.fields.len()
    }

    fn collect_field_indices(fields: &[NamedField]) -> HashMap<&'static str, usize> {
        fields
            .iter()
            .enumerate()
            .map(|(index, field)| (field.name(), index))
            .collect()
    }
}

/// Type info for tuple variants.
#[derive(Clone, Debug)]
pub struct TupleVariantInfo {
    name: &'static str,
    fields: Box<[UnnamedField]>,
}

impl TupleVariantInfo {
    /// Create a new [`TupleVariantInfo`].
    pub fn new(name: &'static str, fields: &[UnnamedField]) -> Self {
        Self {
            name,
            fields: fields.to_vec().into_boxed_slice(),
        }
    }

    /// The name of this variant.
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// Get the field at the given index.
    pub fn field_at(&self, index: usize) -> Option<&UnnamedField> {
        self.fields.get(index)
    }
}

/// Type info for unit variants.
#[derive(Clone, Debug)]
pub struct UnitVariantInfo {
    name: &'static str,
}

impl UnitVariantInfo {
    /// Create a new [`UnitVariantInfo`].
    pub fn new(name: &'static str) -> Self {
        Self { name }
    }

    /// The name of this variant.
    pub fn name(&self) -> &'static str {
        self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Reflect, Typed};

    #[test]
    fn should_return_error_on_invalid_cast() {
        #[derive(Reflect)]
        enum Foo {
            Bar,
        }

        let info = Foo::type_info().as_enum().unwrap();
        let variant = info.variant_at(0).unwrap();
        assert!(matches!(
            variant.as_tuple_variant(),
            Err(VariantInfoError::TypeMismatch {
                expected: VariantType::Tuple,
                received: VariantType::Unit
            })
        ));
    }
}
