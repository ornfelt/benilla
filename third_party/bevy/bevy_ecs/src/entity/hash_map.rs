//! Contains the [`EntityHashMap`] type, a [`HashMap`] pre-configured to use [`EntityHash`] hashing.
//!
//! This module is a lightweight wrapper around Bevy's [`HashMap`] that is more performant for [`Entity`] keys.

use core::{
    fmt::Debug,
    ops::{Deref, DerefMut, Index},
};

use bevy_platform::collections::hash_map::{self, HashMap};

use super::{Entity, EntityEquivalent, EntityHash};

/// A [`HashMap`] pre-configured to use [`EntityHash`] hashing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityHashMap<V>(pub(crate) HashMap<Entity, V, EntityHash>);

impl<V> EntityHashMap<V> {
    /// Creates an empty `EntityHashMap`.
    ///
    /// Equivalent to [`HashMap::with_hasher(EntityHash)`].
    ///
    /// [`HashMap::with_hasher(EntityHash)`]: HashMap::with_hasher
    pub const fn new() -> Self {
        Self(HashMap::with_hasher(EntityHash))
    }

    /// Creates an empty `EntityHashMap` with the specified capacity.
    ///
    /// Equivalent to [`HashMap::with_capacity_and_hasher(n, EntityHash)`].
    ///
    /// [`HashMap:with_capacity_and_hasher(n, EntityHash)`]: HashMap::with_capacity_and_hasher
    pub fn with_capacity(n: usize) -> Self {
        Self(HashMap::with_capacity_and_hasher(n, EntityHash))
    }
}

impl<V> Default for EntityHashMap<V> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<V> Deref for EntityHashMap<V> {
    type Target = HashMap<Entity, V, EntityHash>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<V> DerefMut for EntityHashMap<V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'a, V: Copy> Extend<&'a (Entity, V)> for EntityHashMap<V> {
    fn extend<T: IntoIterator<Item = &'a (Entity, V)>>(&mut self, iter: T) {
        self.0.extend(iter);
    }
}

impl<'a, V: Copy> Extend<(&'a Entity, &'a V)> for EntityHashMap<V> {
    fn extend<T: IntoIterator<Item = (&'a Entity, &'a V)>>(&mut self, iter: T) {
        self.0.extend(iter);
    }
}

impl<V> Extend<(Entity, V)> for EntityHashMap<V> {
    fn extend<T: IntoIterator<Item = (Entity, V)>>(&mut self, iter: T) {
        self.0.extend(iter);
    }
}

impl<V, const N: usize> From<[(Entity, V); N]> for EntityHashMap<V> {
    fn from(value: [(Entity, V); N]) -> Self {
        Self(HashMap::from_iter(value))
    }
}

impl<V> FromIterator<(Entity, V)> for EntityHashMap<V> {
    fn from_iter<I: IntoIterator<Item = (Entity, V)>>(iterable: I) -> Self {
        Self(HashMap::from_iter(iterable))
    }
}

impl<V, Q: EntityEquivalent + ?Sized> Index<&Q> for EntityHashMap<V> {
    type Output = V;
    fn index(&self, key: &Q) -> &V {
        self.0.index(&key.entity())
    }
}

impl<'a, V> IntoIterator for &'a EntityHashMap<V> {
    type Item = (&'a Entity, &'a V);
    type IntoIter = hash_map::Iter<'a, Entity, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<'a, V> IntoIterator for &'a mut EntityHashMap<V> {
    type Item = (&'a Entity, &'a mut V);
    type IntoIter = hash_map::IterMut<'a, Entity, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter_mut()
    }
}

impl<V> IntoIterator for EntityHashMap<V> {
    type Item = (Entity, V);
    type IntoIter = hash_map::IntoIter<Entity, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use static_assertions::assert_impl_all;

    // Check that the HashMaps are Clone if the key/values are Clone
    assert_impl_all!(EntityHashMap::<usize>: Clone);
}
