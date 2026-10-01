//! Mass property functionality for [rigid bodies] and [colliders].
//!
//! # Overview
//!
//! Every dynamic rigid body has [mass], [angular inertia], and a [center of mass].
//! These mass properties determine how the rigid body responds to forces and torques.
//!
//! - **Mass**: Represents resistance to linear acceleration. A higher mass requires more force for the same acceleration.
//! - **Angular Inertia**: Represents resistance to angular acceleration. A higher angular inertia requires more torque for the same angular acceleration.
//! - **Center of Mass**: The average position of mass in the body. Applying forces at this point produces no torque.
//!
//! Static and kinematic rigid bodies have infinite mass and angular inertia,
//! and do not respond to forces or torques. Zero mass for a dynamic body is also
//! treated as a special case, and corresponds to infinite mass.
//!
//! Mass properties can be set for individual entities using the [`Mass`], [`AngularInertia`],
//! and [`CenterOfMass`] components. If they are not present, mass properties are instead computed
//! automatically from attached colliders based on their shape and [`ColliderDensity`].
//!
//! If a rigid body has child entities, their mass properties are combined to compute
//! the total mass properties of the rigid body. These are stored in the [`ComputedMass`],
//! [`ComputedAngularInertia`], and [`ComputedCenterOfMass`] components, which are updated
//! automatically when mass properties are changed, or when colliders are added or removed.
//!
//! [rigid bodies]: crate::dynamics::rigid_body::RigidBody
//! [colliders]: crate::collision::collider::Collider
//! [mass]: components::Mass
//! [angular inertia]: components::AngularInertia
//! [center of mass]: components::CenterOfMass
//!
//! ## Example
//!
//! If no mass properties are set, they are computed automatically from attached colliders
//! based on their shape and density.
//!
//! ```
//! # use avian3d::prelude::*;
//! # use bevy::prelude::*;
//! #
//! # fn setup(mut commands: Commands) {
//! // Note: `ColliderDensity` is optional, and defaults to `1.0` if not present.
//! commands.spawn((
//!     RigidBody::Dynamic,
//!     Collider::capsule(0.5, 1.5),
//!     ColliderDensity(2.0),
//! ));
//! # }
//! ```
//!
//! If mass properties are set with the [`Mass`], [`AngularInertia`], and [`CenterOfMass`] components,
//! they override the values computed from colliders.
//!
//! ```
//! # use avian3d::prelude::*;
//! # use bevy::prelude::*;
//! #
//! # fn setup(mut commands: Commands) {
//! // Override mass and the center of mass, but use the collider's angular inertia.
//! commands.spawn((
//!     RigidBody::Dynamic,
//!     Collider::capsule(0.5, 1.5),
//!     Mass(5.0),
//!     CenterOfMass::new(0.0, -0.5, 0.0),
//! ));
//! # }
//! ```
//!
//! If the rigid body has child colliders, their mass properties will be combined for
//! the total [`ComputedMass`], [`ComputedAngularInertia`], and [`ComputedCenterOfMass`].
//!
//! ```
//! # use avian3d::prelude::*;
//! # use bevy::prelude::*;
//! #
//! # fn setup(mut commands: Commands) {
//! // Total mass: 10.0 + 5.0 = 15.0
//! // Total center of mass: (10.0 * [0.0, -0.5, 0.0] + 5.0 * [0.0, 4.0, 0.0]) / (10.0 + 5.0) = [0.0, 1.0, 0.0]
//! commands.spawn((
//!     RigidBody::Dynamic,
//!     Collider::capsule(0.5, 1.5),
//!     Mass(10.0),
//!     CenterOfMass::new(0.0, -0.5, 0.0),
//!     Transform::default(),
//! ))
//! .with_child((
//!     Collider::sphere(1.0),
//!     Mass(5.0),
//!     Transform::from_xyz(0.0, 4.0, 0.0),
//! ));
//! # }
//! ```
//!
//! # Computing Mass Properties for Shapes
//!
//! Mass properties of colliders and Bevy's primitive shapes can be computed using methods
//! provided by the [`ComputeMassProperties2d`] and [`ComputeMassProperties3d`] traits.
//!
//! ```
//! # use avian3d::prelude::*;
//! # use bevy::prelude::*;
//! #
//! #
//! // Compute mass properties for a capsule collider with a density of `2.0`.
//! let capsule = Collider::capsule(0.5, 1.5);
//! let mass_properties = capsule.mass_properties(2.0);
//!
//! // Compute individual mass properties for a `Circle`.
//! let circle = Circle::new(1.0);
//! let mass = circle.mass(2.0);
//! let angular_inertia = circle.angular_inertia(mass);
//! let center_of_mass = circle.center_of_mass();
//! ```
//!
//! Similarly, shapes can be used to construct the [`Mass`], [`AngularInertia`],
//! and [`CenterOfMass`] components, or the [`MassPropertiesBundle`].
//!
//! ```
//! # use avian3d::prelude::*;
//! # use bevy::prelude::*;
//! #
//! # fn setup(mut commands: Commands) {
//! // Construct individual mass properties from a collider.
//! let shape = Collider::sphere(0.5);
//! commands.spawn((
//!     RigidBody::Dynamic,
//!     Mass::from_shape(&shape, 2.0),
//!     AngularInertia::from_shape(&shape, 1.5),
//!     CenterOfMass::from_shape(&shape),
//! ));
//!
//! // Construct a `MassPropertiesBundle` from a primitive shape.
//! let shape = Sphere::new(0.5);
//! commands.spawn((RigidBody::Dynamic, MassPropertiesBundle::from_shape(&shape, 2.0)));
//! # }
//! ```
//!
//! This mass property computation functionality is provided by the [`bevy_heavy`] crate.
//!
//! # Mass Property Helper
//!
//! [`MassPropertyHelper`] is a [`SystemParam`](bevy::ecs::system::SystemParam) that provides convenient helper methods
//! that can be used to modify or compute mass properties for individual entities and hierarchies at runtime.
//!
//! For example, [`MassPropertyHelper::total_mass_properties`] computes the total mass properties of an entity,
//! taking into account the mass properties of descendants and colliders.

use crate::physics_transform::PhysicsTransformSystems;
use crate::prelude::*;
use bevy::{
    ecs::{intern::Interned, schedule::ScheduleLabel},
    prelude::*,
};

pub mod components;
use components::RecomputeMassProperties;

mod system_param;
pub use system_param::MassPropertyHelper;

/// Mass property computation with `bevy_heavy`, re-exported for your convenience.
pub use bevy_heavy;

pub(crate) use bevy_heavy::{
    ComputeMassProperties3d as ComputeMassProperties, MassProperties3d as MassProperties,
};

/// An extension trait for [`MassProperties`].
pub trait MassPropertiesExt {
    /// Converts the [`MassProperties`] to a [`MassPropertiesBundle`]
    /// containing the [`Mass`], [`AngularInertia`], and [`CenterOfMass`] components.
    fn to_bundle(&self) -> MassPropertiesBundle;
}

impl MassPropertiesExt for MassProperties {
    fn to_bundle(&self) -> MassPropertiesBundle {
        let angular_inertia = AngularInertia::new_with_local_frame(
            self.principal_angular_inertia.f32(),
            self.local_inertial_frame.f32(),
        );

        MassPropertiesBundle {
            mass: Mass(self.mass),
            angular_inertia,
            center_of_mass: CenterOfMass(self.center_of_mass),
        }
    }
}

/// A plugin for managing [mass properties] of rigid bodies.
///
/// - Updates the [`ComputedMass`], [`ComputedAngularInertia`], and [`ComputedCenterOfMass`] components
///   for rigid bodies when their mass properties are changed, or when colliders are added or removed.
/// - Logs warnings when dynamic bodies have invalid [`Mass`] or [`AngularInertia`].
///
/// [mass properties]: crate::dynamics::rigid_body::mass_properties
pub struct MassPropertyPlugin {
    schedule: Interned<dyn ScheduleLabel>,
}

impl MassPropertyPlugin {
    /// Creates a [`MassPropertyPlugin`] with the schedule that is used for running the [`PhysicsSchedule`].
    ///
    /// The default schedule is `FixedPostUpdate`.
    pub fn new(schedule: impl ScheduleLabel) -> Self {
        Self {
            schedule: schedule.intern(),
        }
    }
}

impl Default for MassPropertyPlugin {
    fn default() -> Self {
        Self::new(FixedPostUpdate)
    }
}

impl Plugin for MassPropertyPlugin {
    fn build(&self, app: &mut App) {
        // TODO: We probably don't need this since we have the observer.
        // Force mass property computation for new rigid bodies.
        app.register_required_components::<RigidBody, RecomputeMassProperties>();

        // Compute mass properties for new rigid bodies at spawn.
        app.add_observer(
            |trigger: On<Add, RigidBody>, mut mass_helper: MassPropertyHelper| {
                mass_helper.update_mass_properties(trigger.entity);
            },
        );

        // Update the mass properties of rigid bodies when colliders added or removed.
        // TODO: Avoid duplicating work with the above observer.
        app.add_observer(
            |trigger: On<Insert, RigidBodyColliders>, mut mass_helper: MassPropertyHelper| {
                mass_helper.update_mass_properties(trigger.entity);
            },
        );

        // Configure system sets for mass property computation.
        app.configure_sets(
            self.schedule,
            (
                MassPropertySystems::UpdateColliderMassProperties,
                MassPropertySystems::QueueRecomputation,
                MassPropertySystems::UpdateComputedMassProperties,
            )
                .chain()
                .in_set(PhysicsSystems::Prepare)
                .after(PhysicsTransformSystems::TransformToPosition),
        );

        // Queue mass property recomputation when mass properties are changed.
        app.add_systems(
            self.schedule,
            (
                queue_mass_recomputation_on_mass_change,
                queue_mass_recomputation_on_collider_mass_change,
            )
                .in_set(MassPropertySystems::QueueRecomputation),
        );

        // Update mass properties for entities with the `RecomputeMassProperties` component.
        app.add_systems(
            self.schedule,
            (update_mass_properties, warn_invalid_mass)
                .chain()
                .in_set(MassPropertySystems::UpdateComputedMassProperties),
        );
    }
}

/// A system set for logic related to updating mass properties.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MassPropertySystems {
    /// Update [`ColliderMassProperties`] for colliders.
    UpdateColliderMassProperties,
    /// Adds the [`RecomputeMassProperties`] component to entities with changed mass properties.
    QueueRecomputation,
    /// Update [`ComputedMass`], [`ComputedAngularInertia`], and [`ComputedCenterOfMass`]
    /// for entities with the [`RecomputeMassProperties`] component. The component is removed after updating.
    UpdateComputedMassProperties,
}

/// A query filter for entities with [`ComputedMass`], [`ComputedAngularInertia`], or [`ComputedCenterOfMass`].
pub type WithComputedMassProperty = Or<(
    With<ComputedMass>,
    With<ComputedAngularInertia>,
    With<ComputedCenterOfMass>,
)>;

/// A query filter for entities with changed [`Mass`], [`AngularInertia`], or [`CenterOfMass`].
pub type MassPropertyChanged = Or<(
    Changed<Mass>,
    Changed<AngularInertia>,
    Changed<CenterOfMass>,
)>;

/// Queues mass property recomputation for rigid bodies when their [`Mass`], [`AngularInertia`],
/// or [`CenterOfMass`] components are changed.
///
/// Colliders attached to rigid bodies are excluded, as they are handled by
/// [`queue_mass_recomputation_on_collider_mass_change`].
fn queue_mass_recomputation_on_mass_change(
    mut commands: Commands,
    mut query: Query<
        Entity,
        (
            WithComputedMassProperty,
            Without<ColliderOf>,
            MassPropertyChanged,
        ),
    >,
) {
    for entity in &mut query {
        commands.entity(entity).insert(RecomputeMassProperties);
    }
}

/// Queues mass property recomputation for rigid bodies when the [`ColliderMassProperties`],
/// [`Mass`], [`AngularInertia`], or [`CenterOfMass`] components of their colliders are changed.
fn queue_mass_recomputation_on_collider_mass_change(
    mut commands: Commands,
    mut query: Query<
        &ColliderOf,
        Or<(
            Changed<ColliderMassProperties>,
            Changed<ColliderTransform>,
            MassPropertyChanged,
        )>,
    >,
) {
    for &ColliderOf { body } in &mut query {
        if let Ok(mut entity_commands) = commands.get_entity(body) {
            entity_commands.insert(RecomputeMassProperties);
        }
    }
}

fn update_mass_properties(
    mut commands: Commands,
    query: Query<Entity, With<RecomputeMassProperties>>,
    mut mass_helper: MassPropertyHelper,
) {
    // TODO: Parallelize mass property updates.
    for entity in query.iter() {
        mass_helper.update_mass_properties(entity);
        commands.entity(entity).remove::<RecomputeMassProperties>();
    }
}

/// Logs warnings when dynamic bodies have invalid [`Mass`] or [`AngularInertia`].
fn warn_invalid_mass(
    mut bodies: Query<
        (
            Entity,
            &RigidBody,
            Ref<ComputedMass>,
            Ref<ComputedAngularInertia>,
        ),
        Or<(Changed<ComputedMass>, Changed<ComputedAngularInertia>)>,
    >,
) {
    for (entity, rb, mass, inertia) in &mut bodies {
        let is_mass_valid = mass.is_finite();
        let is_inertia_valid = inertia.is_finite();

        // Warn about dynamic bodies with no mass or inertia
        if rb.is_dynamic() && !(is_mass_valid && is_inertia_valid) {
            warn!(
                "Dynamic rigid body {:?} has no mass or inertia. This can cause NaN values. Consider adding a `MassPropertiesBundle` or a `Collider` with mass.",
                entity
            );
        }
    }
}
