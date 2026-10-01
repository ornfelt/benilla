use bevy_utils::prelude::DebugName;

use crate::{
    batching::BatchingStrategy,
    change_detection::Tick,
    entity::{Entity, EntityEquivalent, EntitySet},
    query::{
        DebugCheckedUnwrap, NopWorldQuery, QueryCombinationIter, QueryData, QueryEntityError,
        QueryFilter, QueryIter, QueryManyIter, QueryManyUniqueIter, QueryParIter, QuerySingleError,
        QueryState, ROQueryItem, ReadOnlyQueryData,
    },
    world::unsafe_world_cell::UnsafeWorldCell,
};
use core::{
    marker::PhantomData,
    mem::MaybeUninit,
    ops::{Deref, DerefMut},
};

/// A [system parameter] that provides selective access to the [`Component`] data stored in a [`World`].
///
/// Queries enable systems to access [entity identifiers] and [components] without requiring direct access to the [`World`].
/// Its iterators and getter methods return *query items*, which are types containing data related to an entity.
///
/// `Query` is a generic data structure that accepts two type parameters:
///
/// - **`D` (query data)**:
///   The type of data fetched by the query, which will be returned as the query item.
///   Only entities that match the requested data will generate an item.
///   Must implement the [`QueryData`] trait.
/// - **`F` (query filter)**:
///   An optional set of conditions that determine whether query items should be kept or discarded.
///   This defaults to [`unit`], which means no additional filters will be applied.
///   Must implement the [`QueryFilter`] trait.
///
/// [system parameter]: crate::system::SystemParam
/// [`Component`]: crate::component::Component
/// [`World`]: crate::world::World
/// [entity identifiers]: Entity
/// [components]: crate::component::Component
///
/// # Similar parameters
///
/// `Query` has few sibling [`SystemParam`]s, which perform additional validation:
///
/// - [`Single`] - Exactly one matching query item.
/// - [`Option<Single>`] - Zero or one matching query item.
/// - [`Populated`] - At least one matching query item.
///
/// These parameters will prevent systems from running if their requirements are not met.
///
/// [`SystemParam`]: crate::system::system_param::SystemParam
/// [`Option<Single>`]: Single
///
/// # System parameter declaration
///
/// A query should always be declared as a system parameter.
/// This section shows the most common idioms involving the declaration of `Query`.
///
/// ## Component access
///
/// You can fetch an entity's component by specifying a reference to that component in the query's data parameter:
///
/// ```
/// # use bevy_ecs::prelude::*;
/// #
/// # #[derive(Component)]
/// # struct ComponentA;
/// #
/// // A component can be accessed by a shared reference...
/// fn immutable_query(query: Query<&ComponentA>) {
///     // ...
/// }
///
/// // ...or by a mutable reference.
/// fn mutable_query(query: Query<&mut ComponentA>) {
///     // ...
/// }
/// #
/// # bevy_ecs::system::assert_is_system(immutable_query);
/// # bevy_ecs::system::assert_is_system(mutable_query);
/// ```
///
/// Note that components need to be behind a reference (`&` or `&mut`), or the query will not compile:
///
/// ```compile_fail,E0277
/// # use bevy_ecs::prelude::*;
/// #
/// # #[derive(Component)]
/// # struct ComponentA;
/// #
/// // This needs to be `&ComponentA` or `&mut ComponentA` in order to compile.
/// fn invalid_query(query: Query<ComponentA>) {
///     // ...
/// }
/// ```
///
/// ## Query filtering
///
/// Setting the query filter type parameter will ensure that each query item satisfies the given condition:
///
/// ```
/// # use bevy_ecs::prelude::*;
/// #
/// # #[derive(Component)]
/// # struct ComponentA;
/// #
/// # #[derive(Component)]
/// # struct ComponentB;
/// #
/// // `ComponentA` data will be accessed, but only for entities that also contain `ComponentB`.
/// fn filtered_query(query: Query<&ComponentA, With<ComponentB>>) {
///     // ...
/// }
/// #
/// # bevy_ecs::system::assert_is_system(filtered_query);
/// ```
///
/// Note that the filter is `With<ComponentB>`, not `With<&ComponentB>`. Unlike query data, `With`
/// does not require components to be behind a reference.
///
/// ## `QueryData` or `QueryFilter` tuples
///
/// Using [`tuple`]s, each `Query` type parameter can contain multiple elements.
///
/// In the following example two components are accessed simultaneously, and the query items are
/// filtered on two conditions:
///
/// ```
/// # use bevy_ecs::prelude::*;
/// #
/// # #[derive(Component)]
/// # struct ComponentA;
/// #
/// # #[derive(Component)]
/// # struct ComponentB;
/// #
/// # #[derive(Component)]
/// # struct ComponentC;
/// #
/// # #[derive(Component)]
/// # struct ComponentD;
/// #
/// fn complex_query(
///     query: Query<(&mut ComponentA, &ComponentB), (With<ComponentC>, Without<ComponentD>)>
/// ) {
///     // ...
/// }
/// #
/// # bevy_ecs::system::assert_is_system(complex_query);
/// ```
///
/// Note that this currently only works on tuples with 15 or fewer items. You may nest tuples to
/// get around this limit:
///
/// ```
/// # use bevy_ecs::prelude::*;
/// #
/// # #[derive(Component)]
/// # struct ComponentA;
/// #
/// # #[derive(Component)]
/// # struct ComponentB;
/// #
/// # #[derive(Component)]
/// # struct ComponentC;
/// #
/// # #[derive(Component)]
/// # struct ComponentD;
/// #
/// fn nested_query(
///     query: Query<(&ComponentA, &ComponentB, (&mut ComponentC, &mut ComponentD))>
/// ) {
///     // ...
/// }
/// #
/// # bevy_ecs::system::assert_is_system(nested_query);
/// ```
///
/// ## Entity identifier access
///
/// You can access [`Entity`], the entity identifier, by including it in the query data parameter:
///
/// ```
/// # use bevy_ecs::prelude::*;
/// #
/// # #[derive(Component)]
/// # struct ComponentA;
/// #
/// fn entity_id_query(query: Query<(Entity, &ComponentA)>) {
///     // ...
/// }
/// #
/// # bevy_ecs::system::assert_is_system(entity_id_query);
/// ```
///
/// Be aware that [`Entity`] is not a component, so it does not need to be behind a reference.
///
/// ## Optional component access
///
/// A component can be made optional by wrapping it into an [`Option`]. In the following example, a
/// query item will still be generated even if the queried entity does not contain `ComponentB`.
/// When this is the case, `Option<&ComponentB>`'s corresponding value will be `None`.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// #
/// # #[derive(Component)]
/// # struct ComponentA;
/// #
/// # #[derive(Component)]
/// # struct ComponentB;
/// #
/// // Queried items must contain `ComponentA`. If they also contain `ComponentB`, its value will
/// // be fetched as well.
/// fn optional_component_query(query: Query<(&ComponentA, Option<&ComponentB>)>) {
///     // ...
/// }
/// #
/// # bevy_ecs::system::assert_is_system(optional_component_query);
/// ```
///
/// Optional components can hurt performance in some cases, so please read the [performance]
/// section to learn more about them. Additionally, if you need to declare several optional
/// components, you may be interested in using [`AnyOf`].
///
/// [performance]: #performance
/// [`AnyOf`]: crate::query::AnyOf
///
/// ## Disjoint queries
///
/// A system cannot contain two queries that break Rust's mutability rules, or else it will panic
/// when initialized. This can often be fixed with the [`Without`] filter, which makes the queries
/// disjoint.
///
/// In the following example, the two queries can mutably access the same `&mut Health` component
/// if an entity has both the `Player` and `Enemy` components. Bevy will catch this and panic,
/// however, instead of breaking Rust's mutability rules:
///
/// ```should_panic
/// # use bevy_ecs::prelude::*;
/// #
/// # #[derive(Component)]
/// # struct Health;
/// #
/// # #[derive(Component)]
/// # struct Player;
/// #
/// # #[derive(Component)]
/// # struct Enemy;
/// #
/// fn randomize_health(
///     player_query: Query<&mut Health, With<Player>>,
///     enemy_query: Query<&mut Health, With<Enemy>>,
/// ) {
///     // ...
/// }
/// #
/// # bevy_ecs::system::assert_system_does_not_conflict(randomize_health);
/// ```
///
/// Adding a [`Without`] filter will disjoint the queries. In the following example, any entity
/// that has both the `Player` and `Enemy` components will be excluded from _both_ queries:
///
/// ```
/// # use bevy_ecs::prelude::*;
/// #
/// # #[derive(Component)]
/// # struct Health;
/// #
/// # #[derive(Component)]
/// # struct Player;
/// #
/// # #[derive(Component)]
/// # struct Enemy;
/// #
/// fn randomize_health(
///     player_query: Query<&mut Health, (With<Player>, Without<Enemy>)>,
///     enemy_query: Query<&mut Health, (With<Enemy>, Without<Player>)>,
/// ) {
///     // ...
/// }
/// #
/// # bevy_ecs::system::assert_system_does_not_conflict(randomize_health);
/// ```
///
/// An alternative solution to this problem would be to wrap the conflicting queries in
/// [`ParamSet`].
///
/// [`Without`]: crate::query::Without
/// [`ParamSet`]: crate::system::ParamSet
///
/// [`With`]: crate::query::With
///
/// # Accessing query items
///
/// The following table summarizes the behavior of safe methods that can be used to get query
/// items:
///
/// |Query methods|Effect|
/// |-|-|
/// |[`iter`]\[[`_mut`][`iter_mut`]\]|Returns an iterator over all query items.|
/// |[`iter[_mut]().for_each()`][`for_each`],<br />[`par_iter`]\[[`_mut`][`par_iter_mut`]\]|Runs a specified function for each query item.|
/// |[`iter_many`]\[[`_unique`][`iter_many_unique`]\]\[[`_mut`][`iter_many_mut`]\]|Iterates over query items that match a list of entities.|
/// |[`iter_combinations`]|Iterates over all combinations of query items.|
/// |[`single`](Self::single)\[[`_mut`][`single_mut`]\]|Returns a single query item if only one exists.|
/// |[`get`]\[[`_mut`][`get_mut`]\]|Returns the query item for a specified entity.|
/// |[`get_many`]\[[`_mut`][`get_many_mut`]\]|Returns all query items that match a list of entities.|
///
/// There are two methods for each type of query operation: immutable and mutable (ending with `_mut`).
/// When using immutable methods, the query items returned are of type [`ROQueryItem`], a read-only version of the query item.
/// In this circumstance, every mutable reference in the query fetch type parameter is substituted by a shared reference.
///
/// [`iter`]: Self::iter
/// [`iter_mut`]: Self::iter_mut
/// [`for_each`]: #iteratorfor_each
/// [`par_iter`]: Self::par_iter
/// [`par_iter_mut`]: Self::par_iter_mut
/// [`iter_many`]: Self::iter_many
/// [`iter_many_unique`]: Self::iter_many_unique
/// [`iter_many_mut`]: Self::iter_many_mut
/// [`iter_combinations`]: Self::iter_combinations
/// [`single_mut`]: Self::single_mut
/// [`get`]: Self::get
/// [`get_mut`]: Self::get_mut
/// [`get_many`]: Self::get_many
/// [`get_many_mut`]: Self::get_many_mut
///
/// # Performance
///
/// Creating a `Query` is a low-cost constant operation. Iterating it, on the other hand, fetches
/// data from the world and generates items, which can have a significant computational cost.
///
/// Two systems cannot be executed in parallel if both access the same component type where at
/// least one of the accesses is mutable. Because of this, it is recommended for queries to only
/// fetch mutable access to components when necessary, since immutable access can be parallelized.
///
/// Query filters ([`With`] / [`Without`]) can improve performance because they narrow the kinds of
/// entities that can be fetched. Systems that access fewer kinds of entities are more likely to be
/// parallelized by the scheduler.
///
/// On the other hand, be careful using optional components (`Option<&ComponentA>`) because they
/// broaden the amount of entities kinds that can be accessed. This is especially true of a query
/// that _only_ fetches optional components, as the query would iterate over all entities in the
/// world.
///
/// There are two types of [component storage types]: [`Table`] and [`SparseSet`]. [`Table`] offers
/// fast iteration speeds, but slower insertion and removal speeds. [`SparseSet`] is the opposite:
/// it offers fast component insertion and removal speeds, but slower iteration speeds.
///
/// The following table compares the computational complexity of the various methods and
/// operations, where:
///
/// - **n** is the number of entities that match the query.
/// - **r** is the number of elements in a combination.
/// - **k** is the number of involved entities in the operation.
/// - **a** is the number of archetypes in the world.
/// - **C** is the [binomial coefficient], used to count combinations. <sub>n</sub>C<sub>r</sub> is
///   read as "*n* choose *r*" and is equivalent to the number of distinct unordered subsets of *r*
///   elements that can be taken from a set of *n* elements.
///
/// |Query operation|Computational complexity|
/// |-|-|
/// |[`iter`]\[[`_mut`][`iter_mut`]\]|O(n)|
/// |[`iter[_mut]().for_each()`][`for_each`],<br/>[`par_iter`]\[[`_mut`][`par_iter_mut`]\]|O(n)|
/// |[`iter_many`]\[[`_mut`][`iter_many_mut`]\]|O(k)|
/// |[`iter_combinations`]|O(<sub>n</sub>C<sub>r</sub>)|
/// |[`single`](Self::single)\[[`_mut`][`single_mut`]\]|O(a)|
/// |[`get`]\[[`_mut`][`get_mut`]\]|O(1)|
/// |[`get_many`]|O(k)|
/// |[`get_many_mut`]|O(k<sup>2</sup>)|
/// |Archetype-based filtering ([`With`], [`Without`], [`Or`])|O(a)|
/// |Change detection filtering ([`Added`], [`Changed`], [`Spawned`])|O(a + n)|
///
/// [component storage types]: crate::component::StorageType
/// [`Table`]: crate::storage::Table
/// [`SparseSet`]: crate::storage::SparseSet
/// [binomial coefficient]: https://en.wikipedia.org/wiki/Binomial_coefficient
/// [`Or`]: crate::query::Or
/// [`Added`]: crate::query::Added
/// [`Changed`]: crate::query::Changed
/// [`Spawned`]: crate::query::Spawned
///
/// # `Iterator::for_each`
///
/// The `for_each` methods appear to be generally faster than `for`-loops when run on worlds with
/// high archetype fragmentation, and may enable additional optimizations like [autovectorization]. It
/// is strongly advised to only use [`Iterator::for_each`] if it tangibly improves performance.
/// *Always* profile or benchmark before and after the change!
///
/// ```rust
/// # use bevy_ecs::prelude::*;
/// #
/// # #[derive(Component)]
/// # struct ComponentA;
/// #
/// fn system(query: Query<&ComponentA>) {
///     // This may result in better performance...
///     query.iter().for_each(|component| {
///         // ...
///     });
///
///     // ...than this. Always benchmark to validate the difference!
///     for component in query.iter() {
///         // ...
///     }
/// }
/// #
/// # bevy_ecs::system::assert_is_system(system);
/// ```
///
/// [autovectorization]: https://en.wikipedia.org/wiki/Automatic_vectorization
pub struct Query<'world, 'state, D: QueryData, F: QueryFilter = ()> {
    // SAFETY: Must have access to the components registered in `state`.
    world: UnsafeWorldCell<'world>,
    state: &'state QueryState<D, F>,
    last_run: Tick,
    this_run: Tick,
}

impl<D: ReadOnlyQueryData, F: QueryFilter> Clone for Query<'_, '_, D, F> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<D: ReadOnlyQueryData, F: QueryFilter> Copy for Query<'_, '_, D, F> {}

impl<D: QueryData, F: QueryFilter> core::fmt::Debug for Query<'_, '_, D, F> {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        f.debug_struct("Query")
            .field("matched_entities", &self.iter().count())
            .field("state", &self.state)
            .field("last_run", &self.last_run)
            .field("this_run", &self.this_run)
            .field("world", &self.world)
            .finish()
    }
}

impl<'w, 's, D: QueryData, F: QueryFilter> Query<'w, 's, D, F> {
    /// Creates a new query.
    ///
    /// # Safety
    ///
    /// * This will create a query that could violate memory safety rules. Make sure that this is only
    ///   called in ways that ensure the queries have unique mutable access.
    /// * `world` must be the world used to create `state`.
    #[inline]
    pub(crate) unsafe fn new(
        world: UnsafeWorldCell<'w>,
        state: &'s QueryState<D, F>,
        last_run: Tick,
        this_run: Tick,
    ) -> Self {
        Self {
            world,
            state,
            last_run,
            this_run,
        }
    }

    /// Returns another `Query` from this that fetches the read-only version of the query items.
    ///
    /// For example, `Query<(&mut D1, &D2, &mut D3), With<F>>` will become `Query<(&D1, &D2, &D3), With<F>>`.
    /// This can be useful when working around the borrow checker,
    /// or reusing functionality between systems via functions that accept query types.
    ///
    /// # See also
    ///
    /// [`into_readonly`](Self::into_readonly) for a version that consumes the `Query` to return one with the full `'world` lifetime.
    pub fn as_readonly(&self) -> Query<'_, 's, D::ReadOnly, F> {
        // SAFETY: The reborrowed query is converted to read-only, so it cannot perform mutable access,
        // and the original query is held with a shared borrow, so it cannot perform mutable access either.
        unsafe { self.reborrow_unsafe() }.into_readonly()
    }

    /// Returns another `Query` from this does not return any data, which can be faster.
    ///
    /// The resulting query will ignore any non-archetypal filters in `D`,
    /// so this is only equivalent if `D::IS_ARCHETYPAL` is `true`.
    fn as_nop(&self) -> Query<'_, 's, NopWorldQuery<D>, F> {
        let new_state = self.state.as_nop();
        // SAFETY:
        // - The reborrowed query is converted to read-only, so it cannot perform mutable access,
        //   and the original query is held with a shared borrow, so it cannot perform mutable access either.
        //   Note that although `NopWorldQuery` itself performs *no* access and could soundly alias a mutable query,
        //   it has the original `QueryState::component_access` and could be `transmute`d to a read-only query.
        // - The world matches because it was the same one used to construct self.
        unsafe { Query::new(self.world, new_state, self.last_run, self.this_run) }
    }

    /// Returns another `Query` from this that fetches the read-only version of the query items.
    ///
    /// For example, `Query<(&mut D1, &D2, &mut D3), With<F>>` will become `Query<(&D1, &D2, &D3), With<F>>`.
    /// This can be useful when working around the borrow checker,
    /// or reusing functionality between systems via functions that accept query types.
    ///
    /// # See also
    ///
    /// [`as_readonly`](Self::as_readonly) for a version that borrows the `Query` instead of consuming it.
    pub fn into_readonly(self) -> Query<'w, 's, D::ReadOnly, F> {
        let new_state = self.state.as_readonly();
        // SAFETY:
        // - This is memory safe because it turns the query immutable.
        // - The world matches because it was the same one used to construct self.
        unsafe { Query::new(self.world, new_state, self.last_run, self.this_run) }
    }

    /// Returns a new `Query` reborrowing the access from this one. The current query will be unusable
    /// while the new one exists.
    ///
    /// # Example
    ///
    /// For example this allows to call other methods or other systems that require an owned `Query` without
    /// completely giving up ownership of it.
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # #[derive(Component)]
    /// # struct ComponentA;
    ///
    /// fn helper_system(query: Query<&ComponentA>) { /* ... */}
    ///
    /// fn system(mut query: Query<&ComponentA>) {
    ///     helper_system(query.reborrow());
    ///     // Can still use query here:
    ///     for component in &query {
    ///         // ...
    ///     }
    /// }
    /// ```
    pub fn reborrow(&mut self) -> Query<'_, 's, D, F> {
        // SAFETY: this query is exclusively borrowed while the new one exists, so
        // no overlapping access can occur.
        unsafe { self.reborrow_unsafe() }
    }

    /// Returns a new `Query` reborrowing the access from this one.
    /// The current query will still be usable while the new one exists, but must not be used in a way that violates aliasing.
    ///
    /// # Safety
    ///
    /// This function makes it possible to violate Rust's aliasing guarantees.
    /// You must make sure this call does not result in a mutable or shared reference to a component with a mutable reference.
    ///
    /// # See also
    ///
    /// - [`reborrow`](Self::reborrow) for the safe versions.
    pub unsafe fn reborrow_unsafe(&self) -> Query<'_, 's, D, F> {
        // SAFETY:
        // - This is memory safe because the caller ensures that there are no conflicting references.
        // - The world matches because it was the same one used to construct self.
        unsafe { self.copy_unsafe() }
    }

    /// Returns a new `Query` copying the access from this one.
    /// The current query will still be usable while the new one exists, but must not be used in a way that violates aliasing.
    ///
    /// # Safety
    ///
    /// This function makes it possible to violate Rust's aliasing guarantees.
    /// You must make sure this call does not result in a mutable or shared reference to a component with a mutable reference.
    ///
    /// # See also
    ///
    /// - [`reborrow_unsafe`](Self::reborrow_unsafe) for a safer version that constrains the returned `'w` lifetime to the length of the borrow.
    unsafe fn copy_unsafe(&self) -> Query<'w, 's, D, F> {
        // SAFETY:
        // - This is memory safe because the caller ensures that there are no conflicting references.
        // - The world matches because it was the same one used to construct self.
        unsafe { Query::new(self.world, self.state, self.last_run, self.this_run) }
    }

    /// Returns an [`Iterator`] over the read-only query items.
    ///
    /// This iterator is always guaranteed to return results from each matching entity once and only once.
    /// Iteration order is not guaranteed.
    ///
    /// # Example
    ///
    /// Here, the `report_names_system` iterates over the `Player` component of every entity that contains it:
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # #[derive(Component)]
    /// # struct Player { name: String }
    /// #
    /// fn report_names_system(query: Query<&Player>) {
    ///     for player in &query {
    ///         println!("Say hello to {}!", player.name);
    ///     }
    /// }
    /// # bevy_ecs::system::assert_is_system(report_names_system);
    /// ```
    ///
    /// # See also
    ///
    /// [`iter_mut`](Self::iter_mut) for mutable query items.
    #[inline]
    pub fn iter(&self) -> QueryIter<'_, 's, D::ReadOnly, F> {
        self.as_readonly().into_iter()
    }

    /// Returns an [`Iterator`] over the query items.
    ///
    /// This iterator is always guaranteed to return results from each matching entity once and only once.
    /// Iteration order is not guaranteed.
    ///
    /// # Example
    ///
    /// Here, the `gravity_system` updates the `Velocity` component of every entity that contains it:
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # #[derive(Component)]
    /// # struct Velocity { x: f32, y: f32, z: f32 }
    /// fn gravity_system(mut query: Query<&mut Velocity>) {
    ///     const DELTA: f32 = 1.0 / 60.0;
    ///     for mut velocity in &mut query {
    ///         velocity.y -= 9.8 * DELTA;
    ///     }
    /// }
    /// # bevy_ecs::system::assert_is_system(gravity_system);
    /// ```
    ///
    /// # See also
    ///
    /// [`iter`](Self::iter) for read-only query items.
    #[inline]
    pub fn iter_mut(&mut self) -> QueryIter<'_, 's, D, F> {
        self.reborrow().into_iter()
    }

    /// Returns a [`QueryCombinationIter`] over all combinations of `K` read-only query items without repetition.
    ///
    /// This iterator is always guaranteed to return results from each unique pair of matching entities.
    /// Iteration order is not guaranteed.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # #[derive(Component)]
    /// # struct ComponentA;
    /// #
    /// fn some_system(query: Query<&ComponentA>) {
    ///     for [a1, a2] in query.iter_combinations() {
    ///         // ...
    ///     }
    /// }
    /// ```
    ///
    /// # See also
    ///
    /// - [`iter_combinations_inner`](Self::iter_combinations_inner) for mutable query item combinations with the full `'world` lifetime.
    #[inline]
    pub fn iter_combinations<const K: usize>(
        &self,
    ) -> QueryCombinationIter<'_, 's, D::ReadOnly, F, K> {
        self.as_readonly().iter_combinations_inner()
    }

    /// Returns a [`QueryCombinationIter`] over all combinations of `K` query items without repetition.
    /// This consumes the [`Query`] to return results with the actual "inner" world lifetime.
    ///
    /// This iterator is always guaranteed to return results from each unique pair of matching entities.
    /// Iteration order is not guaranteed.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # #[derive(Component)]
    /// # struct ComponentA;
    /// fn some_system(query: Query<&mut ComponentA>) {
    ///     let mut combinations = query.iter_combinations_inner();
    ///     while let Some([mut a1, mut a2]) = combinations.fetch_next() {
    ///         // mutably access components data
    ///     }
    /// }
    /// ```
    ///
    /// # See also
    ///
    /// - [`iter_combinations`](Self::iter_combinations) for read-only query item combinations.
    #[inline]
    pub fn iter_combinations_inner<const K: usize>(self) -> QueryCombinationIter<'w, 's, D, F, K> {
        // SAFETY: `self.world` has permission to access the required components.
        unsafe { QueryCombinationIter::new(self.world, self.state, self.last_run, self.this_run) }
    }

    /// Returns an [`Iterator`] over the read-only query items generated from an [`Entity`] list.
    ///
    /// Items are returned in the order of the list of entities, and may not be unique if the input
    /// doesn't guarantee uniqueness. Entities that don't match the query are skipped.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # #[derive(Component)]
    /// # struct Counter {
    /// #     value: i32
    /// # }
    /// #
    /// // A component containing an entity list.
    /// #[derive(Component)]
    /// struct Friends {
    ///     list: Vec<Entity>,
    /// }
    ///
    /// fn system(
    ///     friends_query: Query<&Friends>,
    ///     counter_query: Query<&Counter>,
    /// ) {
    ///     for friends in &friends_query {
    ///         for counter in counter_query.iter_many(&friends.list) {
    ///             println!("Friend's counter: {}", counter.value);
    ///         }
    ///     }
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    ///
    /// # See also
    ///
    /// - [`iter_many_mut`](Self::iter_many_mut) to get mutable query items.
    /// - [`iter_many_inner`](Self::iter_many_inner) to get mutable query items with the full `'world` lifetime.
    #[inline]
    pub fn iter_many<EntityList: IntoIterator<Item: EntityEquivalent>>(
        &self,
        entities: EntityList,
    ) -> QueryManyIter<'_, 's, D::ReadOnly, F, EntityList::IntoIter> {
        self.as_readonly().iter_many_inner(entities)
    }

    /// Returns an iterator over the query items generated from an [`Entity`] list.
    ///
    /// Items are returned in the order of the list of entities, and may not be unique if the input
    /// doesn't guarantee uniqueness. Entities that don't match the query are skipped.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #[derive(Component)]
    /// struct Counter {
    ///     value: i32
    /// }
    ///
    /// #[derive(Component)]
    /// struct Friends {
    ///     list: Vec<Entity>,
    /// }
    ///
    /// fn system(
    ///     friends_query: Query<&Friends>,
    ///     mut counter_query: Query<&mut Counter>,
    /// ) {
    ///     for friends in &friends_query {
    ///         let mut iter = counter_query.iter_many_mut(&friends.list);
    ///         while let Some(mut counter) = iter.fetch_next() {
    ///             println!("Friend's counter: {}", counter.value);
    ///             counter.value += 1;
    ///         }
    ///     }
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    /// # See also
    ///
    /// - [`iter_many`](Self::iter_many) to get read-only query items.
    /// - [`iter_many_inner`](Self::iter_many_inner) to get mutable query items with the full `'world` lifetime.
    #[inline]
    pub fn iter_many_mut<EntityList: IntoIterator<Item: EntityEquivalent>>(
        &mut self,
        entities: EntityList,
    ) -> QueryManyIter<'_, 's, D, F, EntityList::IntoIter> {
        self.reborrow().iter_many_inner(entities)
    }

    /// Returns an iterator over the query items generated from an [`Entity`] list.
    /// This consumes the [`Query`] to return results with the actual "inner" world lifetime.
    ///
    /// Items are returned in the order of the list of entities, and may not be unique if the input
    /// doesn't guarantee uniqueness. Entities that don't match the query are skipped.
    ///
    /// # See also
    ///
    /// - [`iter_many`](Self::iter_many) to get read-only query items.
    /// - [`iter_many_mut`](Self::iter_many_mut) to get mutable query items.
    #[inline]
    pub fn iter_many_inner<EntityList: IntoIterator<Item: EntityEquivalent>>(
        self,
        entities: EntityList,
    ) -> QueryManyIter<'w, 's, D, F, EntityList::IntoIter> {
        // SAFETY: `self.world` has permission to access the required components.
        unsafe {
            QueryManyIter::new(
                self.world,
                self.state,
                entities,
                self.last_run,
                self.this_run,
            )
        }
    }

    /// Returns an [`Iterator`] over the unique read-only query items generated from an [`EntitySet`].
    ///
    /// Items are returned in the order of the list of entities. Entities that don't match the query are skipped.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::{prelude::*, entity::{EntitySet, UniqueEntityIter}};
    /// # use core::slice;
    /// # #[derive(Component)]
    /// # struct Counter {
    /// #     value: i32
    /// # }
    /// #
    /// // `Friends` ensures that it only lists unique entities.
    /// #[derive(Component)]
    /// struct Friends {
    ///     unique_list: Vec<Entity>,
    /// }
    ///
    /// impl<'a> IntoIterator for &'a Friends {
    ///
    ///     type Item = &'a Entity;
    ///     type IntoIter = UniqueEntityIter<slice::Iter<'a, Entity>>;
    ///
    ///     fn into_iter(self) -> Self::IntoIter {
    ///         // SAFETY: `Friends` ensures that it unique_list contains only unique entities.
    ///        unsafe { UniqueEntityIter::from_iterator_unchecked(self.unique_list.iter()) }
    ///     }
    /// }
    ///
    /// fn system(
    ///     friends_query: Query<&Friends>,
    ///     counter_query: Query<&Counter>,
    /// ) {
    ///     for friends in &friends_query {
    ///         for counter in counter_query.iter_many_unique(friends) {
    ///             println!("Friend's counter: {:?}", counter.value);
    ///         }
    ///     }
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    ///
    /// # See also
    ///
    /// - [`iter_many_unique_mut`](Self::iter_many_unique_mut) to get mutable query items.
    /// - [`iter_many_unique_inner`](Self::iter_many_unique_inner) to get with the actual "inner" world lifetime.
    #[inline]
    pub fn iter_many_unique<EntityList: EntitySet>(
        &self,
        entities: EntityList,
    ) -> QueryManyUniqueIter<'_, 's, D::ReadOnly, F, EntityList::IntoIter> {
        self.as_readonly().iter_many_unique_inner(entities)
    }

    /// Returns an iterator over the unique query items generated from an [`EntitySet`].
    ///
    /// Items are returned in the order of the list of entities. Entities that don't match the query are skipped.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_ecs::{prelude::*, entity::{EntitySet, UniqueEntityIter}};
    /// # use core::slice;
    /// #[derive(Component)]
    /// struct Counter {
    ///     value: i32
    /// }
    ///
    /// // `Friends` ensures that it only lists unique entities.
    /// #[derive(Component)]
    /// struct Friends {
    ///     unique_list: Vec<Entity>,
    /// }
    ///
    /// impl<'a> IntoIterator for &'a Friends {
    ///     type Item = &'a Entity;
    ///     type IntoIter = UniqueEntityIter<slice::Iter<'a, Entity>>;
    ///
    ///     fn into_iter(self) -> Self::IntoIter {
    ///         // SAFETY: `Friends` ensures that it unique_list contains only unique entities.
    ///         unsafe { UniqueEntityIter::from_iterator_unchecked(self.unique_list.iter()) }
    ///     }
    /// }
    ///
    /// fn system(
    ///     friends_query: Query<&Friends>,
    ///     mut counter_query: Query<&mut Counter>,
    /// ) {
    ///     for friends in &friends_query {
    ///         for mut counter in counter_query.iter_many_unique_mut(friends) {
    ///             println!("Friend's counter: {:?}", counter.value);
    ///             counter.value += 1;
    ///         }
    ///     }
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    /// # See also
    ///
    /// - [`iter_many_unique`](Self::iter_many_unique) to get read-only query items.
    /// - [`iter_many_unique_inner`](Self::iter_many_unique_inner) to get with the actual "inner" world lifetime.
    #[inline]
    pub fn iter_many_unique_mut<EntityList: EntitySet>(
        &mut self,
        entities: EntityList,
    ) -> QueryManyUniqueIter<'_, 's, D, F, EntityList::IntoIter> {
        self.reborrow().iter_many_unique_inner(entities)
    }

    /// Returns an iterator over the unique query items generated from an [`EntitySet`].
    /// This consumes the [`Query`] to return results with the actual "inner" world lifetime.
    ///
    /// Items are returned in the order of the list of entities. Entities that don't match the query are skipped.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_ecs::{prelude::*, entity::{EntitySet, UniqueEntityIter}};
    /// # use core::slice;
    /// #[derive(Component)]
    /// struct Counter {
    ///     value: i32
    /// }
    ///
    /// // `Friends` ensures that it only lists unique entities.
    /// #[derive(Component)]
    /// struct Friends {
    ///     unique_list: Vec<Entity>,
    /// }
    ///
    /// impl<'a> IntoIterator for &'a Friends {
    ///     type Item = &'a Entity;
    ///     type IntoIter = UniqueEntityIter<slice::Iter<'a, Entity>>;
    ///
    ///     fn into_iter(self) -> Self::IntoIter {
    ///         // SAFETY: `Friends` ensures that it unique_list contains only unique entities.
    ///         unsafe { UniqueEntityIter::from_iterator_unchecked(self.unique_list.iter()) }
    ///     }
    /// }
    ///
    /// fn system(
    ///     friends_query: Query<&Friends>,
    ///     mut counter_query: Query<&mut Counter>,
    /// ) {
    ///     let friends = friends_query.single().unwrap();
    ///     for mut counter in counter_query.iter_many_unique_inner(friends) {
    ///         println!("Friend's counter: {:?}", counter.value);
    ///         counter.value += 1;
    ///     }
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    /// # See also
    ///
    /// - [`iter_many_unique`](Self::iter_many_unique) to get read-only query items.
    /// - [`iter_many_unique_mut`](Self::iter_many_unique_mut) to get mutable query items.
    #[inline]
    pub fn iter_many_unique_inner<EntityList: EntitySet>(
        self,
        entities: EntityList,
    ) -> QueryManyUniqueIter<'w, 's, D, F, EntityList::IntoIter> {
        // SAFETY: `self.world` has permission to access the required components.
        unsafe {
            QueryManyUniqueIter::new(
                self.world,
                self.state,
                entities,
                self.last_run,
                self.this_run,
            )
        }
    }

    /// Returns an [`Iterator`] over the unique query items generated from an [`Entity`] list.
    ///
    /// Items are returned in the order of the list of entities. Entities that don't match the query are skipped.
    ///
    /// # Safety
    ///
    /// This allows aliased mutability.
    /// You must make sure this call does not result in multiple mutable references to the same component.
    ///
    /// # See also
    ///
    /// - [`iter_many_unique`](Self::iter_many_unique) to get read-only query items.
    /// - [`iter_many_unique_mut`](Self::iter_many_unique_mut) to get mutable query items.
    /// - [`iter_many_unique_inner`](Self::iter_many_unique_inner) to get with the actual "inner" world lifetime.
    pub unsafe fn iter_many_unique_unsafe<EntityList: EntitySet>(
        &self,
        entities: EntityList,
    ) -> QueryManyUniqueIter<'_, 's, D, F, EntityList::IntoIter> {
        // SAFETY: The caller promises that this will not result in multiple mutable references.
        unsafe { self.reborrow_unsafe() }.iter_many_unique_inner(entities)
    }

    /// Returns a parallel iterator over the query results for the given [`World`].
    ///
    /// This parallel iterator is always guaranteed to return results from each matching entity once and
    /// only once.  Iteration order and thread assignment is not guaranteed.
    ///
    /// If the `multithreaded` feature is disabled, iterating with this operates identically to [`Iterator::for_each`]
    /// on [`QueryIter`].
    ///
    /// This can only be called for read-only queries, see [`par_iter_mut`] for write-queries.
    ///
    /// Note that you must use the `for_each` method to iterate over the
    /// results, see [`par_iter_mut`] for an example.
    ///
    /// [`par_iter_mut`]: Self::par_iter_mut
    /// [`World`]: crate::world::World
    #[inline]
    pub fn par_iter(&self) -> QueryParIter<'_, 's, D::ReadOnly, F> {
        self.as_readonly().par_iter_inner()
    }

    /// Returns a parallel iterator over the query results for the given [`World`].
    ///
    /// This parallel iterator is always guaranteed to return results from each matching entity once and
    /// only once.  Iteration order and thread assignment is not guaranteed.
    ///
    /// If the `multithreaded` feature is disabled, iterating with this operates identically to [`Iterator::for_each`]
    /// on [`QueryIter`].
    ///
    /// This can only be called for mutable queries, see [`par_iter`] for read-only-queries.
    ///
    /// # Example
    ///
    /// Here, the `gravity_system` updates the `Velocity` component of every entity that contains it:
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # #[derive(Component)]
    /// # struct Velocity { x: f32, y: f32, z: f32 }
    /// fn gravity_system(mut query: Query<&mut Velocity>) {
    ///     const DELTA: f32 = 1.0 / 60.0;
    ///     query.par_iter_mut().for_each(|mut velocity| {
    ///         velocity.y -= 9.8 * DELTA;
    ///     });
    /// }
    /// # bevy_ecs::system::assert_is_system(gravity_system);
    /// ```
    ///
    /// [`par_iter`]: Self::par_iter
    /// [`World`]: crate::world::World
    #[inline]
    pub fn par_iter_mut(&mut self) -> QueryParIter<'_, 's, D, F> {
        self.reborrow().par_iter_inner()
    }

    /// Returns a parallel iterator over the query results for the given [`World`](crate::world::World).
    /// This consumes the [`Query`] to return results with the actual "inner" world lifetime.
    ///
    /// This parallel iterator is always guaranteed to return results from each matching entity once and
    /// only once.  Iteration order and thread assignment is not guaranteed.
    ///
    /// If the `multithreaded` feature is disabled, iterating with this operates identically to [`Iterator::for_each`]
    /// on [`QueryIter`].
    ///
    /// # Example
    ///
    /// Here, the `gravity_system` updates the `Velocity` component of every entity that contains it:
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # #[derive(Component)]
    /// # struct Velocity { x: f32, y: f32, z: f32 }
    /// fn gravity_system(query: Query<&mut Velocity>) {
    ///     const DELTA: f32 = 1.0 / 60.0;
    ///     query.par_iter_inner().for_each(|mut velocity| {
    ///         velocity.y -= 9.8 * DELTA;
    ///     });
    /// }
    /// # bevy_ecs::system::assert_is_system(gravity_system);
    /// ```
    #[inline]
    pub fn par_iter_inner(self) -> QueryParIter<'w, 's, D, F> {
        QueryParIter {
            world: self.world,
            state: self.state,
            last_run: self.last_run,
            this_run: self.this_run,
            batching_strategy: BatchingStrategy::new(),
        }
    }

    /// Returns the read-only query item for the given [`Entity`].
    ///
    /// In case of a nonexisting entity or mismatched component, a [`QueryEntityError`] is returned instead.
    ///
    /// This is always guaranteed to run in `O(1)` time.
    ///
    /// # Example
    ///
    /// Here, `get` is used to retrieve the exact query item of the entity specified by the `SelectedCharacter` resource.
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # #[derive(Resource)]
    /// # struct SelectedCharacter { entity: Entity }
    /// # #[derive(Component)]
    /// # struct Character { name: String }
    /// #
    /// fn print_selected_character_name_system(
    ///        query: Query<&Character>,
    ///        selection: Res<SelectedCharacter>
    /// )
    /// {
    ///     if let Ok(selected_character) = query.get(selection.entity) {
    ///         println!("{}", selected_character.name);
    ///     }
    /// }
    /// # bevy_ecs::system::assert_is_system(print_selected_character_name_system);
    /// ```
    ///
    /// # See also
    ///
    /// - [`get_mut`](Self::get_mut) to get a mutable query item.
    #[inline]
    pub fn get(&self, entity: Entity) -> Result<ROQueryItem<'_, 's, D>, QueryEntityError> {
        self.as_readonly().get_inner(entity)
    }

    /// Returns the read-only query items for the given array of [`Entity`].
    ///
    /// The returned query items are in the same order as the input.
    /// In case of a nonexisting entity or mismatched component, a [`QueryEntityError`] is returned instead.
    /// The elements of the array do not need to be unique, unlike `get_many_mut`.
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
    /// let entity_vec: Vec<Entity> = (0..3).map(|i| world.spawn(A(i)).id()).collect();
    /// let entities: [Entity; 3] = entity_vec.try_into().unwrap();
    ///
    /// world.spawn(A(73));
    ///
    /// let mut query_state = world.query::<&A>();
    /// let query = query_state.query(&world);
    ///
    /// let component_values = query.get_many(entities).unwrap();
    ///
    /// assert_eq!(component_values, [&A(0), &A(1), &A(2)]);
    ///
    /// let wrong_entity = Entity::from_raw_u32(365).unwrap();
    ///
    /// assert_eq!(
    ///     match query.get_many([wrong_entity]).unwrap_err() {
    ///         QueryEntityError::NotSpawned(error) => error.entity(),
    ///         _ => panic!(),
    ///     },
    ///     wrong_entity
    /// );
    /// ```
    ///
    /// # See also
    ///
    /// - [`get_many_mut`](Self::get_many_mut) to get mutable query items.
    #[inline]
    pub fn get_many<const N: usize>(
        &self,
        entities: [Entity; N],
    ) -> Result<[ROQueryItem<'_, 's, D>; N], QueryEntityError> {
        // Note that we call a separate `*_inner` method from `get_many_mut`
        // because we don't need to check for duplicates.
        self.as_readonly().get_many_inner(entities)
    }

    /// Returns the query item for the given [`Entity`].
    ///
    /// In case of a nonexisting entity or mismatched component, a [`QueryEntityError`] is returned instead.
    ///
    /// This is always guaranteed to run in `O(1)` time.
    ///
    /// # Example
    ///
    /// Here, `get_mut` is used to retrieve the exact query item of the entity specified by the `PoisonedCharacter` resource.
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # #[derive(Resource)]
    /// # struct PoisonedCharacter { character_id: Entity }
    /// # #[derive(Component)]
    /// # struct Health(u32);
    /// #
    /// fn poison_system(mut query: Query<&mut Health>, poisoned: Res<PoisonedCharacter>) {
    ///     if let Ok(mut health) = query.get_mut(poisoned.character_id) {
    ///         health.0 -= 1;
    ///     }
    /// }
    /// # bevy_ecs::system::assert_is_system(poison_system);
    /// ```
    ///
    /// # See also
    ///
    /// - [`get`](Self::get) to get a read-only query item.
    #[inline]
    pub fn get_mut(&mut self, entity: Entity) -> Result<D::Item<'_, 's>, QueryEntityError> {
        self.reborrow().get_inner(entity)
    }

    /// Returns the query item for the given [`Entity`].
    /// This consumes the [`Query`] to return results with the actual "inner" world lifetime.
    ///
    /// In case of a nonexisting entity or mismatched component, a [`QueryEntityError`] is returned instead.
    ///
    /// This is always guaranteed to run in `O(1)` time.
    ///
    /// # See also
    ///
    /// - [`get_mut`](Self::get_mut) to get the item using a mutable borrow of the [`Query`].
    #[inline]
    pub fn get_inner(self, entity: Entity) -> Result<D::Item<'w, 's>, QueryEntityError> {
        // SAFETY: system runs without conflicts with other systems.
        // same-system queries have runtime borrow checks when they conflict
        unsafe {
            let location = self.world.entities().get_spawned(entity)?;
            if !self
                .state
                .matched_archetypes
                .contains(location.archetype_id.index())
            {
                return Err(QueryEntityError::QueryDoesNotMatch(
                    entity,
                    location.archetype_id,
                ));
            }
            let archetype = self
                .world
                .archetypes()
                .get(location.archetype_id)
                .debug_checked_unwrap();
            let mut fetch = D::init_fetch(
                self.world,
                &self.state.fetch_state,
                self.last_run,
                self.this_run,
            );
            let mut filter = F::init_fetch(
                self.world,
                &self.state.filter_state,
                self.last_run,
                self.this_run,
            );

            let table = self
                .world
                .storages()
                .tables
                .get(location.table_id)
                .debug_checked_unwrap();
            D::set_archetype(&mut fetch, &self.state.fetch_state, archetype, table);
            F::set_archetype(&mut filter, &self.state.filter_state, archetype, table);

            if F::filter_fetch(
                &self.state.filter_state,
                &mut filter,
                entity,
                location.table_row,
            ) && let Some(item) = D::fetch(
                &self.state.fetch_state,
                &mut fetch,
                entity,
                location.table_row,
            ) {
                Ok(item)
            } else {
                Err(QueryEntityError::QueryDoesNotMatch(
                    entity,
                    location.archetype_id,
                ))
            }
        }
    }

    /// Returns the query items for the given array of [`Entity`].
    ///
    /// The returned query items are in the same order as the input.
    /// In case of a nonexisting entity, duplicate entities or mismatched component, a [`QueryEntityError`] is returned instead.
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
    ///
    /// let entities: Vec<Entity> = (0..3).map(|i| world.spawn(A(i)).id()).collect();
    /// let entities: [Entity; 3] = entities.try_into().unwrap();
    ///
    /// world.spawn(A(73));
    /// let wrong_entity = Entity::from_raw_u32(57).unwrap();
    /// let invalid_entity = world.spawn_empty().id();
    ///
    ///
    /// let mut query_state = world.query::<&mut A>();
    /// let mut query = query_state.query_mut(&mut world);
    ///
    /// let mut mutable_component_values = query.get_many_mut(entities).unwrap();
    ///
    /// for mut a in &mut mutable_component_values {
    ///     a.0 += 5;
    /// }
    ///
    /// let component_values = query.get_many(entities).unwrap();
    ///
    /// assert_eq!(component_values, [&A(5), &A(6), &A(7)]);
    ///
    /// assert_eq!(
    ///     match query
    ///         .get_many_mut([wrong_entity])
    ///         .unwrap_err()
    ///     {
    ///         QueryEntityError::NotSpawned(error) => error.entity(),
    ///         _ => panic!(),
    ///     },
    ///     wrong_entity
    /// );
    /// assert_eq!(
    ///     match query
    ///         .get_many_mut([invalid_entity])
    ///         .unwrap_err()
    ///     {
    ///         QueryEntityError::QueryDoesNotMatch(entity, _) => entity,
    ///         _ => panic!(),
    ///     },
    ///     invalid_entity
    /// );
    /// assert_eq!(
    ///     query
    ///         .get_many_mut([entities[0], entities[0]])
    ///         .unwrap_err(),
    ///     QueryEntityError::AliasedMutability(entities[0])
    /// );
    /// ```
    /// # See also
    ///
    /// - [`get_many`](Self::get_many) to get read-only query items without checking for duplicate entities.
    #[inline]
    pub fn get_many_mut<const N: usize>(
        &mut self,
        entities: [Entity; N],
    ) -> Result<[D::Item<'_, 's>; N], QueryEntityError> {
        self.reborrow().get_many_mut_inner(entities)
    }

    /// Returns the query items for the given array of [`Entity`].
    /// This consumes the [`Query`] to return results with the actual "inner" world lifetime.
    ///
    /// The returned query items are in the same order as the input.
    /// In case of a nonexisting entity, duplicate entities or mismatched component, a [`QueryEntityError`] is returned instead.
    ///
    /// # See also
    ///
    /// - [`get_many`](Self::get_many) to get read-only query items without checking for duplicate entities.
    /// - [`get_many_mut`](Self::get_many_mut) to get items using a mutable reference.
    /// - [`get_many_inner`](Self::get_many_mut_inner) to get read-only query items with the actual "inner" world lifetime.
    #[inline]
    pub fn get_many_mut_inner<const N: usize>(
        self,
        entities: [Entity; N],
    ) -> Result<[D::Item<'w, 's>; N], QueryEntityError> {
        // Verify that all entities are unique
        for i in 0..N {
            for j in 0..i {
                if entities[i] == entities[j] {
                    return Err(QueryEntityError::AliasedMutability(entities[i]));
                }
            }
        }
        // SAFETY: All entities are unique, so the results don't alias.
        unsafe { self.get_many_impl(entities) }
    }

    /// Returns the query items for the given array of [`Entity`].
    /// This consumes the [`Query`] to return results with the actual "inner" world lifetime.
    ///
    /// The returned query items are in the same order as the input.
    /// In case of a nonexisting entity or mismatched component, a [`QueryEntityError`] is returned instead.
    ///
    /// # See also
    ///
    /// - [`get_many`](Self::get_many) to get read-only query items without checking for duplicate entities.
    /// - [`get_many_mut`](Self::get_many_mut) to get items using a mutable reference.
    /// - [`get_many_mut_inner`](Self::get_many_mut_inner) to get mutable query items with the actual "inner" world lifetime.
    #[inline]
    pub fn get_many_inner<const N: usize>(
        self,
        entities: [Entity; N],
    ) -> Result<[D::Item<'w, 's>; N], QueryEntityError>
    where
        D: ReadOnlyQueryData,
    {
        // SAFETY: The query results are read-only, so they don't conflict if there are duplicate entities.
        unsafe { self.get_many_impl(entities) }
    }

    /// Returns the query items for the given array of [`Entity`].
    /// This consumes the [`Query`] to return results with the actual "inner" world lifetime.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the query data returned for the entities does not conflict,
    /// either because they are all unique or because the data is read-only.
    unsafe fn get_many_impl<const N: usize>(
        self,
        entities: [Entity; N],
    ) -> Result<[D::Item<'w, 's>; N], QueryEntityError> {
        let mut values = [(); N].map(|_| MaybeUninit::uninit());

        for (value, entity) in core::iter::zip(&mut values, entities) {
            // SAFETY: The caller asserts that the results don't alias
            let item = unsafe { self.copy_unsafe() }.get_inner(entity)?;
            *value = MaybeUninit::new(item);
        }

        // SAFETY: Each value has been fully initialized.
        Ok(values.map(|x| unsafe { x.assume_init() }))
    }

    /// Returns the query item for the given [`Entity`].
    ///
    /// In case of a nonexisting entity or mismatched component, a [`QueryEntityError`] is returned instead.
    ///
    /// This is always guaranteed to run in `O(1)` time.
    ///
    /// # Safety
    ///
    /// This function makes it possible to violate Rust's aliasing guarantees.
    /// You must make sure this call does not result in multiple mutable references to the same component.
    ///
    /// # See also
    ///
    /// - [`get_mut`](Self::get_mut) for the safe version.
    #[inline]
    pub unsafe fn get_unchecked(
        &self,
        entity: Entity,
    ) -> Result<D::Item<'_, 's>, QueryEntityError> {
        // SAFETY: The caller promises that this will not result in multiple mutable references.
        unsafe { self.reborrow_unsafe() }.get_inner(entity)
    }

    /// Returns a single read-only query item when there is exactly one entity matching the query.
    ///
    /// If the number of query items is not exactly one, a [`QuerySingleError`] is returned instead.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # use bevy_ecs::query::QuerySingleError;
    /// # #[derive(Component)]
    /// # struct PlayerScore(i32);
    /// fn player_scoring_system(query: Query<&PlayerScore>) {
    ///     match query.single() {
    ///         Ok(PlayerScore(score)) => {
    ///             println!("Score: {}", score);
    ///         }
    ///         Err(QuerySingleError::NoEntities(_)) => {
    ///             println!("Error: There is no player!");
    ///         }
    ///         Err(QuerySingleError::MultipleEntities(_)) => {
    ///             println!("Error: There is more than one player!");
    ///         }
    ///     }
    /// }
    /// # bevy_ecs::system::assert_is_system(player_scoring_system);
    /// ```
    ///
    /// # See also
    ///
    /// - [`single_mut`](Self::single_mut) to get the mutable query item.
    #[inline]
    pub fn single(&self) -> Result<ROQueryItem<'_, 's, D>, QuerySingleError> {
        self.as_readonly().single_inner()
    }

    /// Returns a single query item when there is exactly one entity matching the query.
    ///
    /// If the number of query items is not exactly one, a [`QuerySingleError`] is returned instead.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # #[derive(Component)]
    /// # struct Player;
    /// # #[derive(Component)]
    /// # struct Health(u32);
    /// #
    /// fn regenerate_player_health_system(mut query: Query<&mut Health, With<Player>>) {
    ///     let mut health = query.single_mut().expect("Error: Could not find a single player.");
    ///     health.0 += 1;
    /// }
    /// # bevy_ecs::system::assert_is_system(regenerate_player_health_system);
    /// ```
    ///
    /// # See also
    ///
    /// - [`single`](Self::single) to get the read-only query item.
    #[inline]
    pub fn single_mut(&mut self) -> Result<D::Item<'_, 's>, QuerySingleError> {
        self.reborrow().single_inner()
    }

    /// Returns a single query item when there is exactly one entity matching the query.
    /// This consumes the [`Query`] to return results with the actual "inner" world lifetime.
    ///
    /// If the number of query items is not exactly one, a [`QuerySingleError`] is returned instead.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # #[derive(Component)]
    /// # struct Player;
    /// # #[derive(Component)]
    /// # struct Health(u32);
    /// #
    /// fn regenerate_player_health_system(query: Query<&mut Health, With<Player>>) {
    ///     let mut health = query.single_inner().expect("Error: Could not find a single player.");
    ///     health.0 += 1;
    /// }
    /// # bevy_ecs::system::assert_is_system(regenerate_player_health_system);
    /// ```
    ///
    /// # See also
    ///
    /// - [`single`](Self::single) to get the read-only query item.
    /// - [`single_mut`](Self::single_mut) to get the mutable query item.
    #[inline]
    pub fn single_inner(self) -> Result<D::Item<'w, 's>, QuerySingleError> {
        let mut query = self.into_iter();
        let first = query.next();
        let extra = query.next().is_some();

        match (first, extra) {
            (Some(r), false) => Ok(r),
            (None, _) => Err(QuerySingleError::NoEntities(DebugName::type_name::<Self>())),
            (Some(_), _) => Err(QuerySingleError::MultipleEntities(DebugName::type_name::<
                Self,
            >())),
        }
    }

    /// Returns `true` if there are no query items.
    ///
    /// This is equivalent to `self.iter().next().is_none()`, and thus the worst case runtime will be `O(n)`
    /// where `n` is the number of *potential* matches. This can be notably expensive for queries that rely
    /// on non-archetypal filters such as [`Added`], [`Changed`] or [`Spawned`] which must individually check
    /// each query result for a match.
    ///
    /// # Example
    ///
    /// Here, the score is increased only if an entity with a `Player` component is present in the world:
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # #[derive(Component)]
    /// # struct Player;
    /// # #[derive(Resource)]
    /// # struct Score(u32);
    /// fn update_score_system(query: Query<(), With<Player>>, mut score: ResMut<Score>) {
    ///     if !query.is_empty() {
    ///         score.0 += 1;
    ///     }
    /// }
    /// # bevy_ecs::system::assert_is_system(update_score_system);
    /// ```
    ///
    /// [`Added`]: crate::query::Added
    /// [`Changed`]: crate::query::Changed
    /// [`Spawned`]: crate::query::Spawned
    #[inline]
    pub fn is_empty(&self) -> bool {
        // If the query data matches every entity, then `as_nop()` can safely
        // skip the cost of initializing the fetch for data that won't be used.
        if D::IS_ARCHETYPAL {
            self.as_nop().iter().next().is_none()
        } else {
            self.iter().next().is_none()
        }
    }

    /// Returns `true` if the given [`Entity`] matches the query.
    ///
    /// This is always guaranteed to run in `O(1)` time.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # #[derive(Component)]
    /// # struct InRange;
    /// #
    /// # #[derive(Resource)]
    /// # struct Target {
    /// #     entity: Entity,
    /// # }
    /// #
    /// fn targeting_system(in_range_query: Query<&InRange>, target: Res<Target>) {
    ///     if in_range_query.contains(target.entity) {
    ///         println!("Bam!")
    ///     }
    /// }
    /// # bevy_ecs::system::assert_is_system(targeting_system);
    /// ```
    #[inline]
    pub fn contains(&self, entity: Entity) -> bool {
        // If the query data matches every entity, then `as_nop()` can safely
        // skip the cost of initializing the fetch for data that won't be used.
        if D::IS_ARCHETYPAL {
            self.as_nop().get(entity).is_ok()
        } else {
            self.get(entity).is_ok()
        }
    }

    /// Counts the number of entities that match the query.
    ///
    /// This is equivalent to `self.iter().count()` but may be more efficient in some cases.
    ///
    /// If [`D::IS_ARCHETYPAL`](QueryData::IS_ARCHETYPAL) && [`F::IS_ARCHETYPAL`](QueryFilter::IS_ARCHETYPAL) is `true`,
    /// this will do work proportional to the number of matched archetypes or tables, but will not iterate each entity.
    /// If it is `false`, it will have to do work for each entity.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # #[derive(Component)]
    /// # struct InRange;
    /// #
    /// fn targeting_system(in_range_query: Query<&InRange>) {
    ///     let count = in_range_query.count();
    ///     println!("{count} targets in range!");
    /// }
    /// # bevy_ecs::system::assert_is_system(targeting_system);
    /// ```
    pub fn count(&self) -> usize {
        // If the query data matches every entity, then `as_nop()` can safely
        // skip the cost of initializing the fetch for data that won't be used.
        if !D::IS_ARCHETYPAL {
            self.into_iter().count()
        } else if !F::IS_ARCHETYPAL {
            // If we have non-archetypal filters, we have to check each entity.
            self.as_nop().into_iter().count()
        } else {
            // For archetypal queries, the `size_hint()` is exact,
            // and we can get the count from the archetype and table counts.
            self.as_nop().into_iter().size_hint().0
        }
    }
}

impl<'w, 's, D: QueryData, F: QueryFilter> IntoIterator for Query<'w, 's, D, F> {
    type Item = D::Item<'w, 's>;
    type IntoIter = QueryIter<'w, 's, D, F>;

    fn into_iter(self) -> Self::IntoIter {
        // SAFETY:
        // - `self.world` has permission to access the required components.
        // - We consume the query, so mutable queries cannot alias.
        //   Read-only queries are `Copy`, but may alias themselves.
        unsafe { QueryIter::new(self.world, self.state, self.last_run, self.this_run) }
    }
}

impl<'w, 's, D: QueryData, F: QueryFilter> IntoIterator for &'w Query<'_, 's, D, F> {
    type Item = ROQueryItem<'w, 's, D>;
    type IntoIter = QueryIter<'w, 's, D::ReadOnly, F>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'w, 's, D: QueryData, F: QueryFilter> IntoIterator for &'w mut Query<'_, 's, D, F> {
    type Item = D::Item<'w, 's>;
    type IntoIter = QueryIter<'w, 's, D, F>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<'w, 's, D: ReadOnlyQueryData, F: QueryFilter> Query<'w, 's, D, F> {
    /// Returns an [`Iterator`] over the query items, with the actual "inner" world lifetime.
    ///
    /// This can only return immutable data (mutable data will be cast to an immutable form).
    /// See [`Self::iter_mut`] for queries that contain at least one mutable component.
    ///
    /// # Example
    ///
    /// Here, the `report_names_system` iterates over the `Player` component of every entity
    /// that contains it:
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # #[derive(Component)]
    /// # struct Player { name: String }
    /// #
    /// fn report_names_system(query: Query<&Player>) {
    ///     for player in &query {
    ///         println!("Say hello to {}!", player.name);
    ///     }
    /// }
    /// # bevy_ecs::system::assert_is_system(report_names_system);
    /// ```
    #[inline]
    pub fn iter_inner(&self) -> QueryIter<'w, 's, D::ReadOnly, F> {
        (*self).into_iter()
    }
}

/// [System parameter] that provides access to single entity's components, much like [`Query::single`]/[`Query::single_mut`].
///
/// This [`SystemParam`](crate::system::SystemParam) fails validation if zero or more than one matching entity exists.
/// This will cause the system to be skipped, according to the rules laid out in [`SystemParamValidationError`](crate::system::SystemParamValidationError).
///
/// Use [`Option<Single<D, F>>`] instead if zero or one matching entities can exist.
///
/// See [`Query`] for more details.
///
/// [System parameter]: crate::system::SystemParam
///
/// # Example
/// ```
/// # use bevy_ecs::prelude::*;
/// #[derive(Component)]
/// struct Boss {
///    health: f32
/// };
///
/// fn hurt_boss(mut boss: Single<&mut Boss>) {
///    boss.health -= 4.0;
/// }
/// ```
/// Note that because [`Single`] implements [`Deref`] and [`DerefMut`], methods and fields like `health` can be accessed directly.
/// You can also access the underlying data manually, by calling `.deref`/`.deref_mut`, or by using the `*` operator.
pub struct Single<'w, 's, D: QueryData, F: QueryFilter = ()> {
    pub(crate) item: D::Item<'w, 's>,
    pub(crate) _filter: PhantomData<F>,
}

impl<'w, 's, D: QueryData, F: QueryFilter> Deref for Single<'w, 's, D, F> {
    type Target = D::Item<'w, 's>;

    fn deref(&self) -> &Self::Target {
        &self.item
    }
}

impl<'w, 's, D: QueryData, F: QueryFilter> DerefMut for Single<'w, 's, D, F> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.item
    }
}

impl<'w, 's, D: QueryData, F: QueryFilter> Single<'w, 's, D, F> {
    /// Returns the inner item with ownership.
    pub fn into_inner(self) -> D::Item<'w, 's> {
        self.item
    }
}

/// [System parameter] that works very much like [`Query`] except it always contains at least one matching entity.
///
/// This [`SystemParam`](crate::system::SystemParam) fails validation if no matching entities exist.
/// This will cause the system to be skipped, according to the rules laid out in [`SystemParamValidationError`](crate::system::SystemParamValidationError).
///
/// Much like [`Query::is_empty`] the worst case runtime will be `O(n)` where `n` is the number of *potential* matches.
/// This can be notably expensive for queries that rely on non-archetypal filters such as [`Added`](crate::query::Added),
/// [`Changed`](crate::query::Changed) of [`Spawned`](crate::query::Spawned) which must individually check each query
/// result for a match.
///
/// See [`Query`] for more details.
///
/// [System parameter]: crate::system::SystemParam
pub struct Populated<'w, 's, D: QueryData, F: QueryFilter = ()>(pub(crate) Query<'w, 's, D, F>);

impl<'w, 's, D: QueryData, F: QueryFilter> Deref for Populated<'w, 's, D, F> {
    type Target = Query<'w, 's, D, F>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<D: QueryData, F: QueryFilter> DerefMut for Populated<'_, '_, D, F> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'w, 's, D: QueryData, F: QueryFilter> IntoIterator for Populated<'w, 's, D, F> {
    type Item = <Query<'w, 's, D, F> as IntoIterator>::Item;

    type IntoIter = <Query<'w, 's, D, F> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a, 'w, 's, D: QueryData, F: QueryFilter> IntoIterator for &'a Populated<'w, 's, D, F> {
    type Item = <&'a Query<'w, 's, D, F> as IntoIterator>::Item;

    type IntoIter = <&'a Query<'w, 's, D, F> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.deref().into_iter()
    }
}

impl<'a, 'w, 's, D: QueryData, F: QueryFilter> IntoIterator for &'a mut Populated<'w, 's, D, F> {
    type Item = <&'a mut Query<'w, 's, D, F> as IntoIterator>::Item;

    type IntoIter = <&'a mut Query<'w, 's, D, F> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.deref_mut().into_iter()
    }
}

#[cfg(test)]
mod tests {
    use crate::{prelude::*, query::QueryEntityError};
    use alloc::vec::Vec;

    #[test]
    fn get_many_uniqueness() {
        let mut world = World::new();

        let entities: Vec<Entity> = (0..10).map(|_| world.spawn_empty().id()).collect();

        let mut query_state = world.query::<Entity>();

        // It's best to test get_many_mut_inner directly, as it is shared
        // We don't care about aliased mutability for the read-only equivalent

        // SAFETY: Query does not access world data.
        assert!(query_state
            .query_mut(&mut world)
            .get_many_mut_inner::<10>(entities.clone().try_into().unwrap())
            .is_ok());

        assert_eq!(
            query_state
                .query_mut(&mut world)
                .get_many_mut_inner([entities[0], entities[0]])
                .unwrap_err(),
            QueryEntityError::AliasedMutability(entities[0])
        );

        assert_eq!(
            query_state
                .query_mut(&mut world)
                .get_many_mut_inner([entities[0], entities[1], entities[0]])
                .unwrap_err(),
            QueryEntityError::AliasedMutability(entities[0])
        );

        assert_eq!(
            query_state
                .query_mut(&mut world)
                .get_many_mut_inner([entities[9], entities[9]])
                .unwrap_err(),
            QueryEntityError::AliasedMutability(entities[9])
        );
    }
}
