//! An [`IdPool`] for managing and reusing identifiers.

use alloc::collections::BinaryHeap;
use core::cmp::Reverse;

/// A pool for efficient allocation and reuse of `u32` identifiers.
///
/// Freed IDs are stored in a min-heap, and reused such that the lowest available IDs are allocated first.
#[derive(Clone, Debug, Default)]
pub struct IdPool {
    /// A min-heap of free IDs. The lowest free IDs are allocated first.
    free_ids: BinaryHeap<Reverse<u32>>,
    /// The next ID to be allocated. Only incremented when no free IDs are available.
    next_index: u32,
}

impl IdPool {
    /// Creates a new empty [`IdPool`].
    #[inline(always)]
    pub const fn new() -> Self {
        Self {
            free_ids: BinaryHeap::new(),
            next_index: 0,
        }
    }

    /// Allocates a new ID.
    ///
    /// If there are free IDs available, the lowest free ID is reused.
    #[inline(always)]
    pub fn alloc(&mut self) -> u32 {
        if let Some(id) = self.free_ids.pop() {
            id.0
        } else {
            let id = self.next_index;
            self.next_index += 1;
            id
        }
    }

    /// Frees an ID, making it available for reuse.
    ///
    /// The ID is assumed to not already be freed.
    #[inline(always)]
    pub fn free(&mut self, id: u32) {
        debug_assert!(id < self.next_index);
        self.free_ids.push(Reverse(id));
    }

    /// Clears the pool, removing all free IDs and resetting the next index.
    #[inline(always)]
    pub fn clear(&mut self) {
        self.free_ids.clear();
        self.next_index = 0;
    }
    /// Returns the next ID that will be allocated.
    #[inline(always)]
    pub fn next_id(&self) -> u32 {
        if let Some(id) = self.free_ids.peek() {
            id.0
        } else {
            self.next_index
        }
    }

    /// Returns the number of allocated IDs.
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.next_index as usize - self.free_ids.len()
    }
}
