//! Components, traits, and plugins related to collider functionality.

use crate::prelude::*;
use bevy::{
    ecs::{
        component::Mutable,
        system::{ReadOnlySystemParam, SystemParam, SystemParamItem},
    },
    prelude::*,
};

mod backend;

pub use backend::{ColliderBackendPlugin, ColliderMarker};

mod cache;
pub use cache::ColliderCachePlugin;
pub mod collider_hierarchy;
pub mod collider_transform;

mod layers;
pub use layers::*;

/// The default [`Collider`] that uses Parry.
mod parry;
pub use parry::*;

/// A trait for creating colliders from other types.
pub trait IntoCollider<C: AnyCollider> {
    /// Creates a collider from `self`.
    fn collider(&self) -> C;
}

/// Context necessary to calculate [`ColliderAabb`]s for an [`AnyCollider`]
#[derive(Deref)]
pub struct AabbContext<'a, 'w, 's, T: ReadOnlySystemParam> {
    #[deref]
    item: &'a SystemParamItem<'w, 's, T>,
}

impl<T: ReadOnlySystemParam> Clone for AabbContext<'_, '_, '_, T> {
    fn clone(&self) -> Self {
        Self { item: self.item }
    }
}

impl<'a, 'w, 's, T: ReadOnlySystemParam> AabbContext<'a, 'w, 's, T> {
    /// Construct an [`AabbContext`]
    pub fn new(item: &'a <T as SystemParam>::Item<'w, 's>) -> Self {
        Self { item }
    }
}

impl AabbContext<'_, '_, '_, ()> {
    fn fake() -> Self {
        Self { item: &() }
    }
}

/// Context necessary to calculate [`ContactManifold`]s for a set of [`AnyCollider`]
#[derive(Deref)]
pub struct ContactManifoldContext<'a, 'w, 's, T: ReadOnlySystemParam> {
    #[deref]
    item: &'a SystemParamItem<'w, 's, T>,
}

impl<'a, 'w, 's, T: ReadOnlySystemParam> ContactManifoldContext<'a, 'w, 's, T> {
    /// Construct a [`ContactManifoldContext`]
    pub fn new(item: &'a <T as SystemParam>::Item<'w, 's>) -> Self {
        Self { item }
    }
}

impl ContactManifoldContext<'_, '_, '_, ()> {
    fn fake() -> Self {
        Self { item: &() }
    }
}

/// A trait that generalizes over colliders. Implementing this trait
/// allows colliders to be used with the physics engine.
pub trait AnyCollider: Component<Mutability = Mutable> + ComputeMassProperties {
    /// A type providing additional context for collider operations.
    ///
    /// `Context` allows you to access an arbitrary [`ReadOnlySystemParam`] on
    /// the world, for context-sensitive behavior in collider operations.
    ///
    /// # Example
    ///
    /// ```
    /// # use avian3d::{prelude::*, math::{Vector, Scalar}};
    /// # use bevy::prelude::*;
    /// # use bevy::ecs::system::{SystemParam, lifetimeless::SRes};
    /// #
    /// #[derive(Component)]
    /// pub struct VoxelCollider;
    ///
    /// # impl ComputeMassProperties2d for VoxelCollider {
    /// #     fn mass(&self, density: f32) -> f32 {0.}
    /// #     fn unit_angular_inertia(&self) -> f32 { 0.}
    /// #     fn center_of_mass(&self) -> Vec2 { Vec2::ZERO }
    /// # }
    /// #
    /// # impl ComputeMassProperties3d for VoxelCollider {
    /// #     fn mass(&self, density: f32) -> f32 {0.}
    /// #     fn unit_principal_angular_inertia(&self) -> Vec3 { Vec3::ZERO }
    /// #     fn center_of_mass(&self) -> Vec3 { Vec3::ZERO }
    /// # }
    /// #
    /// impl AnyCollider for VoxelCollider {
    ///     // any read-only system param
    ///     type Context = SRes<Time>;
    ///
    /// #   fn aabb_with_context(
    /// #       &self,
    /// #       _: Vector,
    /// #       _: impl Into<Rotation>,
    /// #       _: AabbContext<Self::Context>,
    /// #   ) -> ColliderAabb { unimplemented!() }
    /// #
    ///     fn contact_manifolds_with_context(
    ///         &self,
    ///         other: &Self,
    ///         position1: Vector,
    ///         rotation1: impl Into<Rotation>,
    ///         position2: Vector,
    ///         rotation2: impl Into<Rotation>,
    ///         prediction_distance: Scalar,
    ///         manifolds: &mut Vec<ContactManifold>,
    ///         context: ContactManifoldContext<Self::Context>,
    ///     ) {
    ///         let elapsed = context.elapsed();
    ///         // do some computation...
    /// #       unimplemented!()
    ///     }
    /// }
    /// ```
    type Context: for<'w, 's> ReadOnlySystemParam<Item<'w, 's>: Send + Sync>;

    /// Computes the [Axis-Aligned Bounding Box](ColliderAabb) of the collider
    /// with the given position and rotation.
    ///
    /// See [`SimpleCollider::aabb`] for collider types with empty [`AnyCollider::Context`]
    fn aabb_with_context(
        &self,
        position: Vector,
        rotation: impl Into<Rotation>,
        context: AabbContext<Self::Context>,
    ) -> ColliderAabb;

    /// Computes the swept [Axis-Aligned Bounding Box](ColliderAabb) of the collider.
    /// This corresponds to the space the shape would occupy if it moved from the given
    /// start position to the given end position.
    ///
    /// See [`SimpleCollider::swept_aabb`] for collider types with empty [`AnyCollider::Context`]
    fn swept_aabb_with_context(
        &self,
        start_position: Vector,
        start_rotation: impl Into<Rotation>,
        end_position: Vector,
        end_rotation: impl Into<Rotation>,
        context: AabbContext<Self::Context>,
    ) -> ColliderAabb {
        self.aabb_with_context(start_position, start_rotation, context.clone())
            .merged(self.aabb_with_context(end_position, end_rotation, context))
    }

    /// Computes all [`ContactManifold`]s between two colliders.
    ///
    /// Returns an empty vector if the colliders are separated by a distance greater than `prediction_distance`
    /// or if the given shapes are invalid.
    ///
    /// See [`SimpleCollider::contact_manifolds`] for collider types with empty [`AnyCollider::Context`]
    fn contact_manifolds_with_context(
        &self,
        other: &Self,
        position1: Vector,
        rotation1: impl Into<Rotation>,
        position2: Vector,
        rotation2: impl Into<Rotation>,
        prediction_distance: Scalar,
        manifolds: &mut Vec<ContactManifold>,
        context: ContactManifoldContext<Self::Context>,
    );
}

/// A simplified wrapper around [`AnyCollider`] that doesn't require passing in the context for
/// implementations that don't need context
pub trait SimpleCollider: AnyCollider<Context = ()> {
    /// Computes the [Axis-Aligned Bounding Box](ColliderAabb) of the collider
    /// with the given position and rotation.
    ///
    /// See [`AnyCollider::aabb_with_context`] for collider types with non-empty [`AnyCollider::Context`]
    fn aabb(&self, position: Vector, rotation: impl Into<Rotation>) -> ColliderAabb {
        self.aabb_with_context(position, rotation, AabbContext::fake())
    }

    /// Computes the swept [Axis-Aligned Bounding Box](ColliderAabb) of the collider.
    /// This corresponds to the space the shape would occupy if it moved from the given
    /// start position to the given end position.
    ///
    /// See [`AnyCollider::swept_aabb_with_context`] for collider types with non-empty [`AnyCollider::Context`]
    fn swept_aabb(
        &self,
        start_position: Vector,
        start_rotation: impl Into<Rotation>,
        end_position: Vector,
        end_rotation: impl Into<Rotation>,
    ) -> ColliderAabb {
        self.swept_aabb_with_context(
            start_position,
            start_rotation,
            end_position,
            end_rotation,
            AabbContext::fake(),
        )
    }

    /// Computes all [`ContactManifold`]s between two colliders, writing the results into `manifolds`.
    ///
    /// `manifolds` is cleared if the colliders are separated by a distance greater than `prediction_distance`
    /// or if the given shapes are invalid.
    ///
    /// See [`AnyCollider::contact_manifolds_with_context`] for collider types with non-empty [`AnyCollider::Context`]
    fn contact_manifolds(
        &self,
        other: &Self,
        position1: Vector,
        rotation1: impl Into<Rotation>,
        position2: Vector,
        rotation2: impl Into<Rotation>,
        prediction_distance: Scalar,
        manifolds: &mut Vec<ContactManifold>,
    ) {
        self.contact_manifolds_with_context(
            other,
            position1,
            rotation1,
            position2,
            rotation2,
            prediction_distance,
            manifolds,
            ContactManifoldContext::fake(),
        )
    }
}

impl<C: AnyCollider<Context = ()>> SimpleCollider for C {}

/// A trait for colliders that support scaling.
pub trait ScalableCollider: AnyCollider {
    /// Returns the global scaling factor of the collider.
    fn scale(&self) -> Vector;

    /// Sets the global scaling factor of the collider.
    ///
    /// If the scaling factor is not uniform and the resulting scaled shape
    /// can not be represented exactly, the given `detail` is used for an approximation.
    fn set_scale(&mut self, scale: Vector, detail: u32);
}

/// A marker component that indicates that a [collider](Collider) is disabled
/// and should not detect collisions or be included in spatial queries.
///
/// This is useful for temporarily disabling a collider without removing it from the world.
/// To re-enable the collider, simply remove this component.
///
/// Note that a disabled collider will still contribute to the mass properties of the rigid body
/// it is attached to. Set the [`Mass`] of the collider to zero to prevent this.
///
/// [`ColliderDisabled`] only applies to the entity it is attached to, not its children.
///
/// # Example
///
/// ```
/// # use avian3d::prelude::*;
/// # use bevy::prelude::*;
/// #
/// #[derive(Component)]
/// pub struct Character;
///
/// /// Disables colliders for all rigid body characters, for example during cutscenes.
/// fn disable_character_colliders(
///     mut commands: Commands,
///     query: Query<Entity, (With<RigidBody>, With<Character>)>,
/// ) {
///     for entity in &query {
///         commands.entity(entity).insert(ColliderDisabled);
///     }
/// }
///
/// /// Enables colliders for all rigid body characters.
/// fn enable_character_colliders(
///     mut commands: Commands,
///     query: Query<Entity, (With<RigidBody>, With<Character>)>,
/// ) {
///     for entity in &query {
///         commands.entity(entity).remove::<ColliderDisabled>();
///     }
/// }
/// ```
///
/// # Related Components
///
/// - [`RigidBodyDisabled`]: Disables a rigid body.
#[derive(Clone, Copy, Component, Debug, Default)]
pub struct ColliderDisabled;

/// A component that marks a [`Collider`] as a sensor, also known as a trigger.
///
/// Sensor colliders register intersections, but allow other bodies to pass through them. This is often used to detect when something enters
/// or leaves an area or is intersecting some shape.
///
/// Sensor colliders do *not* contribute to the mass properties of rigid bodies.
///
/// # Example
///
/// ```
/// use avian3d::prelude::*;
/// use bevy::prelude::*;
///
/// fn setup(mut commands: Commands) {
///     // Spawn a static body with a sensor collider.
///     // Other bodies will pass through, but it will still send collision events.
///     commands.spawn((RigidBody::Static, Collider::sphere(0.5), Sensor));
/// }
/// ```
#[doc(alias = "Trigger")]
#[derive(Clone, Component, Debug, Default, PartialEq, Eq)]
pub struct Sensor;

/// The Axis-Aligned Bounding Box of a [collider](Collider) in world space.
///
/// This is updated automatically.
#[derive(Clone, Copy, Component, Debug, PartialEq)]
pub struct ColliderAabb {
    /// The minimum point of the AABB.
    pub min: Vector,
    /// The maximum point of the AABB.
    pub max: Vector,
}

impl Default for ColliderAabb {
    fn default() -> Self {
        ColliderAabb::INVALID
    }
}

impl ColliderAabb {
    /// An invalid [`ColliderAabb`] that represents an empty AABB.
    pub const INVALID: Self = Self {
        min: Vector::INFINITY,
        max: Vector::NEG_INFINITY,
    };

    /// Merges this AABB with another one.
    #[inline(always)]
    pub fn merged(self, other: Self) -> Self {
        ColliderAabb {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    /// Increases the size of the bounding volume in each direction by the given amount.
    #[inline(always)]
    pub fn grow(&self, amount: Vector) -> Self {
        let b = Self {
            min: self.min - amount,
            max: self.max + amount,
        };
        debug_assert!(b.min.cmple(b.max).all());
        b
    }

    /// Checks if `self` intersects with `other`.
    #[inline(always)]
    pub fn intersects(&self, other: &Self) -> bool {
        let x_overlaps = self.min.x <= other.max.x && self.max.x >= other.min.x;
        let y_overlaps = self.min.y <= other.max.y && self.max.y >= other.min.y;
        let z_overlaps = self.min.z <= other.max.z && self.max.z >= other.min.z;
        x_overlaps && y_overlaps && z_overlaps
    }

    /// Checks if `self` contains `other`.
    #[inline(always)]
    pub fn contains(&self, other: &Self) -> bool {
        self.min.cmple(other.min).all() && self.max.cmpge(other.max).all()
    }
}

impl From<ColliderAabb> for obvhs::aabb::Aabb {
    fn from(value: ColliderAabb) -> Self {
        Self {
            min: value.min.f32().to_array().into(),
            max: value.max.f32().to_array().into(),
        }
    }
}

/// An Axis-Aligned Bounding Box that contains the [`ColliderAabb`] with an additional margin.
///
/// This is used to avoid updating the Bounding Volume Hierarchy acceleration structure
/// every time a collider moves only a small amount.
///
/// The enlarged AABB is updated automatically whenever the [`ColliderAabb`]
/// moves beyond the bounds of the current enlarged AABB.
#[derive(Clone, Copy, Component, Debug, Default, Deref, PartialEq)]
pub struct EnlargedAabb(ColliderAabb);

impl EnlargedAabb {
    /// Updates the enlarged AABB with the given [`ColliderAabb`] and margin.
    ///
    /// If the AABB is already contained within the enlarged AABB, nothing happens.
    ///
    /// Returns `true` if the AABB was updated.
    pub fn update(&mut self, aabb: &ColliderAabb, margin: Scalar) -> bool {
        if self.contains(aabb) {
            return false;
        }

        let margin = Vector::splat(margin);
        self.0.min = aabb.min - margin;
        self.0.max = aabb.max + margin;

        true
    }

    /// Gets the [`ColliderAabb`] of the enlarged AABB.
    pub fn get(&self) -> ColliderAabb {
        self.0
    }
}
