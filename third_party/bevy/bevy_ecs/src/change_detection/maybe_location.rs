use core::{marker::PhantomData, ops::DerefMut, panic::Location};

/// A value that contains a `T` if the `track_location` feature is enabled,
/// and is a ZST if it is not.
///
/// The overall API is similar to [`Option`], but whether the value is `Some` or `None` is set at compile
/// time and is the same for all values.
///
/// If the `track_location` feature is disabled, then all functions on this type that return
/// an `MaybeLocation` will have an empty body and should be removed by the optimizer.
///
/// This allows code to be written that will be checked by the compiler even when the feature is disabled,
/// but that will be entirely removed during compilation.
#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MaybeLocation<T: ?Sized = &'static Location<'static>> {
    marker: PhantomData<T>,
}

impl<T: core::fmt::Display> core::fmt::Display for MaybeLocation<T> {
    fn fmt(&self, _f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Ok(())
    }
}

impl<T> MaybeLocation<T> {
    /// Constructs a new `MaybeLocation` that wraps the given value.
    ///
    /// This may only accept `Copy` types,
    /// since it needs to drop the value if the `track_location` feature is disabled,
    /// and non-`Copy` types cannot be dropped in `const` context.
    /// Use [`new_with`][Self::new_with] if you need to construct a non-`Copy` value.
    ///
    /// # See also
    /// - [`new_with`][Self::new_with] to initialize using a closure.
    /// - [`new_with_flattened`][Self::new_with_flattened] to initialize using a closure that returns an `Option<MaybeLocation<T>>`.
    #[inline]
    pub const fn new(_value: T) -> Self
    where
        T: Copy,
    {
        Self {
            marker: PhantomData,
        }
    }

    /// Constructs a new `MaybeLocation` that wraps the result of the given closure.
    ///
    /// # See also
    /// - [`new`][Self::new] to initialize using a value.
    /// - [`new_with_flattened`][Self::new_with_flattened] to initialize using a closure that returns an `Option<MaybeLocation<T>>`.
    #[inline]
    pub fn new_with(_f: impl FnOnce() -> T) -> Self {
        Self {
            marker: PhantomData,
        }
    }

    /// Maps an `MaybeLocation<T> `to `MaybeLocation<U>` by applying a function to a contained value.
    #[inline]
    pub fn map<U>(self, _f: impl FnOnce(T) -> U) -> MaybeLocation<U> {
        MaybeLocation {
            marker: PhantomData,
        }
    }

    /// Converts a pair of `MaybeLocation` values to an `MaybeLocation` of a tuple.
    #[inline]
    pub fn zip<U>(self, _other: MaybeLocation<U>) -> MaybeLocation<(T, U)> {
        MaybeLocation {
            marker: PhantomData,
        }
    }

    /// Returns the contained value or a default.
    /// If the `track_location` feature is enabled, this always returns the contained value.
    /// If it is disabled, this always returns `T::Default()`.
    #[inline]
    pub fn unwrap_or_default(self) -> T
    where
        T: Default,
    {
        self.into_option().unwrap_or_default()
    }

    /// Converts an `MaybeLocation` to an [`Option`] to allow run-time branching.
    /// If the `track_location` feature is enabled, this always returns `Some`.
    /// If it is disabled, this always returns `None`.
    #[inline]
    pub fn into_option(self) -> Option<T> {
        None
    }
}

impl<T> MaybeLocation<Option<T>> {
    /// Constructs a new `MaybeLocation` that wraps the result of the given closure.
    /// If the closure returns `Some`, it unwraps the inner value.
    ///
    /// # See also
    /// - [`new`][Self::new] to initialize using a value.
    /// - [`new_with`][Self::new_with] to initialize using a closure.
    #[inline]
    pub fn new_with_flattened(_f: impl FnOnce() -> Option<MaybeLocation<T>>) -> Self {
        Self {
            marker: PhantomData,
        }
    }
}

impl<T> MaybeLocation<&T> {
    /// Maps an `MaybeLocation<&T>` to an `MaybeLocation<T>` by copying the contents.
    #[inline]
    pub const fn copied(&self) -> MaybeLocation<T>
    where
        T: Copy,
    {
        MaybeLocation {
            marker: PhantomData,
        }
    }
}

impl<T> MaybeLocation<&mut T> {
    /// Maps an `MaybeLocation<&mut T>` to an `MaybeLocation<T>` by copying the contents.
    #[inline]
    pub const fn copied(&self) -> MaybeLocation<T>
    where
        T: Copy,
    {
        MaybeLocation {
            marker: PhantomData,
        }
    }

    /// Assigns the contents of an `MaybeLocation<T>` to an `MaybeLocation<&mut T>`.
    #[inline]
    pub fn assign(&mut self, _value: MaybeLocation<T>) {}
}

impl<T: ?Sized> MaybeLocation<T> {
    /// Converts from `&MaybeLocation<T>` to `MaybeLocation<&T>`.
    #[inline]
    pub const fn as_ref(&self) -> MaybeLocation<&T> {
        MaybeLocation {
            marker: PhantomData,
        }
    }

    /// Converts from `&mut MaybeLocation<T>` to `MaybeLocation<&mut T>`.
    #[inline]
    pub const fn as_mut(&mut self) -> MaybeLocation<&mut T> {
        MaybeLocation {
            marker: PhantomData,
        }
    }

    /// Converts from `&mut MaybeLocation<T>` to `MaybeLocation<&mut T::Target>`.
    #[inline]
    pub fn as_deref_mut(&mut self) -> MaybeLocation<&mut T::Target>
    where
        T: DerefMut,
    {
        MaybeLocation {
            marker: PhantomData,
        }
    }
}

impl MaybeLocation {
    /// Returns the source location of the caller of this function. If that function's caller is
    /// annotated then its call location will be returned, and so on up the stack to the first call
    /// within a non-tracked function body.
    #[inline]
    #[track_caller]
    pub const fn caller() -> Self {
        // Note that this cannot use `new_with`, since `FnOnce` invocations cannot be annotated with `#[track_caller]`.
        MaybeLocation {
            marker: PhantomData,
        }
    }
}
