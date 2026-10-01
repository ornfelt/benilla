//! A wrapper around entity slices with a uniqueness invariant.

use core::{
    array::TryFromSliceError,
    borrow::Borrow,
    fmt::Debug,
    iter::FusedIterator,
    ops::{
        Bound, Deref, Index, IndexMut, Range, RangeFrom, RangeFull, RangeInclusive, RangeTo,
        RangeToInclusive,
    },
    ptr, slice,
};

use alloc::{
    borrow::{Cow, ToOwned},
    boxed::Box,
    collections::VecDeque,
    rc::Rc,
    vec::Vec,
};

use bevy_platform::sync::Arc;

use super::{
    unique_vec::{self, UniqueEntityEquivalentVec},
    Entity, EntityEquivalent, EntitySet, EntitySetIterator, FromEntitySetIterator,
    UniqueEntityEquivalentArray, UniqueEntityIter,
};

/// A slice that contains only unique entities.
///
/// This can be obtained by slicing [`UniqueEntityEquivalentVec`].
///
/// When `T` is [`Entity`], use [`UniqueEntitySlice`].
#[repr(transparent)]
#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct UniqueEntityEquivalentSlice<T: EntityEquivalent>([T]);

/// A slice that contains only unique [`Entity`].
///
/// This is the default case of a [`UniqueEntityEquivalentSlice`].
pub type UniqueEntitySlice = UniqueEntityEquivalentSlice<Entity>;

impl<T: EntityEquivalent> UniqueEntityEquivalentSlice<T> {
    /// Constructs a `UniqueEntityEquivalentSlice` from a [`&[T]`] unsafely.
    ///
    /// # Safety
    ///
    /// `slice` must contain only unique elements.
    pub const unsafe fn from_slice_unchecked(slice: &[T]) -> &Self {
        // SAFETY: UniqueEntityEquivalentSlice is a transparent wrapper around [T].
        unsafe { &*(ptr::from_ref(slice) as *const Self) }
    }

    /// Constructs a `UniqueEntityEquivalentSlice` from a [`&mut [T]`] unsafely.
    ///
    /// # Safety
    ///
    /// `slice` must contain only unique elements.
    pub const unsafe fn from_slice_unchecked_mut(slice: &mut [T]) -> &mut Self {
        // SAFETY: UniqueEntityEquivalentSlice is a transparent wrapper around [T].
        unsafe { &mut *(ptr::from_mut(slice) as *mut Self) }
    }

    /// Casts to `self` to a standard slice.
    pub const fn as_inner(&self) -> &[T] {
        &self.0
    }

    /// Constructs a `UniqueEntityEquivalentSlice` from a [`Box<[T]>`] unsafely.
    ///
    /// # Safety
    ///
    /// `slice` must contain only unique elements.
    pub unsafe fn from_boxed_slice_unchecked(slice: Box<[T]>) -> Box<Self> {
        // SAFETY: UniqueEntityEquivalentSlice is a transparent wrapper around [T].
        unsafe { Box::from_raw(Box::into_raw(slice) as *mut Self) }
    }

    /// Constructs a `UniqueEntityEquivalentSlice` from a [`Arc<[T]>`] unsafely.
    ///
    /// # Safety
    ///
    /// `slice` must contain only unique elements.
    pub unsafe fn from_arc_slice_unchecked(slice: Arc<[T]>) -> Arc<Self> {
        // SAFETY: UniqueEntityEquivalentSlice is a transparent wrapper around [T].
        unsafe { Arc::from_raw(Arc::into_raw(slice) as *mut Self) }
    }

    // Constructs a `UniqueEntityEquivalentSlice` from a [`Rc<[T]>`] unsafely.
    ///
    /// # Safety
    ///
    /// `slice` must contain only unique elements.
    pub unsafe fn from_rc_slice_unchecked(slice: Rc<[T]>) -> Rc<Self> {
        // SAFETY: UniqueEntityEquivalentSlice is a transparent wrapper around [T].
        unsafe { Rc::from_raw(Rc::into_raw(slice) as *mut Self) }
    }

    /// Returns an iterator over the slice.
    pub fn iter(&self) -> Iter<'_, T> {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntityIter::from_iterator_unchecked(self.0.iter()) }
    }

    /// Copies self into a new `UniqueEntityEquivalentVec`.
    pub fn to_vec(&self) -> UniqueEntityEquivalentVec<T>
    where
        T: Clone,
    {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntityEquivalentVec::from_vec_unchecked(self.0.to_vec()) }
    }

    /// Converts `self` into a vector without clones or allocation.
    ///
    /// Equivalent to [`[T]::into_vec`](slice::into_vec).
    pub fn into_vec(self: Box<Self>) -> UniqueEntityEquivalentVec<T> {
        // SAFETY:
        // This matches the implementation of `slice::into_vec`.
        // All elements in the original slice are unique.
        unsafe {
            let len = self.len();
            let vec = Vec::from_raw_parts(Box::into_raw(self).cast::<T>(), len, len);
            UniqueEntityEquivalentVec::from_vec_unchecked(vec)
        }
    }
}

/// Casts a slice of entity slices to a slice of [`UniqueEntityEquivalentSlice`]s.
///
/// # Safety
///
/// All elements in each of the cast slices must be unique.
pub unsafe fn cast_slice_of_unique_entity_slice<'a, 'b, T: EntityEquivalent + 'a>(
    slice: &'b [&'a [T]],
) -> &'b [&'a UniqueEntityEquivalentSlice<T>] {
    // SAFETY: All elements in the original iterator are unique slices.
    unsafe { &*(ptr::from_ref(slice) as *const [&UniqueEntityEquivalentSlice<T>]) }
}

/// Casts a mutable slice of mutable entity slices to a slice of mutable [`UniqueEntityEquivalentSlice`]s.
///
/// # Safety
///
/// All elements in each of the cast slices must be unique.
pub unsafe fn cast_slice_of_mut_unique_entity_slice_mut<'a, 'b, T: EntityEquivalent + 'a>(
    slice: &'b mut [&'a mut [T]],
) -> &'b mut [&'a mut UniqueEntityEquivalentSlice<T>] {
    // SAFETY: All elements in the original iterator are unique slices.
    unsafe { &mut *(ptr::from_mut(slice) as *mut [&mut UniqueEntityEquivalentSlice<T>]) }
}

impl<'a, T: EntityEquivalent> IntoIterator for &'a UniqueEntityEquivalentSlice<T> {
    type Item = &'a T;

    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T: EntityEquivalent> IntoIterator for &'a Box<UniqueEntityEquivalentSlice<T>> {
    type Item = &'a T;

    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<T: EntityEquivalent> IntoIterator for Box<UniqueEntityEquivalentSlice<T>> {
    type Item = T;

    type IntoIter = unique_vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.into_vec().into_iter()
    }
}

impl<T: EntityEquivalent> Deref for UniqueEntityEquivalentSlice<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: EntityEquivalent> AsRef<[T]> for UniqueEntityEquivalentSlice<T> {
    fn as_ref(&self) -> &[T] {
        self
    }
}

impl<T: EntityEquivalent> AsRef<Self> for UniqueEntityEquivalentSlice<T> {
    fn as_ref(&self) -> &Self {
        self
    }
}

impl<T: EntityEquivalent> AsMut<Self> for UniqueEntityEquivalentSlice<T> {
    fn as_mut(&mut self) -> &mut Self {
        self
    }
}

impl<T: EntityEquivalent> Borrow<[T]> for UniqueEntityEquivalentSlice<T> {
    fn borrow(&self) -> &[T] {
        self
    }
}

impl<T: EntityEquivalent + Clone> Clone for Box<UniqueEntityEquivalentSlice<T>> {
    fn clone(&self) -> Self {
        self.to_vec().into_boxed_slice()
    }
}

impl<T: EntityEquivalent> Default for &UniqueEntityEquivalentSlice<T> {
    fn default() -> Self {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntityEquivalentSlice::from_slice_unchecked(Default::default()) }
    }
}

impl<T: EntityEquivalent> Default for &mut UniqueEntityEquivalentSlice<T> {
    fn default() -> Self {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntityEquivalentSlice::from_slice_unchecked_mut(Default::default()) }
    }
}

impl<T: EntityEquivalent> Default for Box<UniqueEntityEquivalentSlice<T>> {
    fn default() -> Self {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntityEquivalentSlice::from_boxed_slice_unchecked(Default::default()) }
    }
}

impl<T: EntityEquivalent + Clone> From<&UniqueEntityEquivalentSlice<T>>
    for Box<UniqueEntityEquivalentSlice<T>>
{
    fn from(value: &UniqueEntityEquivalentSlice<T>) -> Self {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntityEquivalentSlice::from_boxed_slice_unchecked(value.0.into()) }
    }
}

impl<T: EntityEquivalent + Clone> From<&UniqueEntityEquivalentSlice<T>>
    for Arc<UniqueEntityEquivalentSlice<T>>
{
    fn from(value: &UniqueEntityEquivalentSlice<T>) -> Self {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntityEquivalentSlice::from_arc_slice_unchecked(value.0.into()) }
    }
}

impl<T: EntityEquivalent + Clone> From<&UniqueEntityEquivalentSlice<T>>
    for Rc<UniqueEntityEquivalentSlice<T>>
{
    fn from(value: &UniqueEntityEquivalentSlice<T>) -> Self {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntityEquivalentSlice::from_rc_slice_unchecked(value.0.into()) }
    }
}

impl<'a, T: EntityEquivalent + Clone> From<&'a UniqueEntityEquivalentSlice<T>>
    for Cow<'a, UniqueEntityEquivalentSlice<T>>
{
    fn from(value: &'a UniqueEntityEquivalentSlice<T>) -> Self {
        Cow::Borrowed(value)
    }
}

impl<T: EntityEquivalent + Clone, const N: usize> From<UniqueEntityEquivalentArray<T, N>>
    for Box<UniqueEntityEquivalentSlice<T>>
{
    fn from(value: UniqueEntityEquivalentArray<T, N>) -> Self {
        // SAFETY: All elements in the original slice are unique.
        unsafe {
            UniqueEntityEquivalentSlice::from_boxed_slice_unchecked(Box::new(value.into_inner()))
        }
    }
}

impl<'a, T: EntityEquivalent + Clone> From<Cow<'a, UniqueEntityEquivalentSlice<T>>>
    for Box<UniqueEntityEquivalentSlice<T>>
{
    fn from(value: Cow<'a, UniqueEntityEquivalentSlice<T>>) -> Self {
        match value {
            Cow::Borrowed(slice) => Box::from(slice),
            Cow::Owned(slice) => Box::from(slice),
        }
    }
}

impl<T: EntityEquivalent> From<UniqueEntityEquivalentVec<T>>
    for Box<UniqueEntityEquivalentSlice<T>>
{
    fn from(value: UniqueEntityEquivalentVec<T>) -> Self {
        value.into_boxed_slice()
    }
}

impl<T: EntityEquivalent> FromIterator<T> for Box<UniqueEntityEquivalentSlice<T>> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        iter.into_iter()
            .collect::<UniqueEntityEquivalentVec<T>>()
            .into_boxed_slice()
    }
}

impl<T: EntityEquivalent> FromEntitySetIterator<T> for Box<UniqueEntityEquivalentSlice<T>> {
    fn from_entity_set_iter<I: EntitySet<Item = T>>(iter: I) -> Self {
        iter.into_iter()
            .collect_set::<UniqueEntityEquivalentVec<T>>()
            .into_boxed_slice()
    }
}

impl<T: EntityEquivalent + PartialEq<U>, U: EntityEquivalent>
    PartialEq<UniqueEntityEquivalentVec<U>> for &UniqueEntityEquivalentSlice<T>
{
    fn eq(&self, other: &UniqueEntityEquivalentVec<U>) -> bool {
        self.0.eq(other.as_vec())
    }
}

impl<T: EntityEquivalent + PartialEq<U>, U: EntityEquivalent>
    PartialEq<UniqueEntityEquivalentVec<U>> for &mut UniqueEntityEquivalentSlice<T>
{
    fn eq(&self, other: &UniqueEntityEquivalentVec<U>) -> bool {
        self.0.eq(other.as_vec())
    }
}

impl<T: EntityEquivalent + PartialEq<U>, U: EntityEquivalent>
    PartialEq<UniqueEntityEquivalentVec<U>> for UniqueEntityEquivalentSlice<T>
{
    fn eq(&self, other: &UniqueEntityEquivalentVec<U>) -> bool {
        self.0.eq(other.as_vec())
    }
}

impl<T: PartialEq<U>, U: EntityEquivalent, const N: usize>
    PartialEq<&UniqueEntityEquivalentSlice<U>> for [T; N]
{
    fn eq(&self, other: &&UniqueEntityEquivalentSlice<U>) -> bool {
        self.eq(&other.0)
    }
}

impl<T: PartialEq<U> + Clone, U: EntityEquivalent> PartialEq<&UniqueEntityEquivalentSlice<U>>
    for Cow<'_, [T]>
{
    fn eq(&self, other: &&UniqueEntityEquivalentSlice<U>) -> bool {
        self.eq(&&other.0)
    }
}

impl<T: EntityEquivalent + PartialEq<U> + Clone, U: EntityEquivalent>
    PartialEq<&UniqueEntityEquivalentSlice<U>> for Cow<'_, UniqueEntityEquivalentSlice<T>>
{
    fn eq(&self, other: &&UniqueEntityEquivalentSlice<U>) -> bool {
        self.0.eq(&other.0)
    }
}

impl<T: PartialEq<U>, U: EntityEquivalent> PartialEq<&UniqueEntityEquivalentSlice<U>> for Vec<T> {
    fn eq(&self, other: &&UniqueEntityEquivalentSlice<U>) -> bool {
        self.eq(&other.0)
    }
}

impl<T: PartialEq<U>, U: EntityEquivalent> PartialEq<&UniqueEntityEquivalentSlice<U>>
    for VecDeque<T>
{
    fn eq(&self, other: &&UniqueEntityEquivalentSlice<U>) -> bool {
        self.eq(&&other.0)
    }
}

impl<T: PartialEq<U>, U: EntityEquivalent, const N: usize>
    PartialEq<&mut UniqueEntityEquivalentSlice<U>> for [T; N]
{
    fn eq(&self, other: &&mut UniqueEntityEquivalentSlice<U>) -> bool {
        self.eq(&other.0)
    }
}

impl<T: PartialEq<U> + Clone, U: EntityEquivalent> PartialEq<&mut UniqueEntityEquivalentSlice<U>>
    for Cow<'_, [T]>
{
    fn eq(&self, other: &&mut UniqueEntityEquivalentSlice<U>) -> bool {
        self.eq(&&**other)
    }
}

impl<T: EntityEquivalent + PartialEq<U> + Clone, U: EntityEquivalent>
    PartialEq<&mut UniqueEntityEquivalentSlice<U>> for Cow<'_, UniqueEntityEquivalentSlice<T>>
{
    fn eq(&self, other: &&mut UniqueEntityEquivalentSlice<U>) -> bool {
        self.0.eq(&other.0)
    }
}

impl<T: EntityEquivalent + PartialEq<U> + Clone, U: EntityEquivalent>
    PartialEq<UniqueEntityEquivalentVec<U>> for Cow<'_, UniqueEntityEquivalentSlice<T>>
{
    fn eq(&self, other: &UniqueEntityEquivalentVec<U>) -> bool {
        self.0.eq(other.as_vec())
    }
}

impl<T: PartialEq<U>, U: EntityEquivalent> PartialEq<&mut UniqueEntityEquivalentSlice<U>>
    for Vec<T>
{
    fn eq(&self, other: &&mut UniqueEntityEquivalentSlice<U>) -> bool {
        self.eq(&other.0)
    }
}

impl<T: PartialEq<U>, U: EntityEquivalent> PartialEq<&mut UniqueEntityEquivalentSlice<U>>
    for VecDeque<T>
{
    fn eq(&self, other: &&mut UniqueEntityEquivalentSlice<U>) -> bool {
        self.eq(&&other.0)
    }
}

impl<T: EntityEquivalent + PartialEq<U>, U: EntityEquivalent>
    PartialEq<UniqueEntityEquivalentSlice<U>> for [T]
{
    fn eq(&self, other: &UniqueEntityEquivalentSlice<U>) -> bool {
        self.eq(&other.0)
    }
}

impl<T: PartialEq<U>, U: EntityEquivalent, const N: usize> PartialEq<UniqueEntityEquivalentSlice<U>>
    for [T; N]
{
    fn eq(&self, other: &UniqueEntityEquivalentSlice<U>) -> bool {
        self.eq(&other.0)
    }
}

impl<T: EntityEquivalent + PartialEq<U>, U: EntityEquivalent>
    PartialEq<UniqueEntityEquivalentSlice<U>> for Vec<T>
{
    fn eq(&self, other: &UniqueEntityEquivalentSlice<U>) -> bool {
        self.eq(&other.0)
    }
}

impl<T: EntityEquivalent + PartialEq<U>, U, const N: usize> PartialEq<[U; N]>
    for &UniqueEntityEquivalentSlice<T>
{
    fn eq(&self, other: &[U; N]) -> bool {
        self.0.eq(other)
    }
}

impl<T: EntityEquivalent + PartialEq<U>, U, const N: usize> PartialEq<[U; N]>
    for &mut UniqueEntityEquivalentSlice<T>
{
    fn eq(&self, other: &[U; N]) -> bool {
        self.0.eq(other)
    }
}

impl<T: EntityEquivalent + PartialEq<U>, U, const N: usize> PartialEq<[U; N]>
    for UniqueEntityEquivalentSlice<T>
{
    fn eq(&self, other: &[U; N]) -> bool {
        self.0.eq(other)
    }
}

impl<T: EntityEquivalent + PartialEq<U>, U: EntityEquivalent, const N: usize>
    PartialEq<UniqueEntityEquivalentArray<U, N>> for &UniqueEntityEquivalentSlice<T>
{
    fn eq(&self, other: &UniqueEntityEquivalentArray<U, N>) -> bool {
        self.0.eq(&other.0)
    }
}

impl<T: EntityEquivalent + PartialEq<U>, U: EntityEquivalent, const N: usize>
    PartialEq<UniqueEntityEquivalentArray<U, N>> for &mut UniqueEntityEquivalentSlice<T>
{
    fn eq(&self, other: &UniqueEntityEquivalentArray<U, N>) -> bool {
        self.0.eq(&other.0)
    }
}

impl<T: EntityEquivalent + PartialEq<U>, U: EntityEquivalent, const N: usize>
    PartialEq<UniqueEntityEquivalentArray<U, N>> for UniqueEntityEquivalentSlice<T>
{
    fn eq(&self, other: &UniqueEntityEquivalentArray<U, N>) -> bool {
        self.0.eq(&other.0)
    }
}

impl<T: EntityEquivalent + PartialEq<U>, U> PartialEq<Vec<U>> for &UniqueEntityEquivalentSlice<T> {
    fn eq(&self, other: &Vec<U>) -> bool {
        self.0.eq(other)
    }
}

impl<T: EntityEquivalent + PartialEq<U>, U> PartialEq<Vec<U>>
    for &mut UniqueEntityEquivalentSlice<T>
{
    fn eq(&self, other: &Vec<U>) -> bool {
        self.0.eq(other)
    }
}

impl<T: EntityEquivalent + PartialEq<U>, U> PartialEq<Vec<U>> for UniqueEntityEquivalentSlice<T> {
    fn eq(&self, other: &Vec<U>) -> bool {
        self.0.eq(other)
    }
}

impl<T: EntityEquivalent + Clone> ToOwned for UniqueEntityEquivalentSlice<T> {
    type Owned = UniqueEntityEquivalentVec<T>;

    fn to_owned(&self) -> Self::Owned {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntityEquivalentVec::from_vec_unchecked(self.0.to_owned()) }
    }
}

impl<'a, T: EntityEquivalent + Copy, const N: usize> TryFrom<&'a UniqueEntityEquivalentSlice<T>>
    for &'a UniqueEntityEquivalentArray<T, N>
{
    type Error = TryFromSliceError;

    fn try_from(value: &'a UniqueEntityEquivalentSlice<T>) -> Result<Self, Self::Error> {
        <&[T; N]>::try_from(&value.0).map(|array|
                // SAFETY: All elements in the original slice are unique.
                unsafe { UniqueEntityEquivalentArray::from_array_ref_unchecked(array) })
    }
}

impl<T: EntityEquivalent + Copy, const N: usize> TryFrom<&UniqueEntityEquivalentSlice<T>>
    for UniqueEntityEquivalentArray<T, N>
{
    type Error = TryFromSliceError;

    fn try_from(value: &UniqueEntityEquivalentSlice<T>) -> Result<Self, Self::Error> {
        <&Self>::try_from(value).copied()
    }
}

impl<T: EntityEquivalent + Copy, const N: usize> TryFrom<&mut UniqueEntityEquivalentSlice<T>>
    for UniqueEntityEquivalentArray<T, N>
{
    type Error = TryFromSliceError;

    fn try_from(value: &mut UniqueEntityEquivalentSlice<T>) -> Result<Self, Self::Error> {
        <Self>::try_from(&*value)
    }
}

impl<T: EntityEquivalent> Index<(Bound<usize>, Bound<usize>)> for UniqueEntityEquivalentSlice<T> {
    type Output = Self;
    fn index(&self, key: (Bound<usize>, Bound<usize>)) -> &Self {
        // SAFETY: All elements in the original slice are unique.
        unsafe { Self::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<T: EntityEquivalent> Index<Range<usize>> for UniqueEntityEquivalentSlice<T> {
    type Output = Self;
    fn index(&self, key: Range<usize>) -> &Self {
        // SAFETY: All elements in the original slice are unique.
        unsafe { Self::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<T: EntityEquivalent> Index<RangeFrom<usize>> for UniqueEntityEquivalentSlice<T> {
    type Output = Self;
    fn index(&self, key: RangeFrom<usize>) -> &Self {
        // SAFETY: All elements in the original slice are unique.
        unsafe { Self::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<T: EntityEquivalent> Index<RangeFull> for UniqueEntityEquivalentSlice<T> {
    type Output = Self;
    fn index(&self, key: RangeFull) -> &Self {
        // SAFETY: All elements in the original slice are unique.
        unsafe { Self::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<T: EntityEquivalent> Index<RangeInclusive<usize>> for UniqueEntityEquivalentSlice<T> {
    type Output = UniqueEntityEquivalentSlice<T>;
    fn index(&self, key: RangeInclusive<usize>) -> &Self {
        // SAFETY: All elements in the original slice are unique.
        unsafe { Self::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<T: EntityEquivalent> Index<RangeTo<usize>> for UniqueEntityEquivalentSlice<T> {
    type Output = UniqueEntityEquivalentSlice<T>;
    fn index(&self, key: RangeTo<usize>) -> &Self {
        // SAFETY: All elements in the original slice are unique.
        unsafe { Self::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<T: EntityEquivalent> Index<RangeToInclusive<usize>> for UniqueEntityEquivalentSlice<T> {
    type Output = UniqueEntityEquivalentSlice<T>;
    fn index(&self, key: RangeToInclusive<usize>) -> &Self {
        // SAFETY: All elements in the original slice are unique.
        unsafe { Self::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<T: EntityEquivalent> Index<usize> for UniqueEntityEquivalentSlice<T> {
    type Output = T;

    fn index(&self, index: usize) -> &T {
        &self.0[index]
    }
}

impl<T: EntityEquivalent> IndexMut<(Bound<usize>, Bound<usize>)>
    for UniqueEntityEquivalentSlice<T>
{
    fn index_mut(&mut self, key: (Bound<usize>, Bound<usize>)) -> &mut Self {
        // SAFETY: All elements in the original slice are unique.
        unsafe { Self::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

impl<T: EntityEquivalent> IndexMut<Range<usize>> for UniqueEntityEquivalentSlice<T> {
    fn index_mut(&mut self, key: Range<usize>) -> &mut Self {
        // SAFETY: All elements in the original slice are unique.
        unsafe { Self::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

impl<T: EntityEquivalent> IndexMut<RangeFrom<usize>> for UniqueEntityEquivalentSlice<T> {
    fn index_mut(&mut self, key: RangeFrom<usize>) -> &mut Self {
        // SAFETY: All elements in the original slice are unique.
        unsafe { Self::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

impl<T: EntityEquivalent> IndexMut<RangeFull> for UniqueEntityEquivalentSlice<T> {
    fn index_mut(&mut self, key: RangeFull) -> &mut Self {
        // SAFETY: All elements in the original slice are unique.
        unsafe { Self::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

impl<T: EntityEquivalent> IndexMut<RangeInclusive<usize>> for UniqueEntityEquivalentSlice<T> {
    fn index_mut(&mut self, key: RangeInclusive<usize>) -> &mut Self {
        // SAFETY: All elements in the original slice are unique.
        unsafe { Self::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

impl<T: EntityEquivalent> IndexMut<RangeTo<usize>> for UniqueEntityEquivalentSlice<T> {
    fn index_mut(&mut self, key: RangeTo<usize>) -> &mut Self {
        // SAFETY: All elements in the original slice are unique.
        unsafe { Self::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

impl<T: EntityEquivalent> IndexMut<RangeToInclusive<usize>> for UniqueEntityEquivalentSlice<T> {
    fn index_mut(&mut self, key: RangeToInclusive<usize>) -> &mut Self {
        // SAFETY: All elements in the original slice are unique.
        unsafe { Self::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

/// Immutable slice iterator.
///
/// This struct is created by [`iter`] method on [`UniqueEntityEquivalentSlice`] and
/// the [`IntoIterator`] impls on it and [`UniqueEntityEquivalentVec`].
///
/// [`iter`]: `UniqueEntityEquivalentSlice::iter`
pub type Iter<'a, T> = UniqueEntityIter<slice::Iter<'a, T>>;

/// Mutable slice iterator.
pub type IterMut<'a, T> = UniqueEntityIter<slice::IterMut<'a, T>>;

/// An iterator that yields `&UniqueEntityEquivalentSlice`. Note that an entity may appear
/// in multiple slices, depending on the wrapped iterator.
#[derive(Debug)]
pub struct UniqueEntityEquivalentSliceIter<
    'a,
    T: EntityEquivalent + 'a,
    I: Iterator<Item = &'a [T]>,
> {
    pub(crate) iter: I,
}

impl<'a, T: EntityEquivalent + 'a, I: Iterator<Item = &'a [T]>> Iterator
    for UniqueEntityEquivalentSliceIter<'a, T, I>
{
    type Item = &'a UniqueEntityEquivalentSlice<T>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|slice|
        // SAFETY: All elements in the original iterator are unique slices.
        unsafe { UniqueEntityEquivalentSlice::from_slice_unchecked(slice) })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<'a, T: EntityEquivalent + 'a, I: ExactSizeIterator<Item = &'a [T]>> ExactSizeIterator
    for UniqueEntityEquivalentSliceIter<'a, T, I>
{
}

impl<'a, T: EntityEquivalent + 'a, I: DoubleEndedIterator<Item = &'a [T]>> DoubleEndedIterator
    for UniqueEntityEquivalentSliceIter<'a, T, I>
{
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iter.next_back().map(|slice|
            // SAFETY: All elements in the original iterator are unique slices.
            unsafe { UniqueEntityEquivalentSlice::from_slice_unchecked(slice) })
    }
}

impl<'a, T: EntityEquivalent + 'a, I: FusedIterator<Item = &'a [T]>> FusedIterator
    for UniqueEntityEquivalentSliceIter<'a, T, I>
{
}

impl<'a, T: EntityEquivalent + 'a, I: Iterator<Item = &'a [T]> + AsRef<[&'a [T]]>>
    AsRef<[&'a UniqueEntityEquivalentSlice<T>]> for UniqueEntityEquivalentSliceIter<'a, T, I>
{
    fn as_ref(&self) -> &[&'a UniqueEntityEquivalentSlice<T>] {
        // SAFETY:
        unsafe { cast_slice_of_unique_entity_slice(self.iter.as_ref()) }
    }
}

/// An iterator over overlapping subslices of length `size`.
pub type Windows<'a, T = Entity> = UniqueEntityEquivalentSliceIter<'a, T, slice::Windows<'a, T>>;

/// An iterator over a slice in (non-overlapping) chunks (`chunk_size` elements at a
/// time), starting at the beginning of the slice.
pub type Chunks<'a, T = Entity> = UniqueEntityEquivalentSliceIter<'a, T, slice::Chunks<'a, T>>;

/// An iterator over a slice in (non-overlapping) chunks (`chunk_size` elements at a
/// time), starting at the beginning of the slice.
pub type ChunksExact<'a, T = Entity> =
    UniqueEntityEquivalentSliceIter<'a, T, slice::ChunksExact<'a, T>>;

/// An iterator over a slice in (non-overlapping) chunks (`chunk_size` elements at a
/// time), starting at the end of the slice.
pub type RChunks<'a, T = Entity> = UniqueEntityEquivalentSliceIter<'a, T, slice::RChunks<'a, T>>;

/// An iterator over a slice in (non-overlapping) chunks (`chunk_size` elements at a
/// time), starting at the end of the slice.
pub type RChunksExact<'a, T = Entity> =
    UniqueEntityEquivalentSliceIter<'a, T, slice::RChunksExact<'a, T>>;

/// An iterator over slice in (non-overlapping) chunks separated by a predicate.
pub type ChunkBy<'a, P, T = Entity> =
    UniqueEntityEquivalentSliceIter<'a, T, slice::ChunkBy<'a, T, P>>;

/// An iterator over subslices separated by elements that match a predicate
/// function.
pub type Split<'a, P, T = Entity> = UniqueEntityEquivalentSliceIter<'a, T, slice::Split<'a, T, P>>;

/// An iterator over subslices separated by elements that match a predicate
/// function.
pub type SplitInclusive<'a, P, T = Entity> =
    UniqueEntityEquivalentSliceIter<'a, T, slice::SplitInclusive<'a, T, P>>;

/// An iterator over subslices separated by elements that match a predicate
/// function, starting from the end of the slice.
pub type RSplit<'a, P, T = Entity> =
    UniqueEntityEquivalentSliceIter<'a, T, slice::RSplit<'a, T, P>>;

/// An iterator over subslices separated by elements that match a predicate
/// function, limited to a given number of splits.
pub type SplitN<'a, P, T = Entity> =
    UniqueEntityEquivalentSliceIter<'a, T, slice::SplitN<'a, T, P>>;

/// An iterator over subslices separated by elements that match a
/// predicate function, limited to a given number of splits, starting
/// from the end of the slice.
pub type RSplitN<'a, P, T = Entity> =
    UniqueEntityEquivalentSliceIter<'a, T, slice::RSplitN<'a, T, P>>;

/// An iterator that yields `&mut UniqueEntityEquivalentSlice`. Note that an entity may appear
/// in multiple slices, depending on the wrapped iterator.
#[derive(Debug)]
pub struct UniqueEntityEquivalentSliceIterMut<
    'a,
    T: EntityEquivalent + 'a,
    I: Iterator<Item = &'a mut [T]>,
> {
    pub(crate) iter: I,
}

impl<'a, T: EntityEquivalent + 'a, I: Iterator<Item = &'a mut [T]>> Iterator
    for UniqueEntityEquivalentSliceIterMut<'a, T, I>
{
    type Item = &'a mut UniqueEntityEquivalentSlice<T>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|slice|
            // SAFETY: All elements in the original iterator are unique slices.
            unsafe { UniqueEntityEquivalentSlice::from_slice_unchecked_mut(slice) })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<'a, T: EntityEquivalent + 'a, I: ExactSizeIterator<Item = &'a mut [T]>> ExactSizeIterator
    for UniqueEntityEquivalentSliceIterMut<'a, T, I>
{
}

impl<'a, T: EntityEquivalent + 'a, I: DoubleEndedIterator<Item = &'a mut [T]>> DoubleEndedIterator
    for UniqueEntityEquivalentSliceIterMut<'a, T, I>
{
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iter.next_back().map(|slice|
            // SAFETY: All elements in the original iterator are unique slices.
            unsafe { UniqueEntityEquivalentSlice::from_slice_unchecked_mut(slice) })
    }
}

impl<'a, T: EntityEquivalent + 'a, I: FusedIterator<Item = &'a mut [T]>> FusedIterator
    for UniqueEntityEquivalentSliceIterMut<'a, T, I>
{
}

impl<'a, T: EntityEquivalent + 'a, I: Iterator<Item = &'a mut [T]> + AsRef<[&'a [T]]>>
    AsRef<[&'a UniqueEntityEquivalentSlice<T>]> for UniqueEntityEquivalentSliceIterMut<'a, T, I>
{
    fn as_ref(&self) -> &[&'a UniqueEntityEquivalentSlice<T>] {
        // SAFETY: All elements in the original iterator are unique slices.
        unsafe { cast_slice_of_unique_entity_slice(self.iter.as_ref()) }
    }
}

impl<'a, T: EntityEquivalent + 'a, I: Iterator<Item = &'a mut [T]> + AsMut<[&'a mut [T]]>>
    AsMut<[&'a mut UniqueEntityEquivalentSlice<T>]>
    for UniqueEntityEquivalentSliceIterMut<'a, T, I>
{
    fn as_mut(&mut self) -> &mut [&'a mut UniqueEntityEquivalentSlice<T>] {
        // SAFETY: All elements in the original iterator are unique slices.
        unsafe { cast_slice_of_mut_unique_entity_slice_mut(self.iter.as_mut()) }
    }
}

/// An iterator over a slice in (non-overlapping) mutable chunks (`chunk_size`
/// elements at a time), starting at the beginning of the slice.
pub type ChunksMut<'a, T = Entity> =
    UniqueEntityEquivalentSliceIterMut<'a, T, slice::ChunksMut<'a, T>>;

/// An iterator over a slice in (non-overlapping) mutable chunks (`chunk_size`
/// elements at a time), starting at the beginning of the slice.
pub type ChunksExactMut<'a, T = Entity> =
    UniqueEntityEquivalentSliceIterMut<'a, T, slice::ChunksExactMut<'a, T>>;

/// An iterator over a slice in (non-overlapping) mutable chunks (`chunk_size`
/// elements at a time), starting at the end of the slice.
pub type RChunksMut<'a, T = Entity> =
    UniqueEntityEquivalentSliceIterMut<'a, T, slice::RChunksMut<'a, T>>;

/// An iterator over a slice in (non-overlapping) mutable chunks (`chunk_size`
/// elements at a time), starting at the end of the slice.
pub type RChunksExactMut<'a, T = Entity> =
    UniqueEntityEquivalentSliceIterMut<'a, T, slice::RChunksExactMut<'a, T>>;

/// An iterator over slice in (non-overlapping) mutable chunks separated
/// by a predicate.
pub type ChunkByMut<'a, P, T = Entity> =
    UniqueEntityEquivalentSliceIterMut<'a, T, slice::ChunkByMut<'a, T, P>>;

/// An iterator over the mutable subslices of the vector which are separated
/// by elements that match `pred`.
pub type SplitMut<'a, P, T = Entity> =
    UniqueEntityEquivalentSliceIterMut<'a, T, slice::SplitMut<'a, T, P>>;

/// An iterator over the mutable subslices of the vector which are separated
/// by elements that match `pred`. Unlike `SplitMut`, it contains the matched
/// parts in the ends of the subslices.
pub type SplitInclusiveMut<'a, P, T = Entity> =
    UniqueEntityEquivalentSliceIterMut<'a, T, slice::SplitInclusiveMut<'a, T, P>>;

/// An iterator over the subslices of the vector which are separated
/// by elements that match `pred`, starting from the end of the slice.
pub type RSplitMut<'a, P, T = Entity> =
    UniqueEntityEquivalentSliceIterMut<'a, T, slice::RSplitMut<'a, T, P>>;

/// An iterator over subslices separated by elements that match a predicate
/// function, limited to a given number of splits.
pub type SplitNMut<'a, P, T = Entity> =
    UniqueEntityEquivalentSliceIterMut<'a, T, slice::SplitNMut<'a, T, P>>;

/// An iterator over subslices separated by elements that match a
/// predicate function, limited to a given number of splits, starting
/// from the end of the slice.
pub type RSplitNMut<'a, P, T = Entity> =
    UniqueEntityEquivalentSliceIterMut<'a, T, slice::RSplitNMut<'a, T, P>>;
