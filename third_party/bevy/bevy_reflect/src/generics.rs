use crate::type_info::impl_type_methods;
use crate::{Type, TypePath};
use alloc::{borrow::Cow, boxed::Box};
use core::ops::Deref;
use derive_more::derive::From;

/// The generic parameters of a type.
///
/// This is automatically generated via the [`Reflect` derive macro]
/// and stored on the [`TypeInfo`] returned by [`Typed::type_info`]
/// for types that have generics.
///
/// It supports both type parameters and const parameters
/// so long as they implement [`TypePath`].
///
/// If the type has no generics, this will be empty.
///
/// If the type is marked with `#[reflect(type_path = false)]`,
/// the generics will be empty even if the type has generics.
///
/// [`Reflect` derive macro]: bevy_reflect_derive::Reflect
/// [`TypeInfo`]: crate::type_info::TypeInfo
/// [`Typed::type_info`]: crate::Typed::type_info
#[derive(Clone, Default, Debug)]
pub struct Generics(Box<[GenericInfo]>);

impl Generics {
    /// Creates an empty set of generics.
    pub fn new() -> Self {
        Self(Box::new([]))
    }
}

impl<T: Into<GenericInfo>> FromIterator<T> for Generics {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self(iter.into_iter().map(Into::into).collect())
    }
}

impl Deref for Generics {
    type Target = [GenericInfo];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// An enum representing a generic parameter.
#[derive(Clone, Debug, From)]
pub enum GenericInfo {
    /// A type parameter.
    ///
    /// An example would be `T` in `struct Foo<T, U>`.
    Type(TypeParamInfo),
    /// A const parameter.
    ///
    /// An example would be `N` in `struct Foo<const N: usize>`.
    Const(ConstParamInfo),
}

impl GenericInfo {
    /// The name of the generic parameter.
    pub fn name(&self) -> &Cow<'static, str> {
        match self {
            Self::Type(info) => info.name(),
            Self::Const(info) => info.name(),
        }
    }

    impl_type_methods!(self => {
        match self {
            Self::Type(info) => info.ty(),
            Self::Const(info) => info.ty(),
        }
    });
}

/// Type information for a generic type parameter.
///
/// An example of a type parameter would be `T` in `struct Foo<T>`.
#[derive(Clone, Debug)]
pub struct TypeParamInfo {
    name: Cow<'static, str>,
    ty: Type,
}

impl TypeParamInfo {
    /// Creates a new type parameter with the given name.
    pub fn new<T: TypePath + ?Sized>(name: impl Into<Cow<'static, str>>) -> Self {
        Self {
            name: name.into(),
            ty: Type::of::<T>(),
        }
    }

    /// The name of the type parameter.
    pub fn name(&self) -> &Cow<'static, str> {
        &self.name
    }

    impl_type_methods!(ty);
}

/// Type information for a const generic parameter.
///
/// An example of a const parameter would be `N` in `struct Foo<const N: usize>`.
#[derive(Clone, Debug)]
pub struct ConstParamInfo {
    name: Cow<'static, str>,
    ty: Type,
}

impl ConstParamInfo {
    /// Creates a new const parameter with the given name.
    pub fn new<T: TypePath + ?Sized>(name: impl Into<Cow<'static, str>>) -> Self {
        Self {
            name: name.into(),
            ty: Type::of::<T>(),
        }
    }

    /// The name of the const parameter.
    pub fn name(&self) -> &Cow<'static, str> {
        &self.name
    }

    impl_type_methods!(ty);
}

macro_rules! impl_generic_info_methods {
    // Implements both getter and setter methods for the given field.
    ($field:ident) => {
        $crate::generics::impl_generic_info_methods!(self => &self.$field);

        /// Sets the generic parameters for this type.
        pub fn with_generics(mut self, generics: crate::generics::Generics) -> Self {
            self.$field = generics;
            self
        }
    };
    // Implements only a getter method for the given expression.
    ($self:ident => $expr:expr) => {
        /// Gets the generic parameters for this type.
        pub fn generics(&$self) -> &crate::generics::Generics {
            $expr
        }
    };
}

pub(crate) use impl_generic_info_methods;

#[cfg(test)]
mod tests {

    use crate::{Reflect, Typed};
    use alloc::string::String;
    use core::fmt::Debug;

    #[test]
    fn should_maintain_order() {
        #[derive(Reflect)]
        struct Test<T, U: Debug, const N: usize>([(T, U); N]);

        let generics = <Test<f32, String, 10> as Typed>::type_info()
            .as_tuple_struct()
            .unwrap()
            .generics();

        assert_eq!(generics.len(), 3);

        let mut iter = generics.iter();

        let t = iter.next().unwrap();
        assert_eq!(t.name(), "T");
        assert!(t.ty().is::<f32>());

        let u = iter.next().unwrap();
        assert_eq!(u.name(), "U");
        assert!(u.ty().is::<String>());

        let n = iter.next().unwrap();
        assert_eq!(n.name(), "N");
        assert!(n.ty().is::<usize>());

        assert!(iter.next().is_none());
    }
}
