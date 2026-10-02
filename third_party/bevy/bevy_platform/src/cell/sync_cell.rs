#![expect(unsafe_code, reason = "SyncCell requires unsafe code.")]

//! A reimplementation of the currently unstable [`std::sync::Exclusive`]
//!
//! [`std::sync::Exclusive`]: https://doc.rust-lang.org/nightly/std/sync/struct.Exclusive.html

/// See [`Exclusive`](https://github.com/rust-lang/rust/issues/98407) for stdlib's upcoming implementation,
/// which should replace this one entirely.
///
/// Provides a wrapper that allows making any type unconditionally [`Sync`] by only providing mutable access.
#[repr(transparent)]
pub struct SyncCell<T: ?Sized> {
    inner: T,
}

impl<T: Sized> SyncCell<T> {
    /// Construct a new instance of a `SyncCell` from the given value.
    pub fn new(inner: T) -> Self {
        Self { inner }
    }
}

impl<T: ?Sized> SyncCell<T> {
    /// Get a reference to this `SyncCell`'s inner value.
    pub fn get(&mut self) -> &mut T {
        &mut self.inner
    }
}

// SAFETY: `Sync` only allows multithreaded access via immutable reference.
// As `SyncCell` requires an exclusive reference to access the wrapped value for `!Sync` types,
// marking this type as `Sync` does not actually allow unsynchronized access to the inner value.
unsafe impl<T: ?Sized> Sync for SyncCell<T> {}
