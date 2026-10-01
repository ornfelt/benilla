use crate::{
    archetype::{Archetype, ArchetypeGeneration, ArchetypeId},
    change_detection::Tick,
    component::ComponentId,
    entity::{Entity, EntityEquivalent, EntitySet},
    entity_disabling::DefaultQueryFilters,
    prelude::FromWorld,
    query::{FilteredAccess, QueryCombinationIter, QueryIter, QueryParIter, WorldQuery},
    storage::{SparseSetIndex, TableId},
    system::Query,
    world::{unsafe_world_cell::UnsafeWorldCell, World, WorldId},
};

use alloc::vec::Vec;
use core::{fmt, ptr};
use fixedbitset::FixedBitSet;
#[cfg(feature = "trace")]
use tracing::Span;

use super::{
    NopWorldQuery, QueryData, QueryEntityError, QueryFilter, QueryManyIter, QueryManyUniqueIter,
    QuerySingleError, ROQueryItem, ReadOnlyQueryData,
};

/// An ID for either a table or an archetype. Used for Query iteration.
///
/// Query iteration is exclusively dense (over tables) or archetypal (over archetypes) based on whether
/// the query filters are dense or not. This is represented by the [`QueryState::is_dense`] field.
///
/// Note that `D::IS_DENSE` and `F::IS_DENSE` have no relationship with `QueryState::is_dense` and
/// any combination of their values can happen.
///
/// This is a union instead of an enum as the usage is determined at compile time, as all [`StorageId`]s for
/// a [`QueryState`] will be all [`TableId`]s or all [`ArchetypeId`]s, and not a mixture of both. This
/// removes the need for discriminator to minimize memory usage and branching during iteration, but requires
/// a safety invariant be verified when disambiguating them.
///
/// # Safety
/// Must be initialized and accessed as a [`TableId`], if both generic parameters to the query are dense.
/// Must be initialized and accessed as an [`ArchetypeId`] otherwise.
#[derive(Clone, Copy)]
pub(super) union StorageId {
    pub(super) table_id: TableId,
    pub(super) archetype_id: ArchetypeId,
}

/// Provides scoped access to a [`World`] state according to a given [`QueryData`] and [`QueryFilter`].
///
/// This data is cached between system runs, and is used to:
/// - store metadata about which [`Table`] or [`Archetype`] are matched by the query. "Matched" means
///   that the query will iterate over the data in the matched table/archetype.
/// - cache the [`State`] needed to compute the [`Fetch`] struct used to retrieve data
///   from a specific [`Table`] or [`Archetype`]
/// - build iterators that can iterate over the query results
///
/// [`State`]: crate::query::world_query::WorldQuery::State
/// [`Fetch`]: crate::query::world_query::WorldQuery::Fetch
/// [`Table`]: crate::storage::Table
#[repr(C)]
// SAFETY NOTE:
// Do not add any new fields that use the `D` or `F` generic parameters as this may
// make `QueryState::as_transmuted_state` unsound if not done with care.
pub struct QueryState<D: QueryData, F: QueryFilter = ()> {
    world_id: WorldId,
    pub(crate) archetype_generation: ArchetypeGeneration,
    /// Metadata about the [`Table`](crate::storage::Table)s matched by this query.
    pub(crate) matched_tables: FixedBitSet,
    /// Metadata about the [`Archetype`]s matched by this query.
    pub(crate) matched_archetypes: FixedBitSet,
    /// [`FilteredAccess`] computed by combining the `D` and `F` access. Used to check which other queries
    /// this query can run in parallel with.
    /// Note that because we do a zero-cost reference conversion in `Query::as_readonly`,
    /// the access for a read-only query may include accesses for the original mutable version,
    /// but the `Query` does not have exclusive access to those components.
    pub(crate) component_access: FilteredAccess,
    // NOTE: we maintain both a bitset and a vec because iterating the vec is faster
    pub(super) matched_storage_ids: Vec<StorageId>,
    // Represents whether this query iteration is dense or not. When this is true
    // `matched_storage_ids` stores `TableId`s, otherwise it stores `ArchetypeId`s.
    pub(super) is_dense: bool,
    pub(crate) fetch_state: D::State,
    pub(crate) filter_state: F::State,
    #[cfg(feature = "trace")]
    par_iter_span: Span,
}

impl<D: QueryData, F: QueryFilter> fmt::Debug for QueryState<D, F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("QueryState")
            .field("world_id", &self.world_id)
            .field("matched_table_count", &self.matched_tables.count_ones(..))
            .field(
                "matched_archetype_count",
                &self.matched_archetypes.count_ones(..),
            )
            .finish_non_exhaustive()
    }
}

impl<D: QueryData, F: QueryFilter> FromWorld for QueryState<D, F> {
    fn from_world(world: &mut World) -> Self {
        world.query_filtered()
    }
}

impl<D: QueryData, F: QueryFilter> QueryState<D, F> {
    /// Converts this `QueryState` reference to a `QueryState` that does not access anything mutably.
    pub fn as_readonly(&self) -> &QueryState<D::ReadOnly, F> {
        // SAFETY: invariant on `WorldQuery` trait upholds that `D::ReadOnly` and `F::ReadOnly`
        // have a subset of the access, and match the exact same archetypes/tables as `D`/`F` respectively.
        unsafe { self.as_transmuted_state::<D::ReadOnly, F>() }
    }

    /// Converts this `QueryState` reference to a `QueryState` that does not return any data
    /// which can be faster.
    ///
    /// This doesn't use `NopWorldQuery` as it loses filter functionality, for example
    /// `NopWorldQuery<Changed<T>>` is functionally equivalent to `With<T>`.
    pub(crate) fn as_nop(&self) -> &QueryState<NopWorldQuery<D>, F> {
        // SAFETY: `NopWorldQuery` doesn't have any accesses and defers to
        // `D` for table/archetype matching
        unsafe { self.as_transmuted_state::<NopWorldQuery<D>, F>() }
    }

    /// Converts this `QueryState` reference to any other `QueryState` with
    /// the same `WorldQuery::State` associated types.
    ///
    /// Consider using `as_readonly` or `as_nop` instead which are safe functions.
    ///
    /// # Safety
    ///
    /// `NewD` must have a subset of the access that `D` does and match the exact same archetypes/tables
    /// `NewF` must have a subset of the access that `F` does and match the exact same archetypes/tables
    pub(crate) unsafe fn as_transmuted_state<
        NewD: ReadOnlyQueryData<State = D::State>,
        NewF: QueryFilter<State = F::State>,
    >(
        &self,
    ) -> &QueryState<NewD, NewF> {
        &*ptr::from_ref(self).cast::<QueryState<NewD, NewF>>()
    }

    /// Creates a new [`QueryState`] from a given [`World`] and inherits the result of `world.id()`.
    pub fn new(world: &mut World) -> Self {
        let mut state = Self::new_uninitialized(world);
        state.update_archetypes(world);
        state
    }

    /// Creates a new [`QueryState`] but does not populate it with the matched results from the World yet
    ///
    /// `new_archetype` and its variants must be called on all of the World's archetypes before the
    /// state can return valid query results.
    fn new_uninitialized(world: &mut World) -> Self {
        let fetch_state = D::init_state(world);
        let filter_state = F::init_state(world);
        Self::from_states_uninitialized(world, fetch_state, filter_state)
    }

    /// Creates a new [`QueryState`] but does not populate it with the matched results from the World yet
    ///
    /// `new_archetype` and its variants must be called on all of the World's archetypes before the
    /// state can return valid query results.
    fn from_states_uninitialized(
        world: &World,
        fetch_state: <D as WorldQuery>::State,
        filter_state: <F as WorldQuery>::State,
    ) -> Self {
        let mut component_access = FilteredAccess::default();
        D::update_component_access(&fetch_state, &mut component_access);

        // Use a temporary empty FilteredAccess for filters. This prevents them from conflicting with the
        // main Query's `fetch_state` access. Filters are allowed to conflict with the main query fetch
        // because they are evaluated *before* a specific reference is constructed.
        let mut filter_component_access = FilteredAccess::default();
        F::update_component_access(&filter_state, &mut filter_component_access);

        // Merge the temporary filter access with the main access. This ensures that filter access is
        // properly considered in a global "cross-query" context (both within systems and across systems).
        component_access.extend(&filter_component_access);

        // For queries without dynamic filters the dense-ness of the query is equal to the dense-ness
        // of its static type parameters.
        let mut is_dense = D::IS_DENSE && F::IS_DENSE;

        if let Some(default_filters) = world.get_resource::<DefaultQueryFilters>() {
            default_filters.modify_access(&mut component_access);
            is_dense &= default_filters.is_dense(world.components());
        }

        Self {
            world_id: world.id(),
            archetype_generation: ArchetypeGeneration::initial(),
            matched_storage_ids: Vec::new(),
            is_dense,
            fetch_state,
            filter_state,
            component_access,
            matched_tables: Default::default(),
            matched_archetypes: Default::default(),
            #[cfg(feature = "trace")]
            par_iter_span: tracing::info_span!(
                "par_for_each",
                query = core::any::type_name::<D>(),
                filter = core::any::type_name::<F>(),
            ),
        }
    }

    /// Creates a [`Query`] from the given [`QueryState`] and [`World`].
    ///
    /// This will create read-only queries, see [`Self::query_mut`] for mutable queries.
    pub fn query<'w, 's>(&'s mut self, world: &'w World) -> Query<'w, 's, D::ReadOnly, F> {
        self.update_archetypes(world);
        self.query_manual(world)
    }

    /// Creates a [`Query`] from the given [`QueryState`] and [`World`].
    ///
    /// This method is slightly more efficient than [`QueryState::query`] in some situations, since
    /// it does not update this instance's internal cache. The resulting query may skip an entity that
    /// belongs to an archetype that has not been cached.
    ///
    /// To ensure that the cache is up to date, call [`QueryState::update_archetypes`] before this method.
    /// The cache is also updated in [`QueryState::new`], [`QueryState::get`], or any method with mutable
    /// access to `self`.
    ///
    /// This will create read-only queries, see [`Self::query_mut`] for mutable queries.
    pub fn query_manual<'w, 's>(&'s self, world: &'w World) -> Query<'w, 's, D::ReadOnly, F> {
        self.validate_world(world.id());
        // SAFETY:
        // - We have read access to the entire world, and we call `as_readonly()` so the query only performs read access.
        // - We called `validate_world`.
        unsafe {
            self.as_readonly()
                .query_unchecked_manual(world.as_unsafe_world_cell_readonly())
        }
    }

    /// Creates a [`Query`] from the given [`QueryState`] and [`World`].
    pub fn query_mut<'w, 's>(&'s mut self, world: &'w mut World) -> Query<'w, 's, D, F> {
        let last_run = world.last_change_tick();
        let this_run = world.change_tick();
        // SAFETY: We have exclusive access to the entire world.
        unsafe { self.query_unchecked_with_ticks(world.as_unsafe_world_cell(), last_run, this_run) }
    }

    /// Creates a [`Query`] from the given [`QueryState`] and [`World`].
    ///
    /// It does not update this instance's internal cache. The resulting query may skip an entity that
    /// belongs to an archetype that has not been cached.
    ///
    /// To ensure that the cache is up to date, call [`QueryState::update_archetypes`] before this method.
    /// The cache is also updated in [`QueryState::new`], [`QueryState::get`], or any method with mutable
    /// access to `self`.
    ///
    /// # Safety
    ///
    /// This does not check for mutable query correctness. To be safe, make sure mutable queries
    /// have unique access to the components they query.
    /// This does not validate that `world.id()` matches `self.world_id`. Calling this on a `world`
    /// with a mismatched [`WorldId`] is unsound.
    pub unsafe fn query_unchecked_manual<'w, 's>(
        &'s self,
        world: UnsafeWorldCell<'w>,
    ) -> Query<'w, 's, D, F> {
        let last_run = world.last_change_tick();
        let this_run = world.change_tick();
        // SAFETY:
        // - The caller ensured we have the correct access to the world.
        // - The caller ensured that the world matches.
        unsafe { self.query_unchecked_manual_with_ticks(world, last_run, this_run) }
    }

    /// Creates a [`Query`] from the given [`QueryState`] and [`World`].
    ///
    /// # Safety
    ///
    /// This does not check for mutable query correctness. To be safe, make sure mutable queries
    /// have unique access to the components they query.
    pub unsafe fn query_unchecked_with_ticks<'w, 's>(
        &'s mut self,
        world: UnsafeWorldCell<'w>,
        last_run: Tick,
        this_run: Tick,
    ) -> Query<'w, 's, D, F> {
        self.update_archetypes_unsafe_world_cell(world);
        // SAFETY:
        // - The caller ensured we have the correct access to the world.
        // - We called `update_archetypes_unsafe_world_cell`, which calls `validate_world`.
        unsafe { self.query_unchecked_manual_with_ticks(world, last_run, this_run) }
    }

    /// Creates a [`Query`] from the given [`QueryState`] and [`World`].
    ///
    /// This method is slightly more efficient than [`QueryState::query_unchecked_with_ticks`] in some situations, since
    /// it does not update this instance's internal cache. The resulting query may skip an entity that
    /// belongs to an archetype that has not been cached.
    ///
    /// To ensure that the cache is up to date, call [`QueryState::update_archetypes`] before this method.
    /// The cache is also updated in [`QueryState::new`], [`QueryState::get`], or any method with mutable
    /// access to `self`.
    ///
    /// # Safety
    ///
    /// This does not check for mutable query correctness. To be safe, make sure mutable queries
    /// have unique access to the components they query.
    /// This does not validate that `world.id()` matches `self.world_id`. Calling this on a `world`
    /// with a mismatched [`WorldId`] is unsound.
    pub unsafe fn query_unchecked_manual_with_ticks<'w, 's>(
        &'s self,
        world: UnsafeWorldCell<'w>,
        last_run: Tick,
        this_run: Tick,
    ) -> Query<'w, 's, D, F> {
        // SAFETY:
        // - The caller ensured we have the correct access to the world.
        // - The caller ensured that the world matches.
        unsafe { Query::new(world, self, last_run, this_run) }
    }

    /// Updates the state's internal view of the [`World`]'s archetypes. If this is not called before querying data,
    /// the results may not accurately reflect what is in the `world`.
    ///
    /// This is only required if a `manual` method (such as [`Self::get_manual`]) is being called, and it only needs to
    /// be called if the `world` has been structurally mutated (i.e. added/removed a component or resource). Users using
    /// non-`manual` methods such as [`QueryState::get`] do not need to call this as it will be automatically called for them.
    ///
    /// If you have an [`UnsafeWorldCell`] instead of `&World`, consider using [`QueryState::update_archetypes_unsafe_world_cell`].
    ///
    /// # Panics
    ///
    /// If `world` does not match the one used to call `QueryState::new` for this instance.
    #[inline]
    pub fn update_archetypes(&mut self, world: &World) {
        self.update_archetypes_unsafe_world_cell(world.as_unsafe_world_cell_readonly());
    }

    /// Updates the state's internal view of the `world`'s archetypes. If this is not called before querying data,
    /// the results may not accurately reflect what is in the `world`.
    ///
    /// This is only required if a `manual` method (such as [`Self::get_manual`]) is being called, and it only needs to
    /// be called if the `world` has been structurally mutated (i.e. added/removed a component or resource). Users using
    /// non-`manual` methods such as [`QueryState::get`] do not need to call this as it will be automatically called for them.
    ///
    /// # Note
    ///
    /// This method only accesses world metadata.
    ///
    /// # Panics
    ///
    /// If `world` does not match the one used to call `QueryState::new` for this instance.
    pub fn update_archetypes_unsafe_world_cell(&mut self, world: UnsafeWorldCell) {
        self.validate_world(world.id());
        if self.component_access.required.is_empty() {
            let archetypes = world.archetypes();
            let old_generation =
                core::mem::replace(&mut self.archetype_generation, archetypes.generation());

            for archetype in &archetypes[old_generation..] {
                // SAFETY: The validate_world call ensures that the world is the same the QueryState
                // was initialized from.
                unsafe {
                    self.new_archetype(archetype);
                }
            }
        } else {
            // skip if we are already up to date
            if self.archetype_generation == world.archetypes().generation() {
                return;
            }
            // if there are required components, we can optimize by only iterating through archetypes
            // that contain at least one of the required components
            let potential_archetypes = self
                .component_access
                .required
                .ones()
                .filter_map(|idx| {
                    let component_id = ComponentId::get_sparse_set_index(idx);
                    world
                        .archetypes()
                        .component_index()
                        .get(&component_id)
                        .map(|index| index.keys())
                })
                // select the component with the fewest archetypes
                .min_by_key(ExactSizeIterator::len);
            if let Some(archetypes) = potential_archetypes {
                for archetype_id in archetypes {
                    // exclude archetypes that have already been processed
                    if archetype_id < &self.archetype_generation.0 {
                        continue;
                    }
                    // SAFETY: get_potential_archetypes only returns archetype ids that are valid for the world
                    let archetype = &world.archetypes()[*archetype_id];
                    // SAFETY: The validate_world call ensures that the world is the same the QueryState
                    // was initialized from.
                    unsafe {
                        self.new_archetype(archetype);
                    }
                }
            }
            self.archetype_generation = world.archetypes().generation();
        }
    }

    /// # Panics
    ///
    /// If `world_id` does not match the [`World`] used to call `QueryState::new` for this instance.
    ///
    /// Many unsafe query methods require the world to match for soundness. This function is the easiest
    /// way of ensuring that it matches.
    #[inline]
    #[track_caller]
    pub fn validate_world(&self, world_id: WorldId) {
        #[inline(never)]
        #[track_caller]
        #[cold]
        fn panic_mismatched(this: WorldId, other: WorldId) -> ! {
            panic!("Encountered a mismatched World. This QueryState was created from {this:?}, but a method was called using {other:?}.");
        }

        if self.world_id != world_id {
            panic_mismatched(self.world_id, world_id);
        }
    }

    /// Update the current [`QueryState`] with information from the provided [`Archetype`]
    /// (if applicable, i.e. if the archetype has any intersecting [`ComponentId`] with the current [`QueryState`]).
    ///
    /// # Safety
    /// `archetype` must be from the `World` this state was initialized from.
    pub unsafe fn new_archetype(&mut self, archetype: &Archetype) {
        if D::matches_component_set(&self.fetch_state, &|id| archetype.contains(id))
            && F::matches_component_set(&self.filter_state, &|id| archetype.contains(id))
            && self.matches_component_set(&|id| archetype.contains(id))
        {
            let archetype_index = archetype.id().index();
            if !self.matched_archetypes.contains(archetype_index) {
                self.matched_archetypes.grow_and_insert(archetype_index);
                if !self.is_dense {
                    self.matched_storage_ids.push(StorageId {
                        archetype_id: archetype.id(),
                    });
                }
            }
            let table_index = archetype.table_id().as_usize();
            if !self.matched_tables.contains(table_index) {
                self.matched_tables.grow_and_insert(table_index);
                if self.is_dense {
                    self.matched_storage_ids.push(StorageId {
                        table_id: archetype.table_id(),
                    });
                }
            }
        }
    }

    /// Returns `true` if this query matches a set of components. Otherwise, returns `false`.
    pub fn matches_component_set(&self, set_contains_id: &impl Fn(ComponentId) -> bool) -> bool {
        self.component_access.filter_sets.iter().any(|set| {
            set.with
                .ones()
                .all(|index| set_contains_id(ComponentId::get_sparse_set_index(index)))
                && set
                    .without
                    .ones()
                    .all(|index| !set_contains_id(ComponentId::get_sparse_set_index(index)))
        })
    }

    /// Gets the query result for the given [`World`] and [`Entity`].
    ///
    /// This can only be called for read-only queries, see [`Self::get_mut`] for write-queries.
    ///
    /// If you need to get multiple items at once but get borrowing errors,
    /// consider using [`Self::update_archetypes`] followed by multiple [`Self::get_manual`] calls,
    /// or making a single call with [`Self::get_many`]  or [`Self::iter_many`].
    ///
    /// This is always guaranteed to run in `O(1)` time.
    #[inline]
    pub fn get<'w>(
        &mut self,
        world: &'w World,
        entity: Entity,
    ) -> Result<ROQueryItem<'w, '_, D>, QueryEntityError> {
        self.query(world).get_inner(entity)
    }

    /// Returns the read-only query results for the given array of [`Entity`].
    ///
    /// In case of a nonexisting entity or mismatched component, a [`QueryEntityError`] is
    /// returned instead.
    ///
    /// Note that the unlike [`QueryState::get_many_mut`], the entities passed in do not need to be unique.
    ///
    /// # Examples
    ///
    /// ```
    /// use bevy_ecs::prelude::*;
    /// use bevy_ecs::query::QueryEntityError;
    ///
    /// #[derive(Component, PartialEq, Debug)]
    /// struct A(usize);
    ///
    /// let mut world = World::new();
    /// let entity_vec: Vec<Entity> = (0..3).map(|i|world.spawn(A(i)).id()).collect();
    /// let entities: [Entity; 3] = entity_vec.try_into().unwrap();
    ///
    /// world.spawn(A(73));
    ///
    /// let mut query_state = world.query::<&A>();
    ///
    /// let component_values = query_state.get_many(&world, entities).unwrap();
    ///
    /// assert_eq!(component_values, [&A(0), &A(1), &A(2)]);
    ///
    /// let wrong_entity = Entity::from_raw_u32(365).unwrap();
    ///
    /// assert_eq!(match query_state.get_many(&mut world, [wrong_entity]).unwrap_err() {QueryEntityError::NotSpawned(error) => error.entity(), _ => panic!()}, wrong_entity);
    /// ```
    #[inline]
    pub fn get_many<'w, const N: usize>(
        &mut self,
        world: &'w World,
        entities: [Entity; N],
    ) -> Result<[ROQueryItem<'w, '_, D>; N], QueryEntityError> {
        self.query(world).get_many_inner(entities)
    }

    /// Gets the query result for the given [`World`] and [`Entity`].
    ///
    /// This is always guaranteed to run in `O(1)` time.
    #[inline]
    pub fn get_mut<'w>(
        &mut self,
        world: &'w mut World,
        entity: Entity,
    ) -> Result<D::Item<'w, '_>, QueryEntityError> {
        self.query_mut(world).get_inner(entity)
    }

    /// Returns the query results for the given array of [`Entity`].
    ///
    /// In case of a nonexisting entity or mismatched component, a [`QueryEntityError`] is
    /// returned instead.
    ///
    /// ```
    /// use bevy_ecs::prelude::*;
    /// use bevy_ecs::query::QueryEntityError;
    ///
    /// #[derive(Component, PartialEq, Debug)]
    /// struct A(usize);
    ///
    /// let mut world = World::new();
    ///
    /// let entities: Vec<Entity> = (0..3).map(|i|world.spawn(A(i)).id()).collect();
    /// let entities: [Entity; 3] = entities.try_into().unwrap();
    ///
    /// world.spawn(A(73));
    ///
    /// let mut query_state = world.query::<&mut A>();
    ///
    /// let mut mutable_component_values = query_state.get_many_mut(&mut world, entities).unwrap();
    ///
    /// for mut a in &mut mutable_component_values {
    ///     a.0 += 5;
    /// }
    ///
    /// let component_values = query_state.get_many(&world, entities).unwrap();
    ///
    /// assert_eq!(component_values, [&A(5), &A(6), &A(7)]);
    ///
    /// let wrong_entity = Entity::from_raw_u32(57).unwrap();
    /// let invalid_entity = world.spawn_empty().id();
    ///
    /// assert_eq!(match query_state.get_many(&mut world, [wrong_entity]).unwrap_err() {QueryEntityError::NotSpawned(error) => error.entity(), _ => panic!()}, wrong_entity);
    /// assert_eq!(match query_state.get_many_mut(&mut world, [invalid_entity]).unwrap_err() {QueryEntityError::QueryDoesNotMatch(entity, _) => entity, _ => panic!()}, invalid_entity);
    /// assert_eq!(query_state.get_many_mut(&mut world, [entities[0], entities[0]]).unwrap_err(), QueryEntityError::AliasedMutability(entities[0]));
    /// ```
    #[inline]
    pub fn get_many_mut<'w, const N: usize>(
        &mut self,
        world: &'w mut World,
        entities: [Entity; N],
    ) -> Result<[D::Item<'w, '_>; N], QueryEntityError> {
        self.query_mut(world).get_many_mut_inner(entities)
    }

    /// Gets the query result for the given [`World`] and [`Entity`].
    ///
    /// This method is slightly more efficient than [`QueryState::get`] in some situations, since
    /// it does not update this instance's internal cache. This method will return an error if `entity`
    /// belongs to an archetype that has not been cached.
    ///
    /// To ensure that the cache is up to date, call [`QueryState::update_archetypes`] before this method.
    /// The cache is also updated in [`QueryState::new`], `QueryState::get`, or any method with mutable
    /// access to `self`.
    ///
    /// This can only be called for read-only queries, see [`Self::get_mut`] for mutable queries.
    ///
    /// This is always guaranteed to run in `O(1)` time.
    #[inline]
    pub fn get_manual<'w>(
        &self,
        world: &'w World,
        entity: Entity,
    ) -> Result<ROQueryItem<'w, '_, D>, QueryEntityError> {
        self.query_manual(world).get_inner(entity)
    }

    /// Returns an [`Iterator`] over the query results for the given [`World`].
    ///
    /// This can only be called for read-only queries, see [`Self::iter_mut`] for write-queries.
    ///
    /// If you need to iterate multiple times at once but get borrowing errors,
    /// consider using [`Self::update_archetypes`] followed by multiple [`Self::iter_manual`] calls.
    #[inline]
    pub fn iter<'w, 's>(&'s mut self, world: &'w World) -> QueryIter<'w, 's, D::ReadOnly, F> {
        self.query(world).into_iter()
    }

    /// Returns an [`Iterator`] over the query results for the given [`World`].
    ///
    /// This iterator is always guaranteed to return results from each matching entity once and only once.
    /// Iteration order is not guaranteed.
    #[inline]
    pub fn iter_mut<'w, 's>(&'s mut self, world: &'w mut World) -> QueryIter<'w, 's, D, F> {
        self.query_mut(world).into_iter()
    }

    /// Returns an [`Iterator`] over the query results for the given [`World`] without updating the query's archetypes.
    /// Archetypes must be manually updated before by using [`Self::update_archetypes`].
    ///
    /// This iterator is always guaranteed to return results from each matching entity once and only once.
    /// Iteration order is not guaranteed.
    ///
    /// This can only be called for read-only queries.
    #[inline]
    pub fn iter_manual<'w, 's>(&'s self, world: &'w World) -> QueryIter<'w, 's, D::ReadOnly, F> {
        self.query_manual(world).into_iter()
    }

    /// Returns an [`Iterator`] over all possible combinations of `K` query results without repetition.
    /// This can only be called for read-only queries.
    ///
    /// A combination is an arrangement of a collection of items where order does not matter.
    ///
    /// `K` is the number of items that make up each subset, and the number of items returned by the iterator.
    /// `N` is the number of total entities output by query.
    ///
    /// For example, given the list [1, 2, 3, 4], where `K` is 2, the combinations without repeats are
    /// [1, 2], [1, 3], [1, 4], [2, 3], [2, 4], [3, 4].
    /// And in this case, `N` would be defined as 4 since the size of the input list is 4.
    ///
    ///  For combinations of size `K` of query taking `N` inputs, you will get:
    /// - if `K == N`: one combination of all query results
    /// - if `K < N`: all possible `K`-sized combinations of query results, without repetition
    /// - if `K > N`: empty set (no `K`-sized combinations exist)
    ///
    /// The `iter_combinations` method does not guarantee order of iteration.
    ///
    /// This iterator is always guaranteed to return results from each unique pair of matching entities.
    /// Iteration order is not guaranteed.
    ///
    /// This can only be called for read-only queries, see [`Self::iter_combinations_mut`] for
    /// write-queries.
    #[inline]
    pub fn iter_combinations<'w, 's, const K: usize>(
        &'s mut self,
        world: &'w World,
    ) -> QueryCombinationIter<'w, 's, D::ReadOnly, F, K> {
        self.query(world).iter_combinations_inner()
    }

    /// Returns an [`Iterator`] over all possible combinations of `K` query results without repetition.
    ///
    /// A combination is an arrangement of a collection of items where order does not matter.
    ///
    /// `K` is the number of items that make up each subset, and the number of items returned by the iterator.
    /// `N` is the number of total entities output by query.
    ///
    /// For example, given the list [1, 2, 3, 4], where `K` is 2, the combinations without repeats are
    /// [1, 2], [1, 3], [1, 4], [2, 3], [2, 4], [3, 4].
    /// And in this case, `N` would be defined as 4 since the size of the input list is 4.
    ///
    ///  For combinations of size `K` of query taking `N` inputs, you will get:
    /// - if `K == N`: one combination of all query results
    /// - if `K < N`: all possible `K`-sized combinations of query results, without repetition
    /// - if `K > N`: empty set (no `K`-sized combinations exist)
    ///
    /// The `iter_combinations_mut` method does not guarantee order of iteration.
    #[inline]
    pub fn iter_combinations_mut<'w, 's, const K: usize>(
        &'s mut self,
        world: &'w mut World,
    ) -> QueryCombinationIter<'w, 's, D, F, K> {
        self.query_mut(world).iter_combinations_inner()
    }

    /// Returns an [`Iterator`] over the read-only query items generated from an [`Entity`] list.
    ///
    /// Items are returned in the order of the list of entities.
    /// Entities that don't match the query are skipped.
    ///
    /// # See also
    ///
    /// - [`iter_many_mut`](Self::iter_many_mut) to get mutable query items.
    #[inline]
    pub fn iter_many<'w, 's, EntityList: IntoIterator<Item: EntityEquivalent>>(
        &'s mut self,
        world: &'w World,
        entities: EntityList,
    ) -> QueryManyIter<'w, 's, D::ReadOnly, F, EntityList::IntoIter> {
        self.query(world).iter_many_inner(entities)
    }

    /// Returns an iterator over the query items generated from an [`Entity`] list.
    ///
    /// Items are returned in the order of the list of entities.
    /// Entities that don't match the query are skipped.
    #[inline]
    pub fn iter_many_mut<'w, 's, EntityList: IntoIterator<Item: EntityEquivalent>>(
        &'s mut self,
        world: &'w mut World,
        entities: EntityList,
    ) -> QueryManyIter<'w, 's, D, F, EntityList::IntoIter> {
        self.query_mut(world).iter_many_inner(entities)
    }

    /// Returns an iterator over the unique query items generated from an [`EntitySet`].
    ///
    /// Items are returned in the order of the list of entities.
    /// Entities that don't match the query are skipped.
    #[inline]
    pub fn iter_many_unique_mut<'w, 's, EntityList: EntitySet>(
        &'s mut self,
        world: &'w mut World,
        entities: EntityList,
    ) -> QueryManyUniqueIter<'w, 's, D, F, EntityList::IntoIter> {
        self.query_mut(world).iter_many_unique_inner(entities)
    }

    /// Returns a parallel iterator over the query results for the given [`World`].
    ///
    /// This can only be called for read-only queries.
    ///
    /// Note that you must use the `for_each` method to iterate over the
    /// results.
    #[inline]
    pub fn par_iter<'w, 's>(
        &'s mut self,
        world: &'w World,
    ) -> QueryParIter<'w, 's, D::ReadOnly, F> {
        self.query(world).par_iter_inner()
    }

    /// Runs `func` on each query result in parallel for the given [`World`], where the last change and
    /// the current change tick are given. This is faster than the equivalent
    /// `iter()` method, but cannot be chained like a normal [`Iterator`].
    ///
    /// # Panics
    /// The [`ComputeTaskPool`] is not initialized. If using this from a query that is being
    /// initialized and run from the ECS scheduler, this should never panic.
    ///
    /// # Safety
    ///
    /// This does not check for mutable query correctness. To be safe, make sure mutable queries
    /// have unique access to the components they query.
    /// This does not validate that `world.id()` matches `self.world_id`. Calling this on a `world`
    /// with a mismatched [`WorldId`] is unsound.
    ///
    /// [`ComputeTaskPool`]: bevy_tasks::ComputeTaskPool
    pub(crate) unsafe fn par_fold_init_unchecked_manual<'w, 's, T, FN, INIT>(
        &'s self,
        init_accum: INIT,
        world: UnsafeWorldCell<'w>,
        batch_size: u32,
        func: FN,
        last_run: Tick,
        this_run: Tick,
    ) where
        FN: Fn(T, D::Item<'w, 's>) -> T + Send + Sync + Clone,
        INIT: Fn() -> T + Sync + Send + Clone,
    {
        // NOTE: If you are changing query iteration code, remember to update the following places, where relevant:
        // QueryIter, QueryIterationCursor, QueryManyIter, QueryCombinationIter,QueryState::par_fold_init_unchecked_manual,
        // QueryState::par_many_fold_init_unchecked_manual, QueryState::par_many_unique_fold_init_unchecked_manual
        use arrayvec::ArrayVec;

        bevy_tasks::ComputeTaskPool::get().scope(|scope| {
            // SAFETY: We only access table data that has been registered in `self.component_access`.
            let tables = unsafe { &world.storages().tables };
            let archetypes = world.archetypes();
            let mut batch_queue = ArrayVec::new();
            let mut queue_entity_count = 0;

            // submit a list of storages which smaller than batch_size as single task
            let submit_batch_queue = |queue: &mut ArrayVec<StorageId, 128>| {
                if queue.is_empty() {
                    return;
                }
                let queue = core::mem::take(queue);
                let mut func = func.clone();
                let init_accum = init_accum.clone();
                scope.spawn(async move {
                    #[cfg(feature = "trace")]
                    let _span = self.par_iter_span.enter();
                    let mut iter = self
                        .query_unchecked_manual_with_ticks(world, last_run, this_run)
                        .into_iter();
                    let mut accum = init_accum();
                    for storage_id in queue {
                        accum = iter.fold_over_storage_range(accum, &mut func, storage_id, None);
                    }
                });
            };

            // submit single storage larger than batch_size
            let submit_single = |count, storage_id: StorageId| {
                for offset in (0..count).step_by(batch_size as usize) {
                    let mut func = func.clone();
                    let init_accum = init_accum.clone();
                    let len = batch_size.min(count - offset);
                    let batch = offset..offset + len;
                    scope.spawn(async move {
                        #[cfg(feature = "trace")]
                        let _span = self.par_iter_span.enter();
                        let accum = init_accum();
                        self.query_unchecked_manual_with_ticks(world, last_run, this_run)
                            .into_iter()
                            .fold_over_storage_range(accum, &mut func, storage_id, Some(batch));
                    });
                }
            };

            let storage_entity_count = |storage_id: StorageId| -> u32 {
                if self.is_dense {
                    tables[storage_id.table_id].entity_count()
                } else {
                    archetypes[storage_id.archetype_id].len()
                }
            };

            for storage_id in &self.matched_storage_ids {
                let count = storage_entity_count(*storage_id);

                // skip empty storage
                if count == 0 {
                    continue;
                }
                // immediately submit large storage
                if count >= batch_size {
                    submit_single(count, *storage_id);
                    continue;
                }
                // merge small storage
                batch_queue.push(*storage_id);
                queue_entity_count += count;

                // submit batch_queue
                if queue_entity_count >= batch_size || batch_queue.is_full() {
                    submit_batch_queue(&mut batch_queue);
                    queue_entity_count = 0;
                }
            }
            submit_batch_queue(&mut batch_queue);
        });
    }
}

impl<D: QueryData, F: QueryFilter> QueryState<D, F> {
    /// Returns a single immutable query result when there is exactly one entity matching
    /// the query.
    ///
    /// This can only be called for read-only queries,
    /// see [`single_mut`](Self::single_mut) for write-queries.
    ///
    /// If the number of query results is not exactly one, a [`QuerySingleError`] is returned
    /// instead.
    ///
    /// # Example
    ///
    /// Sometimes, you might want to handle the error in a specific way,
    /// generally by spawning the missing entity.
    ///
    /// ```rust
    /// use bevy_ecs::prelude::*;
    /// use bevy_ecs::query::QuerySingleError;
    ///
    /// #[derive(Component)]
    /// struct A(usize);
    ///
    /// fn my_system(query: Query<&A>, mut commands: Commands) {
    ///     match query.single() {
    ///         Ok(a) => (), // Do something with `a`
    ///         Err(err) => match err {
    ///             QuerySingleError::NoEntities(_) => {
    ///                 commands.spawn(A(0));
    ///             }
    ///             QuerySingleError::MultipleEntities(_) => panic!("Multiple entities found!"),
    ///         },
    ///     }
    /// }
    /// ```
    ///
    /// However in most cases, this error can simply be handled with a graceful early return.
    /// If this is an expected failure mode, you can do this using the `let else` pattern like so:
    /// ```rust
    /// use bevy_ecs::prelude::*;
    ///
    /// #[derive(Component)]
    /// struct A(usize);
    ///
    /// fn my_system(query: Query<&A>) {
    ///   let Ok(a) = query.single() else {
    ///     return;
    ///   };
    ///
    ///   // Do something with `a`
    /// }
    /// ```
    ///
    /// If this is unexpected though, you should probably use the `?` operator
    /// in combination with Bevy's error handling apparatus.
    ///
    /// ```rust
    /// use bevy_ecs::prelude::*;
    ///
    /// #[derive(Component)]
    /// struct A(usize);
    ///
    /// fn my_system(query: Query<&A>) -> Result {
    ///  let a = query.single()?;
    ///
    ///  // Do something with `a`
    ///  Ok(())
    /// }
    /// ```
    ///
    /// This allows you to globally control how errors are handled in your application,
    /// by setting up a custom error handler.
    /// See the [`bevy_ecs::error`] module docs for more information!
    /// Commonly, you might want to panic on an error during development, but log the error and continue
    /// execution in production.
    ///
    /// Simply unwrapping the [`Result`] also works, but should generally be reserved for tests.
    #[inline]
    pub fn single<'w>(
        &mut self,
        world: &'w World,
    ) -> Result<ROQueryItem<'w, '_, D>, QuerySingleError> {
        self.query(world).single_inner()
    }

    /// Returns a single mutable query result when there is exactly one entity matching
    /// the query.
    ///
    /// If the number of query results is not exactly one, a [`QuerySingleError`] is returned
    /// instead.
    ///
    /// # Examples
    ///
    /// Please see [`Query::single`] for advice on handling the error.
    #[inline]
    pub fn single_mut<'w>(
        &mut self,
        world: &'w mut World,
    ) -> Result<D::Item<'w, '_>, QuerySingleError> {
        self.query_mut(world).single_inner()
    }
}

#[cfg(test)]
mod tests {
    use crate::{component::Component, entity_disabling::DefaultQueryFilters, prelude::*};

    #[test]
    #[should_panic]
    fn right_world_get() {
        let mut world_1 = World::new();
        let world_2 = World::new();

        let mut query_state = world_1.query::<Entity>();
        let _panics = query_state.get(&world_2, Entity::from_raw_u32(0).unwrap());
    }

    #[test]
    #[should_panic]
    fn right_world_get_many() {
        let mut world_1 = World::new();
        let world_2 = World::new();

        let mut query_state = world_1.query::<Entity>();
        let _panics = query_state.get_many(&world_2, []);
    }

    #[test]
    #[should_panic]
    fn right_world_get_many_mut() {
        let mut world_1 = World::new();
        let mut world_2 = World::new();

        let mut query_state = world_1.query::<Entity>();
        let _panics = query_state.get_many_mut(&mut world_2, []);
    }

    #[derive(Component, PartialEq, Debug)]
    struct A(usize);

    #[derive(Component, PartialEq, Debug)]
    struct B(usize);

    #[derive(Component, PartialEq, Debug)]
    struct C(usize);

    #[derive(Component)]
    struct D;

    #[test]
    fn query_respects_default_filters() {
        let mut world = World::new();
        world.spawn((A(0), B(0), D));
        world.spawn((B(0), C(0), D));
        world.spawn((C(0), D));

        world.register_disabling_component::<C>();

        // Without<C> only matches the first entity
        let mut query = QueryState::<&D>::new(&mut world);
        assert_eq!(1, query.iter(&world).count());

        // With<C> matches the last two entities
        let mut query = QueryState::<&D, With<C>>::new(&mut world);
        assert_eq!(2, query.iter(&world).count());

        // Has should bypass the filter entirely
        let mut query = QueryState::<(&D, Has<C>)>::new(&mut world);
        assert_eq!(3, query.iter(&world).count());

        // Allow should bypass the filter entirely
        let mut query = QueryState::<&D, Allow<C>>::new(&mut world);
        assert_eq!(3, query.iter(&world).count());

        // Other filters should still be respected
        let mut query = QueryState::<(&D, Has<C>), Without<B>>::new(&mut world);
        assert_eq!(1, query.iter(&world).count());
    }

    #[derive(Component)]
    struct Table;

    #[derive(Component)]
    #[component(storage = "SparseSet")]
    struct Sparse;

    #[derive(Component)]
    struct Dummy;

    #[test]
    fn query_default_filters_updates_is_dense() {
        let mut world = World::new();
        world.spawn((Dummy, Table, Sparse));
        world.spawn((Dummy, Table));
        world.spawn((Dummy, Sparse));

        let mut query = QueryState::<&Dummy>::new(&mut world);
        // There are no sparse components involved thus the query is dense
        assert!(query.is_dense);
        assert_eq!(3, query.query(&world).count());

        world.register_disabling_component::<Sparse>();

        let mut query = QueryState::<&Dummy>::new(&mut world);
        // The query doesn't ask for sparse components, but the default filters adds
        // a sparse component thus it is NOT dense
        assert!(!query.is_dense);
        assert_eq!(1, query.query(&world).count());

        let mut df = DefaultQueryFilters::from_world(&mut world);
        df.register_disabling_component(world.register_component::<Table>());
        world.insert_resource(df);

        let mut query = QueryState::<&Dummy>::new(&mut world);
        // If the filter is instead a table components, the query can still be dense
        assert!(query.is_dense);
        assert_eq!(1, query.query(&world).count());

        let mut query = QueryState::<&Sparse>::new(&mut world);
        // But only if the original query was dense
        assert!(!query.is_dense);
        assert_eq!(1, query.query(&world).count());
    }
}
