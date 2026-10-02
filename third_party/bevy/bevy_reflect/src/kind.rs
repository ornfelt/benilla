use alloc::boxed::Box;
use thiserror::Error;

use crate::{Array, Enum, List, PartialReflect, Struct, Tuple, TupleStruct};

/// An enumeration of the "kinds" of a reflected type.
///
/// Each kind corresponds to a specific reflection trait,
/// such as [`Struct`] or [`List`],
/// which itself corresponds to the kind or structure of a type.
///
/// A [`ReflectKind`] is obtained via [`PartialReflect::reflect_kind`],
/// or via [`ReflectRef::kind`],[`ReflectMut::kind`] or [`ReflectOwned::kind`].
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ReflectKind {
    /// A [struct-like] type.
    ///
    /// [struct-like]: Struct
    Struct,
    /// A [tuple-struct-like] type.
    ///
    /// [tuple-struct-like]: TupleStruct
    TupleStruct,
    /// A [tuple-like] type.
    ///
    /// [tuple-like]: Tuple
    Tuple,
    /// A [list-like] type.
    ///
    /// [list-like]: List
    List,
    /// An [array-like] type.
    ///
    /// [array-like]: Array
    Array,
    /// An [enum-like] type.
    ///
    /// [enum-like]: Enum
    Enum,
    /// An opaque type.
    ///
    /// This most often represents a type where it is either impossible, difficult,
    /// or unuseful to reflect the type further.
    ///
    /// This includes types like `String` and `Instant`.
    ///
    /// Despite not technically being opaque types,
    /// primitives like `u32` `i32` are considered opaque for the purposes of reflection.
    ///
    /// Additionally, any type that [derives `Reflect`] with the `#[reflect(opaque)]` attribute
    /// will be considered an opaque type.
    ///
    /// [derives `Reflect`]: bevy_reflect_derive::Reflect
    Opaque,
}

impl core::fmt::Display for ReflectKind {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ReflectKind::Struct => f.pad("struct"),
            ReflectKind::TupleStruct => f.pad("tuple struct"),
            ReflectKind::Tuple => f.pad("tuple"),
            ReflectKind::List => f.pad("list"),
            ReflectKind::Array => f.pad("array"),
            ReflectKind::Enum => f.pad("enum"),
            ReflectKind::Opaque => f.pad("opaque"),
        }
    }
}

macro_rules! impl_reflect_kind_conversions {
    ($name:ident$(<$lifetime:lifetime>)?) => {
        impl $name$(<$lifetime>)? {
            /// Returns the "kind" of this reflected type without any information.
            pub fn kind(&self) -> ReflectKind {
                match self {
                    Self::Struct(_) => ReflectKind::Struct,
                    Self::TupleStruct(_) => ReflectKind::TupleStruct,
                    Self::Tuple(_) => ReflectKind::Tuple,
                    Self::List(_) => ReflectKind::List,
                    Self::Array(_) => ReflectKind::Array,
                    Self::Enum(_) => ReflectKind::Enum,
                    Self::Opaque(_) => ReflectKind::Opaque,
                }
            }
        }

        impl From<$name$(<$lifetime>)?> for ReflectKind {
            fn from(value: $name) -> Self {
                match value {
                    $name::Struct(_) => Self::Struct,
                    $name::TupleStruct(_) => Self::TupleStruct,
                    $name::Tuple(_) => Self::Tuple,
                    $name::List(_) => Self::List,
                    $name::Array(_) => Self::Array,
                    $name::Enum(_) => Self::Enum,
                    $name::Opaque(_) => Self::Opaque,
                }
            }
        }
    };
}

/// Caused when a type was expected to be of a certain [kind], but was not.
///
/// [kind]: ReflectKind
#[derive(Debug, Error)]
#[error("kind mismatch: expected {expected:?}, received {received:?}")]
pub struct ReflectKindMismatchError {
    /// Expected kind.
    pub expected: ReflectKind,
    /// Received kind.
    pub received: ReflectKind,
}

macro_rules! impl_cast_method {
    ($name:ident : Opaque => $retval:ty) => {
        #[doc = "Attempts a cast to a [`PartialReflect`] trait object."]
        #[doc = "\n\nReturns an error if `self` is not the [`Self::Opaque`] variant."]
        pub fn $name(self) -> Result<$retval, ReflectKindMismatchError> {
            match self {
                Self::Opaque(value) => Ok(value),
                _ => Err(ReflectKindMismatchError {
                    expected: ReflectKind::Opaque,
                    received: self.kind(),
                }),
            }
        }
    };
    ($name:ident : $kind:ident => $retval:ty) => {
        #[doc = concat!("Attempts a cast to a [`", stringify!($kind), "`] trait object.")]
        #[doc = concat!("\n\nReturns an error if `self` is not the [`Self::", stringify!($kind), "`] variant.")]
        pub fn $name(self) -> Result<$retval, ReflectKindMismatchError> {
            match self {
                Self::$kind(value) => Ok(value),
                _ => Err(ReflectKindMismatchError {
                    expected: ReflectKind::$kind,
                    received: self.kind(),
                }),
            }
        }
    };
}

/// An immutable enumeration of ["kinds"] of a reflected type.
///
/// Each variant contains a trait object with methods specific to a kind of
/// type.
///
/// A [`ReflectRef`] is obtained via [`PartialReflect::reflect_ref`].
///
/// ["kinds"]: ReflectKind
pub enum ReflectRef<'a> {
    /// An immutable reference to a [struct-like] type.
    ///
    /// [struct-like]: Struct
    Struct(&'a dyn Struct),
    /// An immutable reference to a [tuple-struct-like] type.
    ///
    /// [tuple-struct-like]: TupleStruct
    TupleStruct(&'a dyn TupleStruct),
    /// An immutable reference to a [tuple-like] type.
    ///
    /// [tuple-like]: Tuple
    Tuple(&'a dyn Tuple),
    /// An immutable reference to a [list-like] type.
    ///
    /// [list-like]: List
    List(&'a dyn List),
    /// An immutable reference to an [array-like] type.
    ///
    /// [array-like]: Array
    Array(&'a dyn Array),
    /// An immutable reference to an [enum-like] type.
    ///
    /// [enum-like]: Enum
    Enum(&'a dyn Enum),
    /// An immutable reference to an [opaque] type.
    ///
    /// [opaque]: ReflectKind::Opaque
    Opaque(&'a dyn PartialReflect),
}

impl_reflect_kind_conversions!(ReflectRef<'_>);

impl<'a> ReflectRef<'a> {
    impl_cast_method!(as_struct: Struct => &'a dyn Struct);
    impl_cast_method!(as_tuple_struct: TupleStruct => &'a dyn TupleStruct);
    impl_cast_method!(as_tuple: Tuple => &'a dyn Tuple);
    impl_cast_method!(as_list: List => &'a dyn List);
    impl_cast_method!(as_array: Array => &'a dyn Array);
    impl_cast_method!(as_enum: Enum => &'a dyn Enum);
}

/// A mutable enumeration of ["kinds"] of a reflected type.
///
/// Each variant contains a trait object with methods specific to a kind of
/// type.
///
/// A [`ReflectMut`] is obtained via [`PartialReflect::reflect_mut`].
///
/// ["kinds"]: ReflectKind
pub enum ReflectMut<'a> {
    /// A mutable reference to a [struct-like] type.
    ///
    /// [struct-like]: Struct
    Struct(&'a mut dyn Struct),
    /// A mutable reference to a [tuple-struct-like] type.
    ///
    /// [tuple-struct-like]: TupleStruct
    TupleStruct(&'a mut dyn TupleStruct),
    /// A mutable reference to a [tuple-like] type.
    ///
    /// [tuple-like]: Tuple
    Tuple(&'a mut dyn Tuple),
    /// A mutable reference to a [list-like] type.
    ///
    /// [list-like]: List
    List(&'a mut dyn List),
    /// A mutable reference to an [array-like] type.
    ///
    /// [array-like]: Array
    Array(&'a mut dyn Array),
    /// A mutable reference to an [enum-like] type.
    ///
    /// [enum-like]: Enum
    Enum(&'a mut dyn Enum),
    /// A mutable reference to an [opaque] type.
    ///
    /// [opaque]: ReflectKind::Opaque
    Opaque(&'a mut dyn PartialReflect),
}

impl_reflect_kind_conversions!(ReflectMut<'_>);

/// An owned enumeration of ["kinds"] of a reflected type.
///
/// Each variant contains a trait object with methods specific to a kind of
/// type.
///
/// A [`ReflectOwned`] is obtained via [`PartialReflect::reflect_owned`].
///
/// ["kinds"]: ReflectKind
pub enum ReflectOwned {
    /// An owned [struct-like] type.
    ///
    /// [struct-like]: Struct
    Struct(Box<dyn Struct>),
    /// An owned [tuple-struct-like] type.
    ///
    /// [tuple-struct-like]: TupleStruct
    TupleStruct(Box<dyn TupleStruct>),
    /// An owned [tuple-like] type.
    ///
    /// [tuple-like]: Tuple
    Tuple(Box<dyn Tuple>),
    /// An owned [list-like] type.
    ///
    /// [list-like]: List
    List(Box<dyn List>),
    /// An owned [array-like] type.
    ///
    /// [array-like]: Array
    Array(Box<dyn Array>),
    /// An owned [enum-like] type.
    ///
    /// [enum-like]: Enum
    Enum(Box<dyn Enum>),
    /// An owned [opaque] type.
    ///
    /// [opaque]: ReflectKind::Opaque
    Opaque(Box<dyn PartialReflect>),
}

impl_reflect_kind_conversions!(ReflectOwned);

#[cfg(test)]
mod tests {
    use alloc::vec;

    use super::*;

    #[test]
    fn should_cast_ref() {
        let value = vec![1, 2, 3];

        let result = value.reflect_ref().as_list();
        assert!(result.is_ok());

        let result = value.reflect_ref().as_array();
        assert!(matches!(
            result,
            Err(ReflectKindMismatchError {
                expected: ReflectKind::Array,
                received: ReflectKind::List
            })
        ));
    }
}
