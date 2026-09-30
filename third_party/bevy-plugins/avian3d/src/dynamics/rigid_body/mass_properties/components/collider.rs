use super::super::MassProperties;
use bevy::prelude::*;
use derive_more::derive::From;

/// The density of a [`Collider`], used for computing [`ColliderMassProperties`].
/// Defaults to `1.0`.
///
/// If the entity has the [`Mass`] component, it will be used instead of the collider's mass.
///
/// # Example
///
/// ```
/// use avian3d::prelude::*;
/// use bevy::prelude::*;
///
/// // Spawn a body with a collider that has a density of `2.5`.
/// fn setup(mut commands: Commands) {
///     commands.spawn((
///         RigidBody::Dynamic,
///         Collider::sphere(0.5),
///         ColliderDensity(2.5),
///     ));
/// }
/// ```
///
/// [`Collider`]: crate::prelude::Collider
/// [`Mass`]: crate::prelude::Mass
#[derive(Reflect, Clone, Copy, Component, Debug, Deref, DerefMut, PartialEq, PartialOrd, From)]
#[reflect(Debug, Component, PartialEq)]
pub struct ColliderDensity(pub f32);

impl Default for ColliderDensity {
    fn default() -> Self {
        Self(1.0)
    }
}

/// A read-only component for the mass properties of a [`Collider`].
/// Computed automatically from the collider's shape and [`ColliderDensity`].
///
/// If the entity has the [`Mass`], [`AngularInertia`], or [`CenterOfMass`] components,
/// they will be used instead when updating the associated rigid body's [`ComputedMass`],
/// [`ComputedAngularInertia`], and [`ComputedCenterOfMass`] components respectively.
///
/// # Example
///
/// ```no_run
/// use avian3d::prelude::*;
/// use bevy::prelude::*;
///
/// fn setup(mut commands: Commands) {
///     commands.spawn((RigidBody::Dynamic, Collider::sphere(0.5)));
/// }
///
/// fn print_collider_masses(query: Query<&ColliderMassProperties>) {
///     for mass_properties in &query {
///         println!("{}", mass_properties.mass);
///     }
/// }
/// ```
///
/// [`Collider`]: crate::prelude::Collider
/// [`Mass`]: crate::prelude::Mass
/// [`AngularInertia`]: crate::prelude::AngularInertia
/// [`CenterOfMass`]: crate::prelude::CenterOfMass
/// [`ComputedMass`]: crate::prelude::ComputedMass
/// [`ComputedAngularInertia`]: crate::prelude::ComputedAngularInertia
/// [`ComputedCenterOfMass`]: crate::prelude::ComputedCenterOfMass
#[derive(Reflect, Clone, Copy, Component, Debug, Default, Deref, PartialEq, From)]
#[reflect(Debug, Component, PartialEq)]
pub struct ColliderMassProperties(MassProperties);

impl ColliderMassProperties {
    /// The collider has no mass.
    pub const ZERO: Self = Self(MassProperties::ZERO);
}
