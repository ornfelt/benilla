#![allow(clippy::unnecessary_cast)]

use crate::prelude::*;
use bevy::prelude::*;
use derive_more::From;

use super::{AngularInertia, AngularInertiaError, CenterOfMass, Mass, MassError};

/// The total [mass] computed for a dynamic [rigid body], taking into account
/// colliders and descendants. Represents resistance to linear acceleration.
///
/// The total mass is computed as the sum of the masses of all attached colliders
/// and the mass of the rigid body entity itself. The mass of an entity is determined
/// by its [`Mass`] component, or if it is not present, from an attached [`Collider`]
/// based on its shape and [`ColliderDensity`].
///
/// A total mass of zero is a special case, and is interpreted as infinite mass, meaning the rigid body
/// will not be affected by any forces.
///
/// [mass]: https://en.wikipedia.org/wiki/Mass
/// [rigid body]: RigidBody
///
/// # Representation
///
/// Internally, the total mass is actually stored as the inverse mass `1.0 / mass`.
/// This is because most physics calculations operate on the inverse mass, and storing it directly
/// allows for fewer divisions and guards against division by zero.
///
/// When using [`ComputedMass`], you shouldn't need to worry about this internal representation.
/// The provided constructors and getters abstract away the implementation details.
///
/// In terms of performance, the main thing to keep in mind is that [`inverse`](Self::inverse) is a no-op
/// and [`value`](Self::value) contains a division. When dividing by the mass, it's better to use
/// `foo * mass.inverse()` than `foo / mass.value()`.
///
/// # Related Types
///
/// - [`Mass`] can be used to set the mass associated with an individual entity.
/// - [`ComputedAngularInertia`] stores the total angular inertia of a rigid body, taking into account colliders and descendants.
/// - [`ComputedCenterOfMass`] stores the total center of mass of a rigid body, taking into account colliders and descendants.
/// - [`MassPropertyHelper`] is a [`SystemParam`] with utilities for computing and updating mass properties.
///
/// [`SystemParam`]: bevy::ecs::system::SystemParam
#[derive(Reflect, Clone, Copy, Component, Debug, Default, PartialEq)]
#[reflect(Debug, Component, Default, PartialEq)]
pub struct ComputedMass {
    /// The inverse mass.
    ///
    /// This is stored as an inverse because most physics calculations
    /// operate on the inverse mass, and storing it directly allows for
    /// fewer divisions and guards against division by zero.
    inverse: Scalar,
}

impl ComputedMass {
    /// Infinite mass.
    pub const INFINITY: Self = Self { inverse: 0.0 };

    /// Creates a new [`ComputedMass`] from the given mass.
    ///
    /// # Panics
    ///
    /// Panics if the mass is negative or NaN when `debug_assertions` are enabled.
    #[inline]
    pub fn new(mass: Scalar) -> Self {
        Self::from_inverse(mass.recip_or_zero())
    }

    /// Tries to create a new [`ComputedMass`] from the given mass.
    ///
    /// # Errors
    ///
    /// Returns [`Err(MassError)`](MassError) if the mass is negative or NaN.
    #[inline]
    pub fn try_new(mass: Scalar) -> Result<Self, MassError> {
        if mass.is_nan() {
            Err(MassError::NaN)
        } else if mass < 0.0 {
            Err(MassError::Negative)
        } else {
            Ok(Self::from_inverse(mass.recip_or_zero()))
        }
    }

    /// Creates a new [`ComputedMass`] from the given inverse mass.
    ///
    /// # Panics
    ///
    /// Panics if the inverse mass is negative or NaN when `debug_assertions` are enabled.
    #[inline]
    pub fn from_inverse(inverse_mass: Scalar) -> Self {
        debug_assert!(
            inverse_mass >= 0.0 && !inverse_mass.is_nan(),
            "mass must be positive or zero"
        );

        Self {
            inverse: inverse_mass,
        }
    }

    /// Returns the mass. If it is infinite, returns zero.
    ///
    /// Note that this involves a division because [`ComputedMass`] internally stores the inverse mass.
    /// If dividing by the mass, consider using `foo * mass.inverse()` instead of `foo / mass.value()`.
    #[inline]
    pub fn value(self) -> Scalar {
        self.inverse.recip_or_zero()
    }

    /// Returns the inverse mass.
    ///
    /// This is a no-op because [`ComputedMass`] internally stores the inverse mass.
    #[inline]
    pub fn inverse(self) -> Scalar {
        self.inverse
    }

    /// Sets the mass.
    #[inline]
    pub fn set(&mut self, mass: impl Into<ComputedMass>) {
        *self = mass.into();
    }

    /// Returns `true` if the mass is neither infinite nor NaN.
    #[inline]
    pub fn is_finite(self) -> bool {
        !self.is_infinite() && !self.is_nan()
    }

    /// Returns `true` if the mass is positive infinity or negative infinity.
    #[inline]
    pub fn is_infinite(self) -> bool {
        self == Self::INFINITY
    }

    /// Returns `true` if the mass is NaN.
    #[inline]
    pub fn is_nan(self) -> bool {
        self.inverse.is_nan()
    }
}

impl From<Scalar> for ComputedMass {
    fn from(mass: Scalar) -> Self {
        Self::new(mass)
    }
}

impl From<Mass> for ComputedMass {
    fn from(mass: Mass) -> Self {
        ComputedMass::new(mass.0 as Scalar)
    }
}

impl From<ComputedMass> for Mass {
    fn from(mass: ComputedMass) -> Self {
        Self(mass.value() as f32)
    }
}

/// The total local [angular inertia] computed for a dynamic [rigid body] as a 3x3 [tensor] matrix,
/// taking into account colliders and descendants. Represents resistance to angular acceleration.
///
/// The total angular inertia is computed as the sum of the inertias of all attached colliders
/// and the angular inertia of the rigid body entity itself, taking into account offsets from the center of mass.
/// The angular inertia of an entity is determined by its [`AngularInertia`] component, or if it is not present,
/// from an attached [`Collider`] based on its shape and mass.
///
/// This is computed in local space, so the object's orientation is not taken into account.
/// The world-space version can be computed using the associated [`rotated`](Self::rotated) method.
///
/// To manually compute the world-space version that takes the body's rotation into account,
/// use the associated [`rotated`](Self::rotated) method.
///
/// A total angular inertia of zero is a special case, and is interpreted as infinite angular inertia,
/// meaning the rigid body will not be affected by any torques.
///
/// [angular inertia]: https://en.wikipedia.org/wiki/Moment_of_inertia
/// [tensor]: https://en.wikipedia.org/wiki/Moment_of_inertia#Inertia_tensor
/// [rigid body]: RigidBody
///
/// # Representation
///
/// Internally, the angular inertia is actually stored as the inverse angular inertia tensor `angular_inertia_matrix.inverse()`.
/// This is because most physics calculations operate on the inverse angular inertia, and storing it directly
/// allows for fewer inversions and guards against division by zero.
///
/// When using [`ComputedAngularInertia`], you shouldn't need to worry about this internal representation.
/// The provided constructors and getters abstract away the implementation details.
///
/// In terms of performance, the main thing to keep in mind is that [`inverse`](Self::inverse) is a no-op
/// and [`value`](Self::value) contains an inversion. When multiplying by the inverse angular inertia, it's better to use
/// `angular_inertia.inverse() * foo` than `angular_inertia.value().inverse() * foo`.
///
/// # Related Types
///
/// - [`AngularInertia`] can be used to set the local angular inertia associated with an individual entity.
/// - [`ComputedMass`] stores the total mass of a rigid body, taking into account colliders and descendants.
/// - [`ComputedCenterOfMass`] stores the total center of mass of a rigid body, taking into account colliders and descendants.
/// - [`MassPropertyHelper`] is a [`SystemParam`] with utilities for computing and updating mass properties.
///
/// [`SystemParam`]: bevy::ecs::system::SystemParam
#[derive(Reflect, Clone, Copy, Component, Debug, PartialEq)]
#[reflect(Debug, Component, PartialEq)]
#[doc(alias = "ComputedMomentOfInertia")]
pub struct ComputedAngularInertia {
    // TODO: The matrix should be symmetric and positive definite.
    //       We could add a custom `SymmetricMat3` type to enforce symmetricity and reduce memory usage.
    inverse: SymmetricMatrix,
}

impl Default for ComputedAngularInertia {
    fn default() -> Self {
        Self::INFINITY
    }
}

impl ComputedAngularInertia {
    /// Infinite angular inertia.
    pub const INFINITY: Self = Self {
        inverse: SymmetricMatrix::ZERO,
    };

    /// Creates a new [`ComputedAngularInertia`] from the given principal angular inertia.
    ///
    /// The principal angular inertia represents the torque needed for a desired angular acceleration
    /// about the local coordinate axes.
    ///
    /// Note that this involves an invertion because [`ComputedAngularInertia`] internally stores the inverse angular inertia.
    ///
    /// To specify the orientation of the local inertial frame, consider using [`ComputedAngularInertia::new_with_local_frame`].
    ///
    /// # Panics
    ///
    /// Panics if any component of the principal angular inertia is negative or NaN when `debug_assertions` are enabled.
    #[inline]
    #[doc(alias = "from_principal_angular_inertia")]
    pub fn new(principal_angular_inertia: Vector) -> Self {
        debug_assert!(
            principal_angular_inertia.cmpge(Vector::ZERO).all()
                && !principal_angular_inertia.is_nan(),
            "principal angular inertia must be positive or zero for all axes"
        );

        Self::from_inverse_tensor(SymmetricMatrix::from_diagonal(
            principal_angular_inertia.recip_or_zero(),
        ))
    }

    /// Tries to create a new [`ComputedAngularInertia`] from the given principal angular inertia.
    ///
    /// The principal angular inertia represents the torque needed for a desired angular acceleration
    /// about the local coordinate axes.
    ///
    /// Note that this involves an invertion because [`ComputedAngularInertia`] internally stores the inverse angular inertia.
    ///
    /// # Errors
    ///
    /// Returns [`Err(AngularInertiaError)`](AngularInertiaError) if any component of the principal angular inertia is negative or NaN.
    #[inline]
    pub fn try_new(principal_angular_inertia: Vector) -> Result<Self, AngularInertiaError> {
        if principal_angular_inertia.is_nan() {
            Err(AngularInertiaError::NaN)
        } else if !principal_angular_inertia.cmpge(Vector::ZERO).all() {
            Err(AngularInertiaError::Negative)
        } else {
            Ok(Self::from_inverse_tensor(SymmetricMatrix::from_diagonal(
                principal_angular_inertia.recip_or_zero(),
            )))
        }
    }

    /// Creates a new [`ComputedAngularInertia`] from the given principal angular inertia
    /// and the orientation of the local inertial frame.
    ///
    /// The principal angular inertia represents the torque needed for a desired angular acceleration
    /// about the local coordinate axes defined by the given `orientation`.
    ///
    /// Note that this involves an invertion because [`ComputedAngularInertia`] internally stores the inverse angular inertia.
    ///
    /// # Panics
    ///
    /// Panics if any component of the principal angular inertia is negative or NaN when `debug_assertions` are enabled.
    #[inline]
    #[doc(alias = "from_principal_angular_inertia_with_local_frame")]
    pub fn new_with_local_frame(
        principal_angular_inertia: Vector,
        orientation: Quaternion,
    ) -> Self {
        debug_assert!(
            principal_angular_inertia.cmpge(Vector::ZERO).all()
                && !principal_angular_inertia.is_nan(),
            "principal angular inertia must be positive or zero for all axes"
        );

        Self::from_inverse_tensor(SymmetricMatrix::from_mat3_unchecked(
            Matrix::from_quat(orientation)
                * Matrix::from_diagonal(principal_angular_inertia.recip_or_zero())
                * Matrix::from_quat(orientation.inverse()),
        ))
    }

    /// Creates a new [`ComputedAngularInertia`] from the given angular inertia tensor.
    ///
    /// The tensor should be symmetric and positive definite.
    ///
    /// Note that this involves an invertion because [`ComputedAngularInertia`] internally stores the inverse angular inertia.
    #[inline]
    #[doc(alias = "from_mat3")]
    pub fn from_tensor(tensor: SymmetricMatrix) -> Self {
        Self::from_inverse_tensor(tensor.inverse_or_zero())
    }

    /// Creates a new [`ComputedAngularInertia`] from the given inverse angular inertia tensor.
    ///
    /// The tensor should be symmetric and positive definite.
    #[inline]
    #[doc(alias = "from_inverse_mat3")]
    pub fn from_inverse_tensor(inverse_tensor: SymmetricMatrix) -> Self {
        Self {
            inverse: inverse_tensor,
        }
    }

    /// Returns the angular inertia tensor. If it is infinite, returns zero.
    ///
    /// Note that this involves an invertion because [`ComputedAngularInertia`] internally stores the inverse angular inertia.
    /// If multiplying by the inverse angular inertia, consider using `angular_inertia.inverse() * foo`
    /// instead of `angular_inertia.value().inverse() * foo`.
    ///
    /// Equivalent to [`ComputedAngularInertia::tensor`].
    #[inline]
    pub fn value(self) -> SymmetricMatrix {
        self.tensor()
    }

    /// Returns the inverse of the angular inertia tensor.
    ///
    /// Note that this is a no-op because [`ComputedAngularInertia`] internally stores the inverse angular inertia.
    ///
    /// Equivalent to [`ComputedAngularInertia::inverse_tensor`].
    #[inline]
    pub fn inverse(self) -> SymmetricMatrix {
        self.inverse_tensor()
    }

    /// Returns the angular inertia tensor.
    ///
    /// Note that this involves an invertion because [`ComputedAngularInertia`] internally stores the inverse angular inertia.
    /// If multiplying by the inverse angular inertia, consider using `angular_inertia.inverse() * foo`
    /// instead of `angular_inertia.value().inverse() * foo`.
    #[inline]
    #[doc(alias = "as_mat3")]
    pub fn tensor(self) -> SymmetricMatrix {
        self.inverse.inverse_or_zero()
    }

    /// Returns the inverse of the angular inertia tensor.
    ///
    /// Note that this is a no-op because [`ComputedAngularInertia`] internally stores the inverse angular inertia.
    #[inline]
    #[doc(alias = "as_inverse_mat3")]
    pub fn inverse_tensor(self) -> SymmetricMatrix {
        self.inverse
    }

    /// Computes the angular inertia tensor with the given rotation.
    ///
    /// This can be used to transform local angular inertia to world space.
    #[inline]
    pub fn rotated(self, rotation: Quaternion) -> Self {
        let rot_mat3 = Matrix::from_quat(rotation);
        Self::from_inverse_tensor(SymmetricMatrix::from_mat3_unchecked(
            (rot_mat3 * self.inverse) * rot_mat3.transpose(),
        ))
    }

    /// Returns `true` if the angular inertia is neither infinite nor NaN.
    #[inline]
    pub fn is_finite(self) -> bool {
        !self.is_infinite() && !self.is_nan()
    }

    /// Returns `true` if the angular inertia is positive infinity or negative infinity.
    #[inline]
    pub fn is_infinite(self) -> bool {
        self == Self::INFINITY
    }

    /// Returns `true` if the angular inertia is NaN.
    #[inline]
    pub fn is_nan(self) -> bool {
        self.inverse.is_nan()
    }
}

impl From<SymmetricMatrix> for ComputedAngularInertia {
    fn from(tensor: SymmetricMatrix) -> Self {
        Self::from_tensor(tensor)
    }
}

impl From<AngularInertia> for ComputedAngularInertia {
    fn from(inertia: AngularInertia) -> Self {
        ComputedAngularInertia::new_with_local_frame(
            inertia.principal.adjust_precision(),
            inertia.local_frame.adjust_precision(),
        )
    }
}

impl From<ComputedAngularInertia> for AngularInertia {
    fn from(inertia: ComputedAngularInertia) -> Self {
        Self::from_tensor(inertia.tensor().f32())
    }
}

impl core::ops::Mul<Vector> for ComputedAngularInertia {
    type Output = Vector;

    #[inline]
    fn mul(self, rhs: Vector) -> Vector {
        self.value() * rhs
    }
}

/// The local [center of mass] computed for a dynamic [rigid body], taking into account
/// colliders and descendants. Represents the average position of mass in the body.
///
/// The total center of mass is computed as the weighted average of the centers of mass
/// of all attached colliders and the center of mass of the rigid body entity itself.
/// The center of mass of an entity is determined by its [`CenterOfMass`] component,
/// or if it is not present, from an attached [`Collider`] based on its shape.
///
/// [center of mass]: https://en.wikipedia.org/wiki/Center_of_mass
/// [rigid body]: RigidBody
///
/// # Related Types
///
/// - [`CenterOfMass`] can be used to set the local center of mass associated with an individual entity.
/// - [`ComputedMass`] stores the total mass of a rigid body, taking into account colliders and descendants.
/// - [`ComputedAngularInertia`] stores the total angular inertia of a rigid body, taking into account colliders and descendants.
/// - [`MassPropertyHelper`] is a [`SystemParam`] with utilities for computing and updating mass properties.
///
/// [`SystemParam`]: bevy::ecs::system::SystemParam
#[derive(Reflect, Clone, Copy, Component, Debug, Default, Deref, DerefMut, PartialEq, From)]
#[reflect(Debug, Component, Default, PartialEq)]
pub struct ComputedCenterOfMass(pub Vector);

impl From<CenterOfMass> for ComputedCenterOfMass {
    fn from(center_of_mass: CenterOfMass) -> Self {
        Self(center_of_mass.adjust_precision())
    }
}

impl From<ComputedCenterOfMass> for CenterOfMass {
    fn from(center_of_mass: ComputedCenterOfMass) -> Self {
        Self(center_of_mass.f32())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn mass_creation() {
        let mass = ComputedMass::new(10.0);
        assert_eq!(mass, ComputedMass::from_inverse(0.1));
        assert_eq!(mass.value(), 10.0);
        assert_eq!(mass.inverse(), 0.1);
    }

    #[test]
    fn zero_mass() {
        // Zero mass should be equivalent to infinite mass.
        let mass = ComputedMass::new(0.0);
        assert_eq!(mass, ComputedMass::new(Scalar::INFINITY));
        assert_eq!(mass, ComputedMass::from_inverse(0.0));
        assert_eq!(mass.value(), 0.0);
        assert_eq!(mass.inverse(), 0.0);
        assert!(mass.is_infinite());
        assert!(!mass.is_finite());
        assert!(!mass.is_nan());
    }

    #[test]
    fn infinite_mass() {
        let mass = ComputedMass::INFINITY;
        assert_eq!(mass, ComputedMass::new(Scalar::INFINITY));
        assert_eq!(mass, ComputedMass::from_inverse(0.0));
        assert_eq!(mass.value(), 0.0);
        assert_eq!(mass.inverse(), 0.0);
        assert!(mass.is_infinite());
        assert!(!mass.is_finite());
        assert!(!mass.is_nan());
    }

    #[test]
    #[should_panic]
    fn negative_mass_panics() {
        ComputedMass::new(-1.0);
    }

    #[test]
    fn negative_mass_error() {
        assert_eq!(
            ComputedMass::try_new(-1.0),
            Err(MassError::Negative),
            "negative mass should return an error"
        );
    }

    #[test]
    fn nan_mass_error() {
        assert_eq!(
            ComputedMass::try_new(Scalar::NAN),
            Err(MassError::NaN),
            "NaN mass should return an error"
        );
    }

    #[test]
    fn angular_inertia_creation() {
        let angular_inertia = ComputedAngularInertia::new(Vector::new(10.0, 20.0, 30.0));
        assert_relative_eq!(
            angular_inertia.inverse_tensor(),
            ComputedAngularInertia::from_inverse_tensor(SymmetricMatrix::from_diagonal(
                Vector::new(0.1, 0.05, 1.0 / 30.0)
            ))
            .inverse_tensor()
        );
        assert_relative_eq!(
            angular_inertia.tensor(),
            SymmetricMatrix::from_diagonal(Vector::new(10.0, 20.0, 30.0))
        );
        assert_relative_eq!(
            angular_inertia.inverse_tensor(),
            SymmetricMatrix::from_diagonal(Vector::new(0.1, 0.05, 1.0 / 30.0))
        );
    }

    #[test]
    fn zero_angular_inertia() {
        let angular_inertia = ComputedAngularInertia::new(Vector::ZERO);
        assert_eq!(
            angular_inertia,
            ComputedAngularInertia::new(Vector::INFINITY)
        );
        assert_eq!(
            angular_inertia,
            ComputedAngularInertia::from_inverse_tensor(SymmetricMatrix::from_diagonal(
                Vector::ZERO
            ))
        );
        assert_relative_eq!(angular_inertia.tensor(), SymmetricMatrix::ZERO);
        assert_relative_eq!(angular_inertia.inverse_tensor(), SymmetricMatrix::ZERO);
        assert!(angular_inertia.is_infinite());
        assert!(!angular_inertia.is_finite());
        assert!(!angular_inertia.is_nan());
    }

    #[test]
    fn infinite_angular_inertia() {
        let angular_inertia = ComputedAngularInertia::INFINITY;
        assert_eq!(
            angular_inertia,
            ComputedAngularInertia::new(Vector::INFINITY)
        );
        assert_eq!(
            angular_inertia,
            ComputedAngularInertia::from_inverse_tensor(SymmetricMatrix::ZERO)
        );
        assert_relative_eq!(angular_inertia.tensor(), SymmetricMatrix::ZERO);
        assert_relative_eq!(angular_inertia.inverse_tensor(), SymmetricMatrix::ZERO);
        assert!(angular_inertia.is_infinite());
        assert!(!angular_inertia.is_finite());
        assert!(!angular_inertia.is_nan());
    }

    #[test]
    #[should_panic]
    fn negative_angular_inertia_panics() {
        ComputedAngularInertia::new(Vector::new(-1.0, 2.0, 3.0));
    }

    #[test]
    fn negative_angular_inertia_error() {
        assert_eq!(
            ComputedAngularInertia::try_new(Vector::new(-1.0, 2.0, 3.0)),
            Err(AngularInertiaError::Negative),
            "negative angular inertia should return an error"
        );
    }

    #[test]
    fn nan_angular_inertia_error() {
        assert_eq!(
            ComputedAngularInertia::try_new(Vector::new(Scalar::NAN, 2.0, 3.0)),
            Err(AngularInertiaError::NaN),
            "NaN angular inertia should return an error"
        );
    }
}
