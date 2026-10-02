//! Provides [`HashMap`] based on [hashbrown]'s implementation.
//! Unlike [`hashbrown::HashMap`], [`HashMap`] defaults to [`FixedHasher`]
//! instead of [`RandomState`].
//! This provides determinism by default with an acceptable compromise to denial
//! of service resistance in the context of a game engine.

use core::{
    fmt::Debug,
    hash::{BuildHasher, Hash},
    ops::{Deref, DerefMut, Index},
};

use hashbrown::{hash_map as hb, Equivalent};

use crate::hash::FixedHasher;

// Re-exports to match `std::collections::hash_map`
pub use {
    crate::hash::{DefaultHasher, RandomState},
    hb::{Drain, IntoIter, Iter, IterMut, OccupiedEntry},
};

// Additional items from `hashbrown`
pub use hb::RawEntryMut;

/// Shortcut for [`Entry`](hb::Entry) with [`FixedHasher`] as the default hashing provider.
pub type Entry<'a, K, V, S = FixedHasher> = hb::Entry<'a, K, V, S>;

/// New-type for [`HashMap`](hb::HashMap) with [`FixedHasher`] as the default hashing provider.
/// Can be trivially converted to and from a [hashbrown] [`HashMap`](hb::HashMap) using [`From`].
///
/// A new-type is used instead of a type alias due to critical methods like [`new`](hb::HashMap::new)
/// being incompatible with Bevy's choice of default hasher.
///
/// Unlike [`hashbrown::HashMap`], [`HashMap`] defaults to [`FixedHasher`]
/// instead of [`RandomState`].
/// This provides determinism by default with an acceptable compromise to denial
/// of service resistance in the context of a game engine.
#[repr(transparent)]
pub struct HashMap<K, V, S = FixedHasher>(hb::HashMap<K, V, S>);

impl<K, V, S> Clone for HashMap<K, V, S>
where
    hb::HashMap<K, V, S>: Clone,
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

impl<K, V, S> Debug for HashMap<K, V, S>
where
    hb::HashMap<K, V, S>: Debug,
{
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        <hb::HashMap<K, V, S> as Debug>::fmt(&self.0, f)
    }
}

impl<K, V, S> Default for HashMap<K, V, S>
where
    hb::HashMap<K, V, S>: Default,
{
    #[inline]
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<K, V, S> PartialEq for HashMap<K, V, S>
where
    hb::HashMap<K, V, S>: PartialEq,
{
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl<K, V, S> Eq for HashMap<K, V, S> where hb::HashMap<K, V, S>: Eq {}

impl<K, V, S, T> FromIterator<T> for HashMap<K, V, S>
where
    hb::HashMap<K, V, S>: FromIterator<T>,
{
    #[inline]
    fn from_iter<U: IntoIterator<Item = T>>(iter: U) -> Self {
        Self(FromIterator::from_iter(iter))
    }
}

impl<K, V, S, T> Index<T> for HashMap<K, V, S>
where
    hb::HashMap<K, V, S>: Index<T>,
{
    type Output = <hb::HashMap<K, V, S> as Index<T>>::Output;

    #[inline]
    fn index(&self, index: T) -> &Self::Output {
        self.0.index(index)
    }
}

impl<K, V, S> IntoIterator for HashMap<K, V, S>
where
    hb::HashMap<K, V, S>: IntoIterator,
{
    type Item = <hb::HashMap<K, V, S> as IntoIterator>::Item;

    type IntoIter = <hb::HashMap<K, V, S> as IntoIterator>::IntoIter;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a, K, V, S> IntoIterator for &'a HashMap<K, V, S>
where
    &'a hb::HashMap<K, V, S>: IntoIterator,
{
    type Item = <&'a hb::HashMap<K, V, S> as IntoIterator>::Item;

    type IntoIter = <&'a hb::HashMap<K, V, S> as IntoIterator>::IntoIter;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        (&self.0).into_iter()
    }
}

impl<'a, K, V, S> IntoIterator for &'a mut HashMap<K, V, S>
where
    &'a mut hb::HashMap<K, V, S>: IntoIterator,
{
    type Item = <&'a mut hb::HashMap<K, V, S> as IntoIterator>::Item;

    type IntoIter = <&'a mut hb::HashMap<K, V, S> as IntoIterator>::IntoIter;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        (&mut self.0).into_iter()
    }
}

impl<K, V, S, T> Extend<T> for HashMap<K, V, S>
where
    hb::HashMap<K, V, S>: Extend<T>,
{
    #[inline]
    fn extend<U: IntoIterator<Item = T>>(&mut self, iter: U) {
        self.0.extend(iter);
    }
}

impl<K, V, S> From<hb::HashMap<K, V, S>> for HashMap<K, V, S> {
    #[inline]
    fn from(value: hb::HashMap<K, V, S>) -> Self {
        Self(value)
    }
}

impl<K, V, S> From<HashMap<K, V, S>> for hb::HashMap<K, V, S> {
    #[inline]
    fn from(value: HashMap<K, V, S>) -> Self {
        value.0
    }
}

impl<K, V, S> Deref for HashMap<K, V, S> {
    type Target = hb::HashMap<K, V, S>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<K, V, S> DerefMut for HashMap<K, V, S> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[cfg(feature = "serialize")]
impl<K, V, S> serde::Serialize for HashMap<K, V, S>
where
    hb::HashMap<K, V, S>: serde::Serialize,
{
    #[inline]
    fn serialize<T>(&self, serializer: T) -> Result<T::Ok, T::Error>
    where
        T: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

#[cfg(feature = "serialize")]
impl<'de, K, V, S> serde::Deserialize<'de> for HashMap<K, V, S>
where
    hb::HashMap<K, V, S>: serde::Deserialize<'de>,
{
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Self(serde::Deserialize::deserialize(deserializer)?))
    }
}

impl<K, V> HashMap<K, V, FixedHasher> {
    /// Creates an empty [`HashMap`].
    ///
    /// Refer to [`new`](hb::HashMap::new) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashMap;
    /// #
    /// // Creates a HashMap with zero capacity.
    /// let map = HashMap::new();
    /// #
    /// # let mut map = map;
    /// # map.insert(0usize, "foo");
    /// # assert_eq!(map.get(&0), Some("foo").as_ref());
    /// ```
    #[inline]
    pub const fn new() -> Self {
        Self::with_hasher(FixedHasher)
    }
}

impl<K, V, S> HashMap<K, V, S> {
    /// Creates an empty [`HashMap`] which will use the given hash builder to hash
    /// keys.
    ///
    /// Refer to [`with_hasher`](hb::HashMap::with_hasher) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashMap;
    /// # use bevy_platform::hash::FixedHasher as SomeHasher;
    /// // Creates a HashMap with the provided hasher.
    /// let map = HashMap::with_hasher(SomeHasher);
    /// #
    /// # let mut map = map;
    /// # map.insert(0usize, "foo");
    /// # assert_eq!(map.get(&0), Some("foo").as_ref());
    /// ```
    #[inline]
    pub const fn with_hasher(hash_builder: S) -> Self {
        Self(hb::HashMap::with_hasher(hash_builder))
    }

    /// Creates an empty [`HashMap`] with the specified capacity, using `hash_builder`
    /// to hash the keys.
    ///
    /// Refer to [`with_capacity_and_hasher`](hb::HashMap::with_capacity_and_hasher) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashMap;
    /// # use bevy_platform::hash::FixedHasher as SomeHasher;
    /// // Creates a HashMap with capacity for 5 entries and the provided hasher.
    /// let map = HashMap::with_capacity_and_hasher(5, SomeHasher);
    /// #
    /// # let mut map = map;
    /// # map.insert(0usize, "foo");
    /// # assert_eq!(map.get(&0), Some("foo").as_ref());
    /// ```
    #[inline]
    pub fn with_capacity_and_hasher(capacity: usize, hash_builder: S) -> Self {
        Self(hb::HashMap::with_capacity_and_hasher(
            capacity,
            hash_builder,
        ))
    }

    /// An iterator visiting all key-value pairs in arbitrary order.
    /// The iterator element type is `(&'a K, &'a V)`.
    ///
    /// Refer to [`iter`](hb::HashMap::iter) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashMap;
    /// #
    /// let mut map = HashMap::new();
    ///
    /// map.insert("foo", 0);
    /// map.insert("bar", 1);
    /// map.insert("baz", 2);
    ///
    /// for (key, value) in map.iter() {
    ///     // ("foo", 0), ("bar", 1), ("baz", 2)
    ///     // Note that the above order is not guaranteed
    /// }
    /// #
    /// # assert_eq!(map.iter().count(), 3);
    /// ```
    #[inline]
    pub fn iter(&self) -> Iter<'_, K, V> {
        self.0.iter()
    }

    /// Returns the number of elements in the map.
    ///
    /// Refer to [`len`](hb::HashMap::len) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashMap;
    /// let mut map = HashMap::new();
    ///
    /// assert_eq!(map.len(), 0);
    ///
    /// map.insert("foo", 0);
    ///
    /// assert_eq!(map.len(), 1);
    /// ```
    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Clears the map, returning all key-value pairs as an iterator. Keeps the
    /// allocated memory for reuse.
    ///
    /// Refer to [`drain`](hb::HashMap::drain) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashMap;
    /// #
    /// let mut map = HashMap::new();
    ///
    /// map.insert("foo", 0);
    /// map.insert("bar", 1);
    /// map.insert("baz", 2);
    ///
    /// for (key, value) in map.drain() {
    ///     // ("foo", 0), ("bar", 1), ("baz", 2)
    ///     // Note that the above order is not guaranteed
    /// }
    ///
    /// assert!(map.is_empty());
    /// ```
    #[inline]
    pub fn drain(&mut self) -> Drain<'_, K, V> {
        self.0.drain()
    }

    /// Retains only the elements specified by the predicate. Keeps the
    /// allocated memory for reuse.
    ///
    /// Refer to [`retain`](hb::HashMap::retain) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashMap;
    /// #
    /// let mut map = HashMap::new();
    ///
    /// map.insert("foo", 0);
    /// map.insert("bar", 1);
    /// map.insert("baz", 2);
    ///
    /// map.retain(|key, value| *value == 2);
    ///
    /// assert_eq!(map.len(), 1);
    /// ```
    #[inline]
    pub fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&K, &mut V) -> bool,
    {
        self.0.retain(f);
    }
}

impl<K, V, S> HashMap<K, V, S>
where
    K: Eq + Hash,
    S: BuildHasher,
{
    /// Returns a reference to the value corresponding to the key.
    ///
    /// Refer to [`get`](hb::HashMap::get) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashMap;
    /// let mut map = HashMap::new();
    ///
    /// map.insert("foo", 0);
    ///
    /// assert_eq!(map.get("foo"), Some(&0));
    /// ```
    #[inline]
    pub fn get<Q>(&self, k: &Q) -> Option<&V>
    where
        Q: Hash + Equivalent<K> + ?Sized,
    {
        self.0.get(k)
    }

    /// Returns a mutable reference to the value corresponding to the key.
    ///
    /// Refer to [`get_mut`](hb::HashMap::get_mut) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashMap;
    /// let mut map = HashMap::new();
    ///
    /// map.insert("foo", 0);
    ///
    /// assert_eq!(map.get_mut("foo"), Some(&mut 0));
    /// ```
    #[inline]
    pub fn get_mut<Q>(&mut self, k: &Q) -> Option<&mut V>
    where
        Q: Hash + Equivalent<K> + ?Sized,
    {
        self.0.get_mut(k)
    }

    /// Inserts a key-value pair into the map.
    ///
    /// Refer to [`insert`](hb::HashMap::insert) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashMap;
    /// let mut map = HashMap::new();
    ///
    /// map.insert("foo", 0);
    ///
    /// assert_eq!(map.get("foo"), Some(&0));
    /// ```
    #[inline]
    pub fn insert(&mut self, k: K, v: V) -> Option<V> {
        self.0.insert(k, v)
    }

    /// Removes a key from the map, returning the value at the key if the key
    /// was previously in the map. Keeps the allocated memory for reuse.
    ///
    /// Refer to [`remove`](hb::HashMap::remove) for further details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_platform::collections::HashMap;
    /// let mut map = HashMap::new();
    ///
    /// map.insert("foo", 0);
    ///
    /// assert_eq!(map.remove("foo"), Some(0));
    ///
    /// assert!(map.is_empty());
    /// ```
    #[inline]
    pub fn remove<Q>(&mut self, k: &Q) -> Option<V>
    where
        Q: Hash + Equivalent<K> + ?Sized,
    {
        self.0.remove(k)
    }
}
