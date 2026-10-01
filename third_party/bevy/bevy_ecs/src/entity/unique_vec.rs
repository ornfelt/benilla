//! A wrapper around entity [`Vec`]s with a uniqueness invariant.

use core::{
    borrow::{Borrow, BorrowMut},
    ops::{
        Bound, Deref, DerefMut, Index, IndexMut, Range, RangeFrom, RangeFull, RangeInclusive,
        RangeTo, RangeToInclusive,
    },
};

use alloc::{
    borrow::{Cow, ToOwned},
    boxed::Box,
    collections::{BTreeSet, BinaryHeap, VecDeque},
    rc::Rc,
    vec::{self, Vec},
};

use bevy_platform::sync::Arc;

use super::{
    unique_slice::{self, UniqueEntityEquivalentSlice},
    Entity, EntityEquivalent, EntitySet, FromEntitySetIterator, UniqueEntityEquivalentArray,
    UniqueEntityIter,
};

/// A `Vec` that contains only unique entities.
///
/// "Unique" means that `x != y` holds for any 2 entities in this collection.
/// This is always true when less than 2 entities are present.
///
/// This type is best obtained by its `FromEntitySetIterator` impl, via either
/// `EntityIterator::collect_set` or `UniqueEntityEquivalentVec::from_entity_iter`.
///
/// While this type can be constructed via `Iterator::collect`, doing so is inefficient,
/// and not recommended.
///
/// When `T` is [`Entity`], use the [`UniqueEntityVec`] alias.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct UniqueEntityEquivalentVec<T: EntityEquivalent>(Vec<T>);

/// A `Vec` that contains only unique [`Entity`].
///
/// This is the default case of a [`UniqueEntityEquivalentVec`].
pub type UniqueEntityVec = UniqueEntityEquivalentVec<Entity>;

impl<T: EntityEquivalent> UniqueEntityEquivalentVec<T> {
    /// Constructs a new, empty `UniqueEntityEquivalentVec<T>` with at least the specified capacity.
    ///
    /// Equivalent to [`Vec::with_capacity`]
    pub fn with_capacity(capacity: usize) -> Self {
        Self(Vec::with_capacity(capacity))
    }

    /// Constructs a `UniqueEntityEquivalentVec` from a [`Vec<T>`] unsafely.
    ///
    /// # Safety
    ///
    /// `vec` must contain only unique elements.
    pub unsafe fn from_vec_unchecked(vec: Vec<T>) -> Self {
        Self(vec)
    }

    /// Returns a reference to the inner [`Vec<T>`].
    pub fn as_vec(&self) -> &Vec<T> {
        &self.0
    }

    /// Reserves capacity for at least `additional` more elements to be inserted
    /// in the given `Vec<T>`.
    ///
    /// Equivalent to [`Vec::reserve`].
    pub fn reserve(&mut self, additional: usize) {
        self.0.reserve(additional);
    }

    /// Converts the vector into `Box<UniqueEntityEquivalentSlice<T>>`.
    pub fn into_boxed_slice(self) -> Box<UniqueEntityEquivalentSlice<T>> {
        // SAFETY: UniqueEntityEquivalentSlice is a transparent wrapper around [T].
        unsafe {
            UniqueEntityEquivalentSlice::from_boxed_slice_unchecked(self.0.into_boxed_slice())
        }
    }

    /// Returns `true` if the vector contains no elements.
    ///
    /// Equivalent to [`Vec::is_empty`].
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<T: EntityEquivalent> Default for UniqueEntityEquivalentVec<T> {
    fn default() -> Self {
        Self(Vec::default())
    }
}

impl<T: EntityEquivalent> Deref for UniqueEntityEquivalentVec<T> {
    type Target = UniqueEntityEquivalentSlice<T>;

    fn deref(&self) -> &Self::Target {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntityEquivalentSlice::from_slice_unchecked(&self.0) }
    }
}

impl<T: EntityEquivalent> DerefMut for UniqueEntityEquivalentVec<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntityEquivalentSlice::from_slice_unchecked_mut(&mut self.0) }
    }
}

impl<'a, T: EntityEquivalent> IntoIterator for &'a UniqueEntityEquivalentVec<T>
where
    &'a T: EntityEquivalent,
{
    type Item = &'a T;

    type IntoIter = unique_slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        // SAFETY: `self` contains only unique elements.
        unsafe { UniqueEntityIter::from_iterator_unchecked(self.0.iter()) }
    }
}

impl<T: EntityEquivalent> IntoIterator for UniqueEntityEquivalentVec<T> {
    type Item = T;

    type IntoIter = IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        // SAFETY: `self` contains only unique elements.
        unsafe { UniqueEntityIter::from_iterator_unchecked(self.0.into_iter()) }
    }
}

impl<T: EntityEquivalent> AsMut<Self> for UniqueEntityEquivalentVec<T> {
    fn as_mut(&mut self) -> &mut UniqueEntityEquivalentVec<T> {
        self
    }
}

impl<T: EntityEquivalent> AsMut<UniqueEntityEquivalentSlice<T>> for UniqueEntityEquivalentVec<T> {
    fn as_mut(&mut self) -> &mut UniqueEntityEquivalentSlice<T> {
        self
    }
}

impl<T: EntityEquivalent> AsRef<Self> for UniqueEntityEquivalentVec<T> {
    fn as_ref(&self) -> &Self {
        self
    }
}

impl<T: EntityEquivalent> AsRef<Vec<T>> for UniqueEntityEquivalentVec<T> {
    fn as_ref(&self) -> &Vec<T> {
        &self.0
    }
}

impl<T: EntityEquivalent> Borrow<Vec<T>> for UniqueEntityEquivalentVec<T> {
    fn borrow(&self) -> &Vec<T> {
        &self.0
    }
}

impl<T: EntityEquivalent> AsRef<[T]> for UniqueEntityEquivalentVec<T> {
    fn as_ref(&self) -> &[T] {
        &self.0
    }
}

impl<T: EntityEquivalent> AsRef<UniqueEntityEquivalentSlice<T>> for UniqueEntityEquivalentVec<T> {
    fn as_ref(&self) -> &UniqueEntityEquivalentSlice<T> {
        self
    }
}

impl<T: EntityEquivalent> Borrow<[T]> for UniqueEntityEquivalentVec<T> {
    fn borrow(&self) -> &[T] {
        &self.0
    }
}

impl<T: EntityEquivalent> Borrow<UniqueEntityEquivalentSlice<T>> for UniqueEntityEquivalentVec<T> {
    fn borrow(&self) -> &UniqueEntityEquivalentSlice<T> {
        self
    }
}

impl<T: EntityEquivalent> BorrowMut<UniqueEntityEquivalentSlice<T>>
    for UniqueEntityEquivalentVec<T>
{
    fn borrow_mut(&mut self) -> &mut UniqueEntityEquivalentSlice<T> {
        self
    }
}

impl<T: EntityEquivalent + PartialEq<U>, U> PartialEq<Vec<U>> for UniqueEntityEquivalentVec<T> {
    fn eq(&self, other: &Vec<U>) -> bool {
        self.0.eq(other)
    }
}

impl<T: EntityEquivalent + PartialEq<U>, U> PartialEq<&[U]> for UniqueEntityEquivalentVec<T> {
    fn eq(&self, other: &&[U]) -> bool {
        self.0.eq(other)
    }
}

impl<T: EntityEquivalent + PartialEq<U>, U: EntityEquivalent>
    PartialEq<&UniqueEntityEquivalentSlice<U>> for UniqueEntityEquivalentVec<T>
{
    fn eq(&self, other: &&UniqueEntityEquivalentSlice<U>) -> bool {
        self.0.eq(other)
    }
}

impl<T: EntityEquivalent + PartialEq<U>, U> PartialEq<&mut [U]> for UniqueEntityEquivalentVec<T> {
    fn eq(&self, other: &&mut [U]) -> bool {
        self.0.eq(other)
    }
}

impl<T: EntityEquivalent + PartialEq<U>, U: EntityEquivalent>
    PartialEq<&mut UniqueEntityEquivalentSlice<U>> for UniqueEntityEquivalentVec<T>
{
    fn eq(&self, other: &&mut UniqueEntityEquivalentSlice<U>) -> bool {
        self.0.eq(other)
    }
}

impl<T: EntityEquivalent + PartialEq<U>, U, const N: usize> PartialEq<&[U; N]>
    for UniqueEntityEquivalentVec<T>
{
    fn eq(&self, other: &&[U; N]) -> bool {
        self.0.eq(other)
    }
}

impl<T: EntityEquivalent + PartialEq<U>, U: EntityEquivalent, const N: usize>
    PartialEq<&UniqueEntityEquivalentArray<U, N>> for UniqueEntityEquivalentVec<T>
{
    fn eq(&self, other: &&UniqueEntityEquivalentArray<U, N>) -> bool {
        self.0.eq(&other.as_inner())
    }
}

impl<T: EntityEquivalent + PartialEq<U>, U, const N: usize> PartialEq<&mut [U; N]>
    for UniqueEntityEquivalentVec<T>
{
    fn eq(&self, other: &&mut [U; N]) -> bool {
        self.0.eq(&**other)
    }
}

impl<T: EntityEquivalent + PartialEq<U>, U: EntityEquivalent, const N: usize>
    PartialEq<&mut UniqueEntityEquivalentArray<U, N>> for UniqueEntityEquivalentVec<T>
{
    fn eq(&self, other: &&mut UniqueEntityEquivalentArray<U, N>) -> bool {
        self.0.eq(other.as_inner())
    }
}

impl<T: EntityEquivalent + PartialEq<U>, U> PartialEq<[U]> for UniqueEntityEquivalentVec<T> {
    fn eq(&self, other: &[U]) -> bool {
        self.0.eq(other)
    }
}

impl<T: EntityEquivalent + PartialEq<U>, U: EntityEquivalent>
    PartialEq<UniqueEntityEquivalentSlice<U>> for UniqueEntityEquivalentVec<T>
{
    fn eq(&self, other: &UniqueEntityEquivalentSlice<U>) -> bool {
        self.0.eq(&**other)
    }
}

impl<T: EntityEquivalent + PartialEq<U>, U, const N: usize> PartialEq<[U; N]>
    for UniqueEntityEquivalentVec<T>
{
    fn eq(&self, other: &[U; N]) -> bool {
        self.0.eq(other)
    }
}

impl<T: EntityEquivalent + PartialEq<U>, U: EntityEquivalent, const N: usize>
    PartialEq<UniqueEntityEquivalentArray<U, N>> for UniqueEntityEquivalentVec<T>
{
    fn eq(&self, other: &UniqueEntityEquivalentArray<U, N>) -> bool {
        self.0.eq(other.as_inner())
    }
}

impl<T: PartialEq<U>, U: EntityEquivalent> PartialEq<UniqueEntityEquivalentVec<U>> for Vec<T> {
    fn eq(&self, other: &UniqueEntityEquivalentVec<U>) -> bool {
        self.eq(&other.0)
    }
}

impl<T: PartialEq<U>, U: EntityEquivalent> PartialEq<UniqueEntityEquivalentVec<U>> for &[T] {
    fn eq(&self, other: &UniqueEntityEquivalentVec<U>) -> bool {
        self.eq(&other.0)
    }
}

impl<T: PartialEq<U>, U: EntityEquivalent> PartialEq<UniqueEntityEquivalentVec<U>> for &mut [T] {
    fn eq(&self, other: &UniqueEntityEquivalentVec<U>) -> bool {
        self.eq(&other.0)
    }
}

impl<T: EntityEquivalent + PartialEq<U>, U: EntityEquivalent>
    PartialEq<UniqueEntityEquivalentVec<U>> for [T]
{
    fn eq(&self, other: &UniqueEntityEquivalentVec<U>) -> bool {
        self.eq(&other.0)
    }
}

impl<T: PartialEq<U> + Clone, U: EntityEquivalent> PartialEq<UniqueEntityEquivalentVec<U>>
    for Cow<'_, [T]>
{
    fn eq(&self, other: &UniqueEntityEquivalentVec<U>) -> bool {
        self.eq(&other.0)
    }
}

impl<T: PartialEq<U>, U: EntityEquivalent> PartialEq<UniqueEntityEquivalentVec<U>> for VecDeque<T> {
    fn eq(&self, other: &UniqueEntityEquivalentVec<U>) -> bool {
        self.eq(&other.0)
    }
}

impl<T: EntityEquivalent + Clone> From<&UniqueEntityEquivalentSlice<T>>
    for UniqueEntityEquivalentVec<T>
{
    fn from(value: &UniqueEntityEquivalentSlice<T>) -> Self {
        value.to_vec()
    }
}

impl<T: EntityEquivalent + Clone> From<&mut UniqueEntityEquivalentSlice<T>>
    for UniqueEntityEquivalentVec<T>
{
    fn from(value: &mut UniqueEntityEquivalentSlice<T>) -> Self {
        value.to_vec()
    }
}

impl<T: EntityEquivalent> From<Box<UniqueEntityEquivalentSlice<T>>>
    for UniqueEntityEquivalentVec<T>
{
    fn from(value: Box<UniqueEntityEquivalentSlice<T>>) -> Self {
        value.into_vec()
    }
}

impl<T: EntityEquivalent> From<Cow<'_, UniqueEntityEquivalentSlice<T>>>
    for UniqueEntityEquivalentVec<T>
where
    UniqueEntityEquivalentSlice<T>: ToOwned<Owned = UniqueEntityEquivalentVec<T>>,
{
    fn from(value: Cow<UniqueEntityEquivalentSlice<T>>) -> Self {
        value.into_owned()
    }
}

impl<T: EntityEquivalent + Clone> From<&[T; 1]> for UniqueEntityEquivalentVec<T> {
    fn from(value: &[T; 1]) -> Self {
        Self(Vec::from(value))
    }
}

impl<T: EntityEquivalent + Clone> From<&[T; 0]> for UniqueEntityEquivalentVec<T> {
    fn from(value: &[T; 0]) -> Self {
        Self(Vec::from(value))
    }
}

impl<T: EntityEquivalent + Clone> From<&mut [T; 1]> for UniqueEntityEquivalentVec<T> {
    fn from(value: &mut [T; 1]) -> Self {
        Self(Vec::from(value))
    }
}

impl<T: EntityEquivalent + Clone> From<&mut [T; 0]> for UniqueEntityEquivalentVec<T> {
    fn from(value: &mut [T; 0]) -> Self {
        Self(Vec::from(value))
    }
}

impl<T: EntityEquivalent> From<[T; 1]> for UniqueEntityEquivalentVec<T> {
    fn from(value: [T; 1]) -> Self {
        Self(Vec::from(value))
    }
}

impl<T: EntityEquivalent> From<[T; 0]> for UniqueEntityEquivalentVec<T> {
    fn from(value: [T; 0]) -> Self {
        Self(Vec::from(value))
    }
}

impl<T: EntityEquivalent + Clone, const N: usize> From<&UniqueEntityEquivalentArray<T, N>>
    for UniqueEntityEquivalentVec<T>
{
    fn from(value: &UniqueEntityEquivalentArray<T, N>) -> Self {
        Self(Vec::from(value.as_inner().clone()))
    }
}

impl<T: EntityEquivalent + Clone, const N: usize> From<&mut UniqueEntityEquivalentArray<T, N>>
    for UniqueEntityEquivalentVec<T>
{
    fn from(value: &mut UniqueEntityEquivalentArray<T, N>) -> Self {
        Self(Vec::from(value.as_inner().clone()))
    }
}

impl<T: EntityEquivalent, const N: usize> From<UniqueEntityEquivalentArray<T, N>>
    for UniqueEntityEquivalentVec<T>
{
    fn from(value: UniqueEntityEquivalentArray<T, N>) -> Self {
        Self(Vec::from(value.into_inner()))
    }
}

impl<T: EntityEquivalent> From<UniqueEntityEquivalentVec<T>> for Vec<T> {
    fn from(value: UniqueEntityEquivalentVec<T>) -> Self {
        value.0
    }
}

impl<'a, T: EntityEquivalent + Clone> From<UniqueEntityEquivalentVec<T>> for Cow<'a, [T]> {
    fn from(value: UniqueEntityEquivalentVec<T>) -> Self {
        Cow::from(value.0)
    }
}

impl<'a, T: EntityEquivalent + Clone> From<UniqueEntityEquivalentVec<T>>
    for Cow<'a, UniqueEntityEquivalentSlice<T>>
{
    fn from(value: UniqueEntityEquivalentVec<T>) -> Self {
        Cow::Owned(value)
    }
}

impl<T: EntityEquivalent> From<UniqueEntityEquivalentVec<T>> for Arc<[T]> {
    fn from(value: UniqueEntityEquivalentVec<T>) -> Self {
        Arc::from(value.0)
    }
}

impl<T: EntityEquivalent> From<UniqueEntityEquivalentVec<T>>
    for Arc<UniqueEntityEquivalentSlice<T>>
{
    fn from(value: UniqueEntityEquivalentVec<T>) -> Self {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntityEquivalentSlice::from_arc_slice_unchecked(Arc::from(value.0)) }
    }
}

impl<T: EntityEquivalent + Ord> From<UniqueEntityEquivalentVec<T>> for BinaryHeap<T> {
    fn from(value: UniqueEntityEquivalentVec<T>) -> Self {
        BinaryHeap::from(value.0)
    }
}

impl<T: EntityEquivalent> From<UniqueEntityEquivalentVec<T>> for Box<[T]> {
    fn from(value: UniqueEntityEquivalentVec<T>) -> Self {
        Box::from(value.0)
    }
}

impl<T: EntityEquivalent> From<UniqueEntityEquivalentVec<T>> for Rc<[T]> {
    fn from(value: UniqueEntityEquivalentVec<T>) -> Self {
        Rc::from(value.0)
    }
}

impl<T: EntityEquivalent> From<UniqueEntityEquivalentVec<T>>
    for Rc<UniqueEntityEquivalentSlice<T>>
{
    fn from(value: UniqueEntityEquivalentVec<T>) -> Self {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntityEquivalentSlice::from_rc_slice_unchecked(Rc::from(value.0)) }
    }
}

impl<T: EntityEquivalent> From<UniqueEntityEquivalentVec<T>> for VecDeque<T> {
    fn from(value: UniqueEntityEquivalentVec<T>) -> Self {
        VecDeque::from(value.0)
    }
}

impl<T: EntityEquivalent, const N: usize> TryFrom<UniqueEntityEquivalentVec<T>> for Box<[T; N]> {
    type Error = UniqueEntityEquivalentVec<T>;

    fn try_from(value: UniqueEntityEquivalentVec<T>) -> Result<Self, Self::Error> {
        Box::try_from(value.0).map_err(UniqueEntityEquivalentVec)
    }
}

impl<T: EntityEquivalent, const N: usize> TryFrom<UniqueEntityEquivalentVec<T>>
    for Box<UniqueEntityEquivalentArray<T, N>>
{
    type Error = UniqueEntityEquivalentVec<T>;

    fn try_from(value: UniqueEntityEquivalentVec<T>) -> Result<Self, Self::Error> {
        Box::try_from(value.0)
            .map(|v|
                // SAFETY: All elements in the original Vec are unique.
                unsafe { UniqueEntityEquivalentArray::from_boxed_array_unchecked(v) })
            .map_err(UniqueEntityEquivalentVec)
    }
}

impl<T: EntityEquivalent, const N: usize> TryFrom<UniqueEntityEquivalentVec<T>> for [T; N] {
    type Error = UniqueEntityEquivalentVec<T>;

    fn try_from(value: UniqueEntityEquivalentVec<T>) -> Result<Self, Self::Error> {
        <[T; N] as TryFrom<Vec<T>>>::try_from(value.0).map_err(UniqueEntityEquivalentVec)
    }
}

impl<T: EntityEquivalent, const N: usize> TryFrom<UniqueEntityEquivalentVec<T>>
    for UniqueEntityEquivalentArray<T, N>
{
    type Error = UniqueEntityEquivalentVec<T>;

    fn try_from(value: UniqueEntityEquivalentVec<T>) -> Result<Self, Self::Error> {
        <[T; N] as TryFrom<Vec<T>>>::try_from(value.0)
            .map(|v|
            // SAFETY: All elements in the original Vec are unique.
            unsafe { UniqueEntityEquivalentArray::from_array_unchecked(v) })
            .map_err(UniqueEntityEquivalentVec)
    }
}

impl<T: EntityEquivalent> From<BTreeSet<T>> for UniqueEntityEquivalentVec<T> {
    fn from(value: BTreeSet<T>) -> Self {
        Self(value.into_iter().collect::<Vec<T>>())
    }
}

impl<T: EntityEquivalent> FromIterator<T> for UniqueEntityEquivalentVec<T> {
    /// This impl only uses `Eq` to validate uniqueness, resulting in O(n^2) complexity.
    /// It can make sense for very low N, or if `T` implements neither `Ord` nor `Hash`.
    /// When possible, use `FromEntitySetIterator::from_entity_iter` instead.
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        // Matches the `HashSet::from_iter` reservation logic.
        let iter = iter.into_iter();
        let unique_vec = Self::with_capacity(iter.size_hint().0);
        // Internal iteration (fold/for_each) is known to result in better code generation
        // over a for loop.
        iter.fold(unique_vec, |mut unique_vec, item| {
            if !unique_vec.0.contains(&item) {
                unique_vec.0.push(item);
            }
            unique_vec
        })
    }
}

impl<T: EntityEquivalent> FromEntitySetIterator<T> for UniqueEntityEquivalentVec<T> {
    fn from_entity_set_iter<I: EntitySet<Item = T>>(iter: I) -> Self {
        // SAFETY: `iter` is an `EntitySet`.
        unsafe { Self::from_vec_unchecked(Vec::from_iter(iter)) }
    }
}

impl<T: EntityEquivalent> Extend<T> for UniqueEntityEquivalentVec<T> {
    /// Use with caution, because this impl only uses `Eq` to validate uniqueness,
    /// resulting in O(n^2) complexity.
    /// It can make sense for very low N, or if `T` implements neither `Ord` nor `Hash`.
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        // Matches the `HashSet::extend` reservation logic. Their reasoning:
        //  "Keys may be already present or show multiple times in the iterator.
        //  Reserve the entire hint lower bound if the map is empty.
        //  Otherwise reserve half the hint (rounded up), so the map
        //  will only resize twice in the worst case."
        let iter = iter.into_iter();
        let reserve = if self.is_empty() {
            iter.size_hint().0
        } else {
            iter.size_hint().0.div_ceil(2)
        };
        self.reserve(reserve);
        // Internal iteration (fold/for_each) is known to result in better code generation
        // over a for loop.
        iter.for_each(move |item| {
            if !self.0.contains(&item) {
                self.0.push(item);
            }
        });
    }
}

impl<'a, T: EntityEquivalent + Copy + 'a> Extend<&'a T> for UniqueEntityEquivalentVec<T> {
    /// Use with caution, because this impl only uses `Eq` to validate uniqueness,
    /// resulting in O(n^2) complexity.
    /// It can make sense for very low N, or if `T` implements neither `Ord` nor `Hash`.
    fn extend<I: IntoIterator<Item = &'a T>>(&mut self, iter: I) {
        // Matches the `HashSet::extend` reservation logic. Their reasoning:
        //  "Keys may be already present or show multiple times in the iterator.
        //  Reserve the entire hint lower bound if the map is empty.
        //  Otherwise reserve half the hint (rounded up), so the map
        //  will only resize twice in the worst case."
        let iter = iter.into_iter();
        let reserve = if self.is_empty() {
            iter.size_hint().0
        } else {
            iter.size_hint().0.div_ceil(2)
        };
        self.reserve(reserve);
        // Internal iteration (fold/for_each) is known to result in better code generation
        // over a for loop.
        iter.for_each(move |item| {
            if !self.0.contains(item) {
                self.0.push(*item);
            }
        });
    }
}

impl<T: EntityEquivalent> Index<(Bound<usize>, Bound<usize>)> for UniqueEntityEquivalentVec<T> {
    type Output = UniqueEntityEquivalentSlice<T>;
    fn index(&self, key: (Bound<usize>, Bound<usize>)) -> &Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntityEquivalentSlice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<T: EntityEquivalent> Index<Range<usize>> for UniqueEntityEquivalentVec<T> {
    type Output = UniqueEntityEquivalentSlice<T>;
    fn index(&self, key: Range<usize>) -> &Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntityEquivalentSlice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<T: EntityEquivalent> Index<RangeFrom<usize>> for UniqueEntityEquivalentVec<T> {
    type Output = UniqueEntityEquivalentSlice<T>;
    fn index(&self, key: RangeFrom<usize>) -> &Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntityEquivalentSlice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<T: EntityEquivalent> Index<RangeFull> for UniqueEntityEquivalentVec<T> {
    type Output = UniqueEntityEquivalentSlice<T>;
    fn index(&self, key: RangeFull) -> &Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntityEquivalentSlice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<T: EntityEquivalent> Index<RangeInclusive<usize>> for UniqueEntityEquivalentVec<T> {
    type Output = UniqueEntityEquivalentSlice<T>;
    fn index(&self, key: RangeInclusive<usize>) -> &Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntityEquivalentSlice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<T: EntityEquivalent> Index<RangeTo<usize>> for UniqueEntityEquivalentVec<T> {
    type Output = UniqueEntityEquivalentSlice<T>;
    fn index(&self, key: RangeTo<usize>) -> &Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntityEquivalentSlice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<T: EntityEquivalent> Index<RangeToInclusive<usize>> for UniqueEntityEquivalentVec<T> {
    type Output = UniqueEntityEquivalentSlice<T>;
    fn index(&self, key: RangeToInclusive<usize>) -> &Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntityEquivalentSlice::from_slice_unchecked(self.0.index(key)) }
    }
}

impl<T: EntityEquivalent> Index<usize> for UniqueEntityEquivalentVec<T> {
    type Output = T;
    fn index(&self, key: usize) -> &T {
        self.0.index(key)
    }
}

impl<T: EntityEquivalent> IndexMut<(Bound<usize>, Bound<usize>)> for UniqueEntityEquivalentVec<T> {
    fn index_mut(&mut self, key: (Bound<usize>, Bound<usize>)) -> &mut Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntityEquivalentSlice::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

impl<T: EntityEquivalent> IndexMut<Range<usize>> for UniqueEntityEquivalentVec<T> {
    fn index_mut(&mut self, key: Range<usize>) -> &mut Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntityEquivalentSlice::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

impl<T: EntityEquivalent> IndexMut<RangeFrom<usize>> for UniqueEntityEquivalentVec<T> {
    fn index_mut(&mut self, key: RangeFrom<usize>) -> &mut Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntityEquivalentSlice::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

impl<T: EntityEquivalent> IndexMut<RangeFull> for UniqueEntityEquivalentVec<T> {
    fn index_mut(&mut self, key: RangeFull) -> &mut Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntityEquivalentSlice::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

impl<T: EntityEquivalent> IndexMut<RangeInclusive<usize>> for UniqueEntityEquivalentVec<T> {
    fn index_mut(&mut self, key: RangeInclusive<usize>) -> &mut Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntityEquivalentSlice::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

impl<T: EntityEquivalent> IndexMut<RangeTo<usize>> for UniqueEntityEquivalentVec<T> {
    fn index_mut(&mut self, key: RangeTo<usize>) -> &mut Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntityEquivalentSlice::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

impl<T: EntityEquivalent> IndexMut<RangeToInclusive<usize>> for UniqueEntityEquivalentVec<T> {
    fn index_mut(&mut self, key: RangeToInclusive<usize>) -> &mut Self::Output {
        // SAFETY: All elements in the original slice are unique.
        unsafe { UniqueEntityEquivalentSlice::from_slice_unchecked_mut(self.0.index_mut(key)) }
    }
}

/// An iterator that moves out of a vector.
///
/// This `struct` is created by the [`IntoIterator::into_iter`] trait
/// method on [`UniqueEntityEquivalentVec`].
pub type IntoIter<T = Entity> = UniqueEntityIter<vec::IntoIter<T>>;

/// A draining iterator for [`UniqueEntityEquivalentVec<T>`].
/// See its documentation for more.
pub type Drain<'a, T = Entity> = UniqueEntityIter<vec::Drain<'a, T>>;

/// A splicing iterator for [`UniqueEntityEquivalentVec`].
/// See its documentation for more.
pub type Splice<'a, I> = UniqueEntityIter<vec::Splice<'a, I>>;
