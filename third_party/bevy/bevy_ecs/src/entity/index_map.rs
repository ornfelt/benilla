//! Contains the [`EntityIndexMap`] type, an [`IndexMap`] pre-configured to use [`EntityHash`] hashing.
//!
//! This module is a lightweight wrapper around `indexmap`'s [`IndexMap`] that is more performant for [`Entity`] keys.

use core::{
    cmp::Ordering,
    fmt::{self, Debug, Formatter},
    hash::{BuildHasher, Hash, Hasher},
    iter::FusedIterator,
    marker::PhantomData,
    ops::{
        Bound, Deref, DerefMut, Index, IndexMut, Range, RangeFrom, RangeFull, RangeInclusive,
        RangeTo, RangeToInclusive,
    },
    ptr,
};

pub use indexmap::map::Entry;
use indexmap::map::{self, IndexMap};

use super::{Entity, EntityEquivalent, EntityHash};

use bevy_platform::prelude::Box;

/// A [`IndexMap`] pre-configured to use [`EntityHash`] hashing.
#[derive(Debug, Clone)]
pub struct EntityIndexMap<V>(pub(crate) IndexMap<Entity, V, EntityHash>);

impl<V> Default for EntityIndexMap<V> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<V> Deref for EntityIndexMap<V> {
    type Target = IndexMap<Entity, V, EntityHash>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<V> DerefMut for EntityIndexMap<V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'a, V: Copy> Extend<(&'a Entity, &'a V)> for EntityIndexMap<V> {
    fn extend<T: IntoIterator<Item = (&'a Entity, &'a V)>>(&mut self, iter: T) {
        self.0.extend(iter);
    }
}

impl<V> Extend<(Entity, V)> for EntityIndexMap<V> {
    fn extend<T: IntoIterator<Item = (Entity, V)>>(&mut self, iter: T) {
        self.0.extend(iter);
    }
}

impl<V, const N: usize> From<[(Entity, V); N]> for EntityIndexMap<V> {
    fn from(value: [(Entity, V); N]) -> Self {
        Self(IndexMap::from_iter(value))
    }
}

impl<V> FromIterator<(Entity, V)> for EntityIndexMap<V> {
    fn from_iter<I: IntoIterator<Item = (Entity, V)>>(iterable: I) -> Self {
        Self(IndexMap::from_iter(iterable))
    }
}

impl<V, Q: EntityEquivalent + ?Sized> Index<&Q> for EntityIndexMap<V> {
    type Output = V;
    fn index(&self, key: &Q) -> &V {
        self.0.index(&key.entity())
    }
}

impl<V> Index<(Bound<usize>, Bound<usize>)> for EntityIndexMap<V> {
    type Output = Slice<V>;
    fn index(&self, key: (Bound<usize>, Bound<usize>)) -> &Self::Output {
        // SAFETY: The source IndexMap uses EntityHash.
        unsafe { Slice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<V> Index<Range<usize>> for EntityIndexMap<V> {
    type Output = Slice<V>;
    fn index(&self, key: Range<usize>) -> &Self::Output {
        // SAFETY: The source IndexMap uses EntityHash.
        unsafe { Slice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<V> Index<RangeFrom<usize>> for EntityIndexMap<V> {
    type Output = Slice<V>;
    fn index(&self, key: RangeFrom<usize>) -> &Self::Output {
        // SAFETY: The source IndexMap uses EntityHash.
        unsafe { Slice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<V> Index<RangeFull> for EntityIndexMap<V> {
    type Output = Slice<V>;
    fn index(&self, key: RangeFull) -> &Self::Output {
        // SAFETY: The source IndexMap uses EntityHash.
        unsafe { Slice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<V> Index<RangeInclusive<usize>> for EntityIndexMap<V> {
    type Output = Slice<V>;
    fn index(&self, key: RangeInclusive<usize>) -> &Self::Output {
        // SAFETY: The source IndexMap uses EntityHash.
        unsafe { Slice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<V> Index<RangeTo<usize>> for EntityIndexMap<V> {
    type Output = Slice<V>;
    fn index(&self, key: RangeTo<usize>) -> &Self::Output {
        // SAFETY: The source IndexMap uses EntityHash.
        unsafe { Slice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<V> Index<RangeToInclusive<usize>> for EntityIndexMap<V> {
    type Output = Slice<V>;
    fn index(&self, key: RangeToInclusive<usize>) -> &Self::Output {
        // SAFETY: The source IndexMap uses EntityHash.
        unsafe { Slice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<V> Index<usize> for EntityIndexMap<V> {
    type Output = V;
    fn index(&self, key: usize) -> &V {
        self.0.index(key)
    }
}

impl<V, Q: EntityEquivalent + ?Sized> IndexMut<&Q> for EntityIndexMap<V> {
    fn index_mut(&mut self, key: &Q) -> &mut V {
        self.0.index_mut(&key.entity())
    }
}

impl<V> IndexMut<(Bound<usize>, Bound<usize>)> for EntityIndexMap<V> {
    fn index_mut(&mut self, key: (Bound<usize>, Bound<usize>)) -> &mut Self::Output {
        // SAFETY: The source IndexMap uses EntityHash.
        unsafe { Slice::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

impl<V> IndexMut<Range<usize>> for EntityIndexMap<V> {
    fn index_mut(&mut self, key: Range<usize>) -> &mut Self::Output {
        // SAFETY: The source IndexMap uses EntityHash.
        unsafe { Slice::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

impl<V> IndexMut<RangeFrom<usize>> for EntityIndexMap<V> {
    fn index_mut(&mut self, key: RangeFrom<usize>) -> &mut Self::Output {
        // SAFETY: The source IndexMap uses EntityHash.
        unsafe { Slice::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

impl<V> IndexMut<RangeFull> for EntityIndexMap<V> {
    fn index_mut(&mut self, key: RangeFull) -> &mut Self::Output {
        // SAFETY: The source IndexMap uses EntityHash.
        unsafe { Slice::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

impl<V> IndexMut<RangeInclusive<usize>> for EntityIndexMap<V> {
    fn index_mut(&mut self, key: RangeInclusive<usize>) -> &mut Self::Output {
        // SAFETY: The source IndexMap uses EntityHash.
        unsafe { Slice::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

impl<V> IndexMut<RangeTo<usize>> for EntityIndexMap<V> {
    fn index_mut(&mut self, key: RangeTo<usize>) -> &mut Self::Output {
        // SAFETY: The source IndexMap uses EntityHash.
        unsafe { Slice::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

impl<V> IndexMut<RangeToInclusive<usize>> for EntityIndexMap<V> {
    fn index_mut(&mut self, key: RangeToInclusive<usize>) -> &mut Self::Output {
        // SAFETY: The source IndexMap uses EntityHash.
        unsafe { Slice::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

impl<V> IndexMut<usize> for EntityIndexMap<V> {
    fn index_mut(&mut self, key: usize) -> &mut V {
        self.0.index_mut(key)
    }
}

impl<'a, V> IntoIterator for &'a EntityIndexMap<V> {
    type Item = (&'a Entity, &'a V);
    type IntoIter = Iter<'a, V>;

    fn into_iter(self) -> Self::IntoIter {
        Iter(self.0.iter(), PhantomData)
    }
}

impl<'a, V> IntoIterator for &'a mut EntityIndexMap<V> {
    type Item = (&'a Entity, &'a mut V);
    type IntoIter = IterMut<'a, V>;

    fn into_iter(self) -> Self::IntoIter {
        IterMut(self.0.iter_mut(), PhantomData)
    }
}

impl<V> IntoIterator for EntityIndexMap<V> {
    type Item = (Entity, V);
    type IntoIter = IntoIter<V>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter(self.0.into_iter(), PhantomData)
    }
}

impl<V1, V2, S2> PartialEq<IndexMap<Entity, V2, S2>> for EntityIndexMap<V1>
where
    V1: PartialEq<V2>,
    S2: BuildHasher,
{
    fn eq(&self, other: &IndexMap<Entity, V2, S2>) -> bool {
        self.0.eq(other)
    }
}

impl<V1, V2> PartialEq<EntityIndexMap<V2>> for EntityIndexMap<V1>
where
    V1: PartialEq<V2>,
{
    fn eq(&self, other: &EntityIndexMap<V2>) -> bool {
        self.0.eq(other)
    }
}

impl<V: Eq> Eq for EntityIndexMap<V> {}

/// A dynamically-sized slice of key-value pairs in an [`EntityIndexMap`].
///
/// Equivalent to an [`indexmap::map::Slice<V>`] whose source [`IndexMap`]
/// uses [`EntityHash`].
#[repr(transparent)]
pub struct Slice<V, S = EntityHash>(PhantomData<S>, map::Slice<Entity, V>);

impl<V> Slice<V> {
    /// Constructs a [`entity::index_map::Slice`] from a [`indexmap::map::Slice`] unsafely.
    ///
    /// # Safety
    ///
    /// `slice` must stem from an [`IndexMap`] using [`EntityHash`].
    ///
    /// [`entity::index_map::Slice`]: `crate::entity::index_map::Slice`
    pub const unsafe fn from_slice_unchecked(slice: &map::Slice<Entity, V>) -> &Self {
        // SAFETY: Slice is a transparent wrapper around indexmap::map::Slice.
        unsafe { &*(ptr::from_ref(slice) as *const Self) }
    }

    /// Constructs a [`entity::index_map::Slice`] from a [`indexmap::map::Slice`] unsafely.
    ///
    /// # Safety
    ///
    /// `slice` must stem from an [`IndexMap`] using [`EntityHash`].
    ///
    /// [`entity::index_map::Slice`]: `crate::entity::index_map::Slice`
    pub const unsafe fn from_slice_unchecked_mut(slice: &mut map::Slice<Entity, V>) -> &mut Self {
        // SAFETY: Slice is a transparent wrapper around indexmap::map::Slice.
        unsafe { &mut *(ptr::from_mut(slice) as *mut Self) }
    }

    /// Constructs a boxed [`entity::index_map::Slice`] from a boxed [`indexmap::map::Slice`] unsafely.
    ///
    /// # Safety
    ///
    /// `slice` must stem from an [`IndexMap`] using [`EntityHash`].
    ///
    /// [`entity::index_map::Slice`]: `crate::entity::index_map::Slice`
    pub unsafe fn from_boxed_slice_unchecked(slice: Box<map::Slice<Entity, V>>) -> Box<Self> {
        // SAFETY: Slice is a transparent wrapper around indexmap::map::Slice.
        unsafe { Box::from_raw(Box::into_raw(slice) as *mut Self) }
    }

    /// Casts a reference to `self` to the inner slice.
    #[expect(
        clippy::borrowed_box,
        reason = "We wish to access the Box API of the inner type, without consuming it."
    )]
    pub fn as_boxed_inner(self: &Box<Self>) -> &Box<map::Slice<Entity, V>> {
        // SAFETY: Slice is a transparent wrapper around indexmap::map::Slice.
        unsafe { &*(ptr::from_ref(self).cast::<Box<map::Slice<Entity, V>>>()) }
    }

    /// Casts `self` to the inner slice.
    pub fn into_boxed_inner(self: Box<Self>) -> Box<map::Slice<Entity, V>> {
        // SAFETY: Slice is a transparent wrapper around indexmap::map::Slice.
        unsafe { Box::from_raw(Box::into_raw(self) as *mut map::Slice<Entity, V>) }
    }
}

impl<V> Deref for Slice<V> {
    type Target = map::Slice<Entity, V>;

    fn deref(&self) -> &Self::Target {
        &self.1
    }
}

impl<V: Debug> Debug for Slice<V> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Slice")
            .field(&self.0)
            .field(&&self.1)
            .finish()
    }
}

impl<V: Clone> Clone for Box<Slice<V>> {
    fn clone(&self) -> Self {
        // SAFETY: This a clone of a valid slice.
        unsafe { Slice::from_boxed_slice_unchecked(self.as_boxed_inner().clone()) }
    }
}

impl<V> Default for &Slice<V> {
    fn default() -> Self {
        // SAFETY: The source slice is empty.
        unsafe { Slice::from_slice_unchecked(<&map::Slice<Entity, V>>::default()) }
    }
}

impl<V> Default for &mut Slice<V> {
    fn default() -> Self {
        // SAFETY: The source slice is empty.
        unsafe { Slice::from_slice_unchecked_mut(<&mut map::Slice<Entity, V>>::default()) }
    }
}

impl<V> Default for Box<Slice<V>> {
    fn default() -> Self {
        // SAFETY: The source slice is empty.
        unsafe { Slice::from_boxed_slice_unchecked(<Box<map::Slice<Entity, V>>>::default()) }
    }
}

impl<V: Copy> From<&Slice<V>> for Box<Slice<V>> {
    fn from(value: &Slice<V>) -> Self {
        // SAFETY: This slice is a copy of a valid slice.
        unsafe { Slice::from_boxed_slice_unchecked(value.1.into()) }
    }
}

impl<V: Hash> Hash for Slice<V> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.1.hash(state);
    }
}

impl<'a, V> IntoIterator for &'a Slice<V> {
    type Item = (&'a Entity, &'a V);
    type IntoIter = Iter<'a, V>;

    fn into_iter(self) -> Self::IntoIter {
        Iter(self.1.iter(), PhantomData)
    }
}

impl<'a, V> IntoIterator for &'a mut Slice<V> {
    type Item = (&'a Entity, &'a mut V);
    type IntoIter = IterMut<'a, V>;

    fn into_iter(self) -> Self::IntoIter {
        IterMut(self.1.iter_mut(), PhantomData)
    }
}

impl<V> IntoIterator for Box<Slice<V>> {
    type Item = (Entity, V);
    type IntoIter = IntoIter<V>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter(self.into_boxed_inner().into_iter(), PhantomData)
    }
}

impl<V: PartialOrd> PartialOrd for Slice<V> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.1.partial_cmp(&other.1)
    }
}

impl<V: Ord> Ord for Slice<V> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.1.cmp(other)
    }
}

impl<V: PartialEq> PartialEq for Slice<V> {
    fn eq(&self, other: &Self) -> bool {
        self.1 == other.1
    }
}

impl<V: Eq> Eq for Slice<V> {}

impl<V> Index<(Bound<usize>, Bound<usize>)> for Slice<V> {
    type Output = Self;
    fn index(&self, key: (Bound<usize>, Bound<usize>)) -> &Self {
        // SAFETY: This a subslice of a valid slice.
        unsafe { Self::from_slice_unchecked(self.1.index(key)) }
    }
}

impl<V> Index<Range<usize>> for Slice<V> {
    type Output = Self;
    fn index(&self, key: Range<usize>) -> &Self {
        // SAFETY: This a subslice of a valid slice.
        unsafe { Self::from_slice_unchecked(self.1.index(key)) }
    }
}

impl<V> Index<RangeFrom<usize>> for Slice<V> {
    type Output = Self;
    fn index(&self, key: RangeFrom<usize>) -> &Self {
        // SAFETY: This a subslice of a valid slice.
        unsafe { Self::from_slice_unchecked(self.1.index(key)) }
    }
}

impl<V> Index<RangeFull> for Slice<V> {
    type Output = Self;
    fn index(&self, key: RangeFull) -> &Self {
        // SAFETY: This a subslice of a valid slice.
        unsafe { Self::from_slice_unchecked(self.1.index(key)) }
    }
}

impl<V> Index<RangeInclusive<usize>> for Slice<V> {
    type Output = Self;
    fn index(&self, key: RangeInclusive<usize>) -> &Self {
        // SAFETY: This a subslice of a valid slice.
        unsafe { Self::from_slice_unchecked(self.1.index(key)) }
    }
}

impl<V> Index<RangeTo<usize>> for Slice<V> {
    type Output = Self;
    fn index(&self, key: RangeTo<usize>) -> &Self {
        // SAFETY: This a subslice of a valid slice.
        unsafe { Self::from_slice_unchecked(self.1.index(key)) }
    }
}

impl<V> Index<RangeToInclusive<usize>> for Slice<V> {
    type Output = Self;
    fn index(&self, key: RangeToInclusive<usize>) -> &Self {
        // SAFETY: This a subslice of a valid slice.
        unsafe { Self::from_slice_unchecked(self.1.index(key)) }
    }
}

impl<V> Index<usize> for Slice<V> {
    type Output = V;
    fn index(&self, key: usize) -> &V {
        self.1.index(key)
    }
}

impl<V> IndexMut<(Bound<usize>, Bound<usize>)> for Slice<V> {
    fn index_mut(&mut self, key: (Bound<usize>, Bound<usize>)) -> &mut Self {
        // SAFETY: This a subslice of a valid slice.
        unsafe { Self::from_slice_unchecked_mut(self.1.index_mut(key)) }
    }
}

impl<V> IndexMut<Range<usize>> for Slice<V> {
    fn index_mut(&mut self, key: Range<usize>) -> &mut Self {
        // SAFETY: This a subslice of a valid slice.
        unsafe { Self::from_slice_unchecked_mut(self.1.index_mut(key)) }
    }
}

impl<V> IndexMut<RangeFrom<usize>> for Slice<V> {
    fn index_mut(&mut self, key: RangeFrom<usize>) -> &mut Self {
        // SAFETY: This a subslice of a valid slice.
        unsafe { Self::from_slice_unchecked_mut(self.1.index_mut(key)) }
    }
}

impl<V> IndexMut<RangeFull> for Slice<V> {
    fn index_mut(&mut self, key: RangeFull) -> &mut Self {
        // SAFETY: This a subslice of a valid slice.
        unsafe { Self::from_slice_unchecked_mut(self.1.index_mut(key)) }
    }
}

impl<V> IndexMut<RangeInclusive<usize>> for Slice<V> {
    fn index_mut(&mut self, key: RangeInclusive<usize>) -> &mut Self {
        // SAFETY: This a subslice of a valid slice.
        unsafe { Self::from_slice_unchecked_mut(self.1.index_mut(key)) }
    }
}

impl<V> IndexMut<RangeTo<usize>> for Slice<V> {
    fn index_mut(&mut self, key: RangeTo<usize>) -> &mut Self {
        // SAFETY: This a subslice of a valid slice.
        unsafe { Self::from_slice_unchecked_mut(self.1.index_mut(key)) }
    }
}

impl<V> IndexMut<RangeToInclusive<usize>> for Slice<V> {
    fn index_mut(&mut self, key: RangeToInclusive<usize>) -> &mut Self {
        // SAFETY: This a subslice of a valid slice.
        unsafe { Self::from_slice_unchecked_mut(self.1.index_mut(key)) }
    }
}

impl<V> IndexMut<usize> for Slice<V> {
    fn index_mut(&mut self, key: usize) -> &mut V {
        self.1.index_mut(key)
    }
}

/// An iterator over the entries of an [`EntityIndexMap`].
pub struct Iter<'a, V, S = EntityHash>(map::Iter<'a, Entity, V>, PhantomData<S>);

impl<'a, V> Deref for Iter<'a, V> {
    type Target = map::Iter<'a, Entity, V>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, V> Iterator for Iter<'a, V> {
    type Item = (&'a Entity, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl<V> DoubleEndedIterator for Iter<'_, V> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back()
    }
}

impl<V> ExactSizeIterator for Iter<'_, V> {}

impl<V> FusedIterator for Iter<'_, V> {}

impl<V> Clone for Iter<'_, V> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), PhantomData)
    }
}

impl<V: Debug> Debug for Iter<'_, V> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Iter").field(&self.0).field(&self.1).finish()
    }
}

impl<V> Default for Iter<'_, V> {
    fn default() -> Self {
        Self(Default::default(), PhantomData)
    }
}

/// A mutable iterator over the entries of an [`EntityIndexMap`].
pub struct IterMut<'a, V, S = EntityHash>(map::IterMut<'a, Entity, V>, PhantomData<S>);

impl<'a, V> Deref for IterMut<'a, V> {
    type Target = map::IterMut<'a, Entity, V>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, V> Iterator for IterMut<'a, V> {
    type Item = (&'a Entity, &'a mut V);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl<V> DoubleEndedIterator for IterMut<'_, V> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back()
    }
}

impl<V> ExactSizeIterator for IterMut<'_, V> {}

impl<V> FusedIterator for IterMut<'_, V> {}

impl<V: Debug> Debug for IterMut<'_, V> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("IterMut")
            .field(&self.0)
            .field(&self.1)
            .finish()
    }
}

impl<V> Default for IterMut<'_, V> {
    fn default() -> Self {
        Self(Default::default(), PhantomData)
    }
}

/// An owning iterator over the entries of an [`IndexMap`].
///
/// This `struct` is created by the [`IndexMap::into_iter`] method
/// (provided by the [`IntoIterator`] trait). See its documentation for more.
pub struct IntoIter<V, S = EntityHash>(map::IntoIter<Entity, V>, PhantomData<S>);

impl<V> Deref for IntoIter<V> {
    type Target = map::IntoIter<Entity, V>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<V> Iterator for IntoIter<V> {
    type Item = (Entity, V);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl<V> DoubleEndedIterator for IntoIter<V> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back()
    }
}

impl<V> ExactSizeIterator for IntoIter<V> {}

impl<V> FusedIterator for IntoIter<V> {}

impl<V: Clone> Clone for IntoIter<V> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), PhantomData)
    }
}

impl<V: Debug> Debug for IntoIter<V> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("IntoIter")
            .field(&self.0)
            .field(&self.1)
            .finish()
    }
}

impl<V> Default for IntoIter<V> {
    fn default() -> Self {
        Self(Default::default(), PhantomData)
    }
}
