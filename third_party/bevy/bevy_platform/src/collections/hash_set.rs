//! Provides [`HashSet`] based on [hashbrown]'s implementation.
//! Unlike [`hashbrown::HashSet`], [`HashSet`] defaults to [`FixedHasher`]
//! instead of [`RandomState`](crate::hash::RandomState).
//! This provides determinism by default with an acceptable compromise to denial
//! of service resistance in the context of a game engine.

use core::{
    fmt::Debug,
    hash::{BuildHasher, Hash},
    ops::{BitAnd, BitOr, BitXor, Deref, DerefMut, Sub},
};

use hashbrown::{hash_set as hb, Equivalent};

use crate::hash::FixedHasher;

// Re-exports to match `std::collections::hash_set`
pub use hb::{Difference, Drain, Intersection, IntoIter, Iter, SymmetricDifference, Union};

// Additional items from `hashbrown`
pub use hb::OccupiedEntry;

/// Shortcut for [`Entry`](hb::Entry) with [`FixedHasher`] as the default hashing provider.
pub type Entry<'a, T, S = FixedHasher> = hb::Entry<'a, T, S>;

/// New-type for [`HashSet`](hb::HashSet) with [`FixedHasher`] as the default hashing provider.
/// Can be trivially converted to and from a [hashbrown] [`HashSet`](hb::HashSet) using [`From`].
///
/// A new-type is used instead of a type alias due to critical methods like [`new`](hb::HashSet::new)
/// being incompatible with Bevy's choice of default hasher.
///
/// Unlike [`hashbrown::HashSet`], [`HashSet`] defaults to [`FixedHasher`]
/// instead of [`RandomState`](crate::hash::RandomState).
/// This provides determinism by default with an acceptable compromise to denial
/// of service resistance in the context of a game engine.
#[repr(transparent)]
pub struct HashSet<T, S = FixedHasher>(hb::HashSet<T, S>);

impl<T, S> Clone for HashSet<T, S>
where
    hb::HashSet<T, S>: Clone,
{
    #[inline]
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }

    #[inline]
    fn clone_from(&mut self, source: &Self) {
        self.0.clone_from(&source.0);
    }
}

impl<T, S> Debug for HashSet<T, S>
where
    hb::HashSet<T, S>: Debug,
{
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        <hb::HashSet<T, S> as Debug>::fmt(&self.0, f)
    }
}

impl<T, S> Default for HashSet<T, S>
where
    hb::HashSet<T, S>: Default,
{
    #[inline]
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<T, S> PartialEq for HashSet<T, S>
where
    hb::HashSet<T, S>: PartialEq,
{
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl<T, S> Eq for HashSet<T, S> where hb::HashSet<T, S>: Eq {}

impl<T, S, X> FromIterator<X> for HashSet<T, S>
where
    hb::HashSet<T, S>: FromIterator<X>,
{
    #[inline]
    fn from_iter<U: IntoIterator<Item = X>>(iter: U) -> Self {
        Self(FromIterator::from_iter(iter))
    }
}

impl<T, S> IntoIterator for HashSet<T, S>
where
    hb::HashSet<T, S>: IntoIterator,
{
    type Item = <hb::HashSet<T, S> as IntoIterator>::Item;

    type IntoIter = <hb::HashSet<T, S> as IntoIterator>::IntoIter;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a, T, S> IntoIterator for &'a HashSet<T, S>
where
    &'a hb::HashSet<T, S>: IntoIterator,
{
    type Item = <&'a hb::HashSet<T, S> as IntoIterator>::Item;

    type IntoIter = <&'a hb::HashSet<T, S> as IntoIterator>::IntoIter;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        (&self.0).into_iter()
    }
}

impl<'a, T, S> IntoIterator for &'a mut HashSet<T, S>
where
    &'a mut hb::HashSet<T, S>: IntoIterator,
{
    type Item = <&'a mut hb::HashSet<T, S> as IntoIterator>::Item;

    type IntoIter = <&'a mut hb::HashSet<T, S> as IntoIterator>::IntoIter;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        (&mut self.0).into_iter()
    }
}

impl<T, S, X> Extend<X> for HashSet<T, S>
where
    hb::HashSet<T, S>: Extend<X>,
{
    #[inline]
    fn extend<U: IntoIterator<Item = X>>(&mut self, iter: U) {
        self.0.extend(iter);
    }
}

impl<T, S> From<hb::HashSet<T, S>> for HashSet<T, S> {
    #[inline]
    fn from(value: hb::HashSet<T, S>) -> Self {
        Self(value)
    }
}

impl<T, S> From<HashSet<T, S>> for hb::HashSet<T, S> {
    #[inline]
    fn from(value: HashSet<T, S>) -> Self {
        value.0
    }
}

impl<T, S> Deref for HashSet<T, S> {
    type Target = hb::HashSet<T, S>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T, S> DerefMut for HashSet<T, S> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[cfg(feature = "serialize")]
impl<T, S> serde::Serialize for HashSet<T, S>
where
    hb::HashSet<T, S>: serde::Serialize,
{
    #[inline]
    fn serialize<U>(&self, serializer: U) -> Result<U::Ok, U::Error>
    where
        U: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

#[cfg(feature = "serialize")]
impl<'de, T, S> serde::Deserialize<'de> for HashSet<T, S>
where
    hb::HashSet<T, S>: serde::Deserialize<'de>,
{
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Self(serde::Deserialize::deserialize(deserializer)?))
    }
}

impl<T, S> HashSet<T, S> {
    /// An iterator visiting all elements in arbitrary order.
    /// The iterator element type is `&'a T`.
    ///
    /// Refer to [`iter`](hb::HashSet::iter) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashSet;
    /// #
    /// let mut map = HashSet::new();
    ///
    /// map.insert("foo");
    /// map.insert("bar");
    /// map.insert("baz");
    ///
    /// for value in map.iter() {
    ///     // "foo", "bar", "baz"
    ///     // Note that the above order is not guaranteed
    /// }
    /// #
    /// # assert_eq!(map.iter().count(), 3);
    /// ```
    #[inline]
    pub fn iter(&self) -> Iter<'_, T> {
        self.0.iter()
    }

    /// Returns the number of elements in the set.
    ///
    /// Refer to [`len`](hb::HashSet::len) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashSet;
    /// let mut map = HashSet::new();
    ///
    /// assert_eq!(map.len(), 0);
    ///
    /// map.insert("foo");
    ///
    /// assert_eq!(map.len(), 1);
    /// ```
    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Clears the set, returning all elements in an iterator.
    ///
    /// Refer to [`drain`](hb::HashSet::drain) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashSet;
    /// #
    /// let mut map = HashSet::new();
    ///
    /// map.insert("foo");
    /// map.insert("bar");
    /// map.insert("baz");
    ///
    /// for value in map.drain() {
    ///     // "foo", "bar", "baz"
    ///     // Note that the above order is not guaranteed
    /// }
    ///
    /// assert!(map.is_empty());
    /// ```
    #[inline]
    pub fn drain(&mut self) -> Drain<'_, T> {
        self.0.drain()
    }

    /// Retains only the elements specified by the predicate.
    ///
    /// Refer to [`retain`](hb::HashSet::retain) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashSet;
    /// #
    /// let mut map = HashSet::new();
    ///
    /// map.insert("foo");
    /// map.insert("bar");
    /// map.insert("baz");
    ///
    /// map.retain(|value| *value == "baz");
    ///
    /// assert_eq!(map.len(), 1);
    /// ```
    #[inline]
    pub fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&T) -> bool,
    {
        self.0.retain(f);
    }

    /// Creates a new empty hash set which will use the given hasher to hash
    /// keys.
    ///
    /// Refer to [`with_hasher`](hb::HashSet::with_hasher) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashSet;
    /// # use bevy_platform::hash::FixedHasher as SomeHasher;
    /// // Creates a HashSet with the provided hasher.
    /// let map = HashSet::with_hasher(SomeHasher);
    /// #
    /// # let mut map = map;
    /// # map.insert("foo");
    /// # assert_eq!(map.get("foo"), Some("foo").as_ref());
    /// ```
    #[inline]
    pub const fn with_hasher(hasher: S) -> Self {
        Self(hb::HashSet::with_hasher(hasher))
    }

    /// Creates an empty [`HashSet`] with the specified capacity, using
    /// `hasher` to hash the keys.
    ///
    /// Refer to [`with_capacity_and_hasher`](hb::HashSet::with_capacity_and_hasher) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashSet;
    /// # use bevy_platform::hash::FixedHasher as SomeHasher;
    /// // Creates a HashSet with capacity for 5 entries and the provided hasher.
    /// let map = HashSet::with_capacity_and_hasher(5, SomeHasher);
    /// #
    /// # let mut map = map;
    /// # map.insert("foo");
    /// # assert_eq!(map.get("foo"), Some("foo").as_ref());
    /// ```
    #[inline]
    pub fn with_capacity_and_hasher(capacity: usize, hasher: S) -> Self {
        Self(hb::HashSet::with_capacity_and_hasher(capacity, hasher))
    }
}

impl<T, S> HashSet<T, S>
where
    T: Eq + Hash,
    S: BuildHasher,
{
    /// Returns `true` if the set contains a value.
    ///
    /// Refer to [`contains`](hb::HashSet::contains) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashSet;
    /// let mut map = HashSet::new();
    ///
    /// map.insert("foo");
    ///
    /// assert!(map.contains("foo"));
    /// ```
    #[inline]
    pub fn contains<Q>(&self, value: &Q) -> bool
    where
        Q: Hash + Equivalent<T> + ?Sized,
    {
        self.0.contains(value)
    }

    /// Returns a reference to the value in the set, if any, that is equal to the given value.
    ///
    /// Refer to [`get`](hb::HashSet::get) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashSet;
    /// let mut map = HashSet::new();
    ///
    /// map.insert("foo");
    ///
    /// assert_eq!(map.get("foo"), Some(&"foo"));
    /// ```
    #[inline]
    pub fn get<Q>(&self, value: &Q) -> Option<&T>
    where
        Q: Hash + Equivalent<T> + ?Sized,
    {
        self.0.get(value)
    }

    /// Adds a value to the set.
    ///
    /// Refer to [`insert`](hb::HashSet::insert) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashSet;
    /// let mut map = HashSet::new();
    ///
    /// map.insert("foo");
    ///
    /// assert!(map.contains("foo"));
    /// ```
    #[inline]
    pub fn insert(&mut self, value: T) -> bool {
        self.0.insert(value)
    }

    /// Removes a value from the set. Returns whether the value was
    /// present in the set.
    ///
    /// Refer to [`remove`](hb::HashSet::remove) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashSet;
    /// let mut map = HashSet::new();
    ///
    /// map.insert("foo");
    ///
    /// assert!(map.remove("foo"));
    ///
    /// assert!(map.is_empty());
    /// ```
    #[inline]
    pub fn remove<Q>(&mut self, value: &Q) -> bool
    where
        Q: Hash + Equivalent<T> + ?Sized,
    {
        self.0.remove(value)
    }
}

impl<T, S> BitOr<&HashSet<T, S>> for &HashSet<T, S>
where
    for<'a> &'a hb::HashSet<T, S>: BitOr<&'a hb::HashSet<T, S>, Output = hb::HashSet<T, S>>,
{
    type Output = HashSet<T, S>;

    /// Returns the union of `self` and `rhs` as a new `HashSet<T, S>`.
    #[inline]
    fn bitor(self, rhs: &HashSet<T, S>) -> HashSet<T, S> {
        HashSet(self.0.bitor(&rhs.0))
    }
}

impl<T, S> BitAnd<&HashSet<T, S>> for &HashSet<T, S>
where
    for<'a> &'a hb::HashSet<T, S>: BitAnd<&'a hb::HashSet<T, S>, Output = hb::HashSet<T, S>>,
{
    type Output = HashSet<T, S>;

    /// Returns the intersection of `self` and `rhs` as a new `HashSet<T, S>`.
    #[inline]
    fn bitand(self, rhs: &HashSet<T, S>) -> HashSet<T, S> {
        HashSet(self.0.bitand(&rhs.0))
    }
}

impl<T, S> BitXor<&HashSet<T, S>> for &HashSet<T, S>
where
    for<'a> &'a hb::HashSet<T, S>: BitXor<&'a hb::HashSet<T, S>, Output = hb::HashSet<T, S>>,
{
    type Output = HashSet<T, S>;

    /// Returns the symmetric difference of `self` and `rhs` as a new `HashSet<T, S>`.
    #[inline]
    fn bitxor(self, rhs: &HashSet<T, S>) -> HashSet<T, S> {
        HashSet(self.0.bitxor(&rhs.0))
    }
}

impl<T, S> Sub<&HashSet<T, S>> for &HashSet<T, S>
where
    for<'a> &'a hb::HashSet<T, S>: Sub<&'a hb::HashSet<T, S>, Output = hb::HashSet<T, S>>,
{
    type Output = HashSet<T, S>;

    /// Returns the difference of `self` and `rhs` as a new `HashSet<T, S>`.
    #[inline]
    fn sub(self, rhs: &HashSet<T, S>) -> HashSet<T, S> {
        HashSet(self.0.sub(&rhs.0))
    }
}
