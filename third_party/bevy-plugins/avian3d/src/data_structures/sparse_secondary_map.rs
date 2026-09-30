//! A map for associating data with previously stored entities in a generational arena.
//!
//! This is an adaptation of [`slotmap::SparseSecondaryMap`], tailored for Avian.
//! Some modifications include:
//!
//! - The key is always an [`Entity`] instead of a generic key type.
//! - Much more minimalistic. No entry API or iterators.
//! - `no_std` compatible.
//!
//! [`slotmap::SparseSecondaryMap`]: https://docs.rs/slotmap/1.0.7/slotmap/struct.SparseSecondaryMap.html

use bevy::platform::hash::RandomState;
use std::collections::hash_map::{self, HashMap};
use std::hash;

use bevy::ecs::entity::{Entity, EntityGeneration};

#[derive(Debug, Clone)]
struct Slot<T> {
    generation: EntityGeneration,
    value: T,
}

/// Sparse secondary map for associating data with previously stored entities
/// in a generational arena.
#[derive(Debug, Clone)]
pub struct SparseSecondaryEntityMap<V, S: hash::BuildHasher = RandomState> {
    slots: HashMap<u32, Slot<V>, S>,
}

/// Returns if a is an older generation than b, taking into account wrapping of
/// generations.
fn is_older_generation(a: u32, b: u32) -> bool {
    let diff = a.wrapping_sub(b);
    diff >= (1 << 31)
}

impl<V, S: hash::BuildHasher> SparseSecondaryEntityMap<V, S> {
    /// Creates an empty [`SparseSecondaryEntityMap`] which will use the given hash
    /// builder to hash keys.
    ///
    /// The secondary map will not reallocate until it holds at least `capacity`
    /// slots.
    #[inline]
    pub fn with_hasher(hash_builder: S) -> Self {
        Self {
            slots: HashMap::with_hasher(hash_builder),
        }
    }

    /// Inserts a value into the secondary map at the given `entity`.
    ///
    /// Returns [`None`] if this entity was not present in the map,
    /// and the old value otherwise.
    #[inline]
    pub fn insert(&mut self, entity: Entity, value: V) -> Option<V> {
        if entity == Entity::PLACEHOLDER {
            return None;
        }

        let (index, generation) = (entity.index_u32(), entity.generation());

        if let Some(slot) = self.slots.get_mut(&index) {
            if slot.generation == generation {
                return Some(core::mem::replace(&mut slot.value, value));
            }

            // Don't replace existing newer values.
            if unsafe {
                is_older_generation(
                    core::mem::transmute::<EntityGeneration, u32>(generation),
                    core::mem::transmute::<EntityGeneration, u32>(slot.generation),
                )
            } {
                return None;
            }

            *slot = Slot { generation, value };

            return None;
        }

        self.slots.insert(index, Slot { generation, value });

        None
    }

    /// Removes a entity from the secondary map, returning the value at the entity if
    /// the entity was not previously removed.
    #[inline]
    pub fn remove(&mut self, entity: Entity) -> Option<V> {
        if let hash_map::Entry::Occupied(entry) = self.slots.entry(entity.index_u32())
            && entry.get().generation == entity.generation()
        {
            return Some(entry.remove_entry().1.value);
        }

        None
    }

    /// Returns a reference to the value corresponding to the entity.
    #[inline]
    pub fn get(&self, entity: Entity) -> Option<&V> {
        self.slots
            .get(&entity.index_u32())
            .filter(|slot| slot.generation == entity.generation())
            .map(|slot| &slot.value)
    }

    /// Returns a mutable reference to the value corresponding to the entity.
    #[inline]
    pub fn get_mut(&mut self, entity: Entity) -> Option<&mut V> {
        self.slots
            .get_mut(&entity.index_u32())
            .filter(|slot| slot.generation == entity.generation())
            .map(|slot| &mut slot.value)
    }

    /// Returns the value corresponding to the entity if it exists, otherwise inserts
    /// the value returned by `f` and returns it.
    #[inline]
    pub fn get_or_insert_with<F>(&mut self, entity: Entity, f: F) -> V
    where
        F: FnOnce() -> V,
        V: Clone + Copy,
    {
        if let Some(slot) = self
            .slots
            .get(&entity.index_u32())
            .filter(|s| s.generation == entity.generation())
        {
            slot.value
        } else {
            let value = f();
            self.insert(entity, value);
            value
        }
    }
}

impl<V, S> Default for SparseSecondaryEntityMap<V, S>
where
    S: hash::BuildHasher + Default,
{
    fn default() -> Self {
        Self::with_hasher(Default::default())
    }
}
