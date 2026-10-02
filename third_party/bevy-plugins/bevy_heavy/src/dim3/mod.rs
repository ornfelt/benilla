use alloc::vec::Vec;

use bevy_math::{Isometry3d, Quat, Vec3};

mod angular_inertia;
pub use angular_inertia::{AngularInertiaTensor, AngularInertiaTensorError};

use crate::RecipOrZero;

/// A trait for computing [`MassProperties3d`] for 3D objects.
pub trait ComputeMassProperties3d {
    /// Computes the [mass] of the object with a given `density`.
    ///
    /// [mass]: crate#mass
    fn mass(&self, density: f32) -> f32;

    /// Computes the principal [angular inertia] corresponding to a mass of `1.0`.
    ///
    /// [angular inertia]: crate#angular-inertia
    #[doc(alias = "unit_principal_moment_of_inertia")]
    fn unit_principal_angular_inertia(&self) -> Vec3;

    /// Computes the principal [angular inertia] corresponding to the given `mass`.
    ///
    /// Equivalent to `mass * shape.unit_principal_angular_inertia()`.
    ///
    /// [angular inertia]: crate#angular-inertia
    #[inline]
    #[doc(alias = "principal_moment_of_inertia")]
    fn principal_angular_inertia(&self, mass: f32) -> Vec3 {
        mass * self.unit_principal_angular_inertia()
    }

    /// Computes the orientation of the inertial frame used by the principal axes of [inertia] in local space.
    ///
    /// For most primitive shapes, this returns an identity quaternion, which means that the principal axes
    /// are aligned with the object's XYZ axes.
    ///
    /// [inertia]: crate#angular-inertia
    #[inline]
    fn local_inertial_frame(&self) -> Quat {
        Quat::IDENTITY
    }

    /// Computes the 3x3 [`AngularInertiaTensor`] corresponding to a mass of `1.0`.
    #[inline]
    fn unit_angular_inertia_tensor(&self) -> AngularInertiaTensor {
        AngularInertiaTensor::new_with_local_frame(
            self.principal_angular_inertia(1.0),
            self.local_inertial_frame(),
        )
    }

    /// Computes the 3x3 [`AngularInertiaTensor`] corresponding to the given `mass`.
    #[inline]
    fn angular_inertia_tensor(&self, mass: f32) -> AngularInertiaTensor {
        mass * self.unit_angular_inertia_tensor()
    }

    /// Computes the local [center of mass] relative to the object's origin.
    ///
    /// [center of mass]: crate#center-of-mass
    fn center_of_mass(&self) -> Vec3;

    /// Computes the [`MassProperties3d`] with a given `density`.
    #[inline]
    fn mass_properties(&self, density: f32) -> MassProperties3d {
        let mass = self.mass(density);
        MassProperties3d::new_with_local_frame(
            mass,
            self.principal_angular_inertia(mass),
            self.local_inertial_frame(),
            self.center_of_mass(),
        )
    }
}

/// The [mass], [angular inertia], and local [center of mass] of an object in 3D space.
///
/// [mass]: crate#mass
/// [angular inertia]: crate#angular-inertia
/// [center of mass]: crate#center-of-mass
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MassProperties3d {
    /// The [mass].
    ///
    /// [mass]: crate#mass
    pub mass: f32,
    /// The [angular inertia] along the principal axes defined by the local inertial frame.
    ///
    /// [angular inertia]: crate#angular-inertia
    pub principal_angular_inertia: Vec3,
    /// The orientation of the local inertial frame, defining the principal axes.
    pub local_inertial_frame: Quat,
    /// The local [center of mass] relative to the object's origin.
    ///
    /// [center of mass]: crate#center-of-mass
    pub center_of_mass: Vec3,
}

impl Default for MassProperties3d {
    /// Returns the default [`MassProperties3d`], with zero mass and angular inertia.
    fn default() -> Self {
        Self::ZERO
    }
}

impl MassProperties3d {
    /// Zero mass and angular inertia.
    pub const ZERO: Self = Self {
        mass: 0.0,
        principal_angular_inertia: Vec3::ZERO,
        local_inertial_frame: Quat::IDENTITY,
        center_of_mass: Vec3::ZERO,
    };

    /// Creates a new [`MassProperties3d`] from a given mass, principal angular inertia,
    /// local inertial frame, and center of mass in local space.
    ///
    /// The principal angular inertia is the angular inertia along the coordinate axes defined
    /// by the `local_inertial_frame`, expressed in local space.
    #[inline]
    pub fn new_with_local_frame(
        mass: f32,
        principal_angular_inertia: Vec3,
        local_inertial_frame: Quat,
        center_of_mass: Vec3,
    ) -> Self {
        Self {
            mass,
            principal_angular_inertia,
            local_inertial_frame,
            center_of_mass,
        }
    }

    /// Creates a new [`MassProperties3d`] from a given mass, angular inertia tensor,
    /// and center of mass in local space.
    ///
    /// The angular inertia tensor will be diagonalized in order to extract the principal inertia
    /// values and local principal inertia frme.
    #[inline]
    pub fn new_with_angular_inertia_tensor(
        mass: f32,
        tensor: impl Into<AngularInertiaTensor>,
        center_of_mass: Vec3,
    ) -> Self {
        let (principal, local_frame) = tensor.into().principal_angular_inertia_with_local_frame();
        Self::new_with_local_frame(mass, principal, local_frame, center_of_mass)
    }

    /// Returns the center of mass transformed into global space using the given [isometry].
    ///
    /// [isometry]: Isometry3d
    #[inline]
    pub fn global_center_of_mass(&self, isometry: impl Into<Isometry3d>) -> Vec3 {
        let isometry: Isometry3d = isometry.into();
        isometry.transform_point(self.center_of_mass).into()
    }

    /// Computes the world-space angular inertia tensor from the principal inertia.
    #[inline]
    pub fn angular_inertia_tensor(&self) -> AngularInertiaTensor {
        AngularInertiaTensor::new_with_local_frame(
            self.principal_angular_inertia,
            self.local_inertial_frame,
        )
    }

    /// Computes the angular inertia tensor at a given `offset`.
    #[inline]
    pub fn shifted_angular_inertia_tensor(&self, offset: Vec3) -> AngularInertiaTensor {
        self.angular_inertia_tensor().shifted(self.mass, offset)
    }

    /// Transforms the mass properties by the given [isometry].
    ///
    /// [isometry]: Isometry3d
    #[inline]
    pub fn transform_by(&mut self, isometry: impl Into<Isometry3d>) {
        let isometry: Isometry3d = isometry.into();
        self.center_of_mass = self.global_center_of_mass(isometry);
        self.local_inertial_frame = isometry.rotation * self.local_inertial_frame;
    }

    /// Sets the mass to the given `new_mass`.
    ///
    /// If `update_angular_inertia` is `true`, the principal angular inertia will be scaled accordingly.
    #[inline]
    pub fn set_mass(&mut self, new_mass: f32, update_angular_inertia: bool) {
        if update_angular_inertia {
            // Adjust angular inertia to match the new mass.
            self.principal_angular_inertia *= new_mass * self.mass.recip_or_zero();
        }
        self.mass = new_mass;
    }
}

impl core::ops::Add for MassProperties3d {
    type Output = Self;

    #[inline]
    fn add(self, other: Self) -> Self::Output {
        if self == Self::ZERO {
            return other;
        } else if other == Self::ZERO {
            return self;
        }

        let mass1 = self.mass;
        let mass2 = other.mass;
        let new_mass = mass1 + mass2;

        // The new center of mass is the weighted average of the centers of masses of `self` and `other`.
        let new_center_of_mass =
            (self.center_of_mass * mass1 + other.center_of_mass * mass2) / new_mass;

        // Compute the new principal angular inertia, taking the new center of mass into account.
        let i1 = self.shifted_angular_inertia_tensor(new_center_of_mass - self.center_of_mass);
        let i2 = other.shifted_angular_inertia_tensor(new_center_of_mass - other.center_of_mass);
        let new_angular_inertia = AngularInertiaTensor::from_symmetric_mat3(
            i1.as_symmetric_mat3() + i2.as_symmetric_mat3(),
        );

        Self::new_with_angular_inertia_tensor(new_mass, new_angular_inertia, new_center_of_mass)
    }
}

impl core::ops::AddAssign for MassProperties3d {
    #[inline]
    fn add_assign(&mut self, other: Self) {
        *self = *self + other;
    }
}

impl core::ops::Sub for MassProperties3d {
    type Output = Self;

    #[inline]
    fn sub(self, other: Self) -> Self::Output {
        if self == Self::ZERO || other == Self::ZERO {
            return self;
        }

        let mass1 = self.mass;
        let mass2 = other.mass;

        if mass1 <= mass2 {
            // The result would have non-positive mass.
            return Self {
                center_of_mass: self.center_of_mass,
                ..Self::ZERO
            };
        }

        let new_mass = mass1 - mass2;

        // The new center of mass is the negated weighted average of the centers of masses of `self` and `other`.
        let new_center_of_mass =
            (self.center_of_mass * mass1 - other.center_of_mass * mass2) * new_mass.recip_or_zero();

        // Compute the new principal angular inertia, taking the new center of mass into account.
        let i1 = self.shifted_angular_inertia_tensor(new_center_of_mass - self.center_of_mass);
        let i2 = other.shifted_angular_inertia_tensor(new_center_of_mass - other.center_of_mass);
        let new_angular_inertia = AngularInertiaTensor::from_symmetric_mat3(
            i1.as_symmetric_mat3() - i2.as_symmetric_mat3(),
        );

        Self::new_with_angular_inertia_tensor(new_mass, new_angular_inertia, new_center_of_mass)
    }
}

impl core::ops::SubAssign for MassProperties3d {
    #[inline]
    fn sub_assign(&mut self, other: Self) {
        *self = *self - other;
    }
}

impl core::iter::Sum for MassProperties3d {
    #[inline]
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        let mut total_mass = 0.0;
        let mut total_angular_inertia = AngularInertiaTensor::ZERO;
        let mut total_center_of_mass = Vec3::ZERO;

        // TODO: Avoid this allocation if possible. This is currently needed because we iterate twice.
        let mut all_properties = Vec::with_capacity(iter.size_hint().1.unwrap_or_default());

        for props in iter {
            total_mass += props.mass;
            total_center_of_mass += props.center_of_mass * props.mass;
            all_properties.push(props);
        }

        if total_mass > 0.0 {
            total_center_of_mass *= total_mass.recip_or_zero();
        }

        for props in all_properties {
            total_angular_inertia +=
                props.shifted_angular_inertia_tensor(total_center_of_mass - props.center_of_mass);
        }

        if !total_center_of_mass.is_finite() {
            // The center of mass can be non-finite if the mass is non-finite.
            total_center_of_mass = Vec3::ZERO;
        }

        Self::new_with_angular_inertia_tensor(
            total_mass,
            total_angular_inertia,
            total_center_of_mass,
        )
    }
}

#[cfg(any(feature = "approx", test))]
impl approx::AbsDiffEq for MassProperties3d {
    type Epsilon = f32;
    fn default_epsilon() -> f32 {
        f32::EPSILON
    }
    fn abs_diff_eq(&self, other: &Self, epsilon: f32) -> bool {
        self.mass.abs_diff_eq(&other.mass, epsilon)
            && self
                .principal_angular_inertia
                .abs_diff_eq(other.principal_angular_inertia, epsilon)
            && self
                .local_inertial_frame
                .abs_diff_eq(other.local_inertial_frame, epsilon)
            && self
                .center_of_mass
                .abs_diff_eq(other.center_of_mass, epsilon)
    }
}

#[cfg(any(feature = "approx", test))]
impl approx::RelativeEq for MassProperties3d {
    fn default_max_relative() -> f32 {
        f32::EPSILON
    }
    fn relative_eq(&self, other: &Self, epsilon: f32, max_relative: f32) -> bool {
        self.mass.relative_eq(&other.mass, epsilon, max_relative)
            && self.principal_angular_inertia.relative_eq(
                &other.principal_angular_inertia,
                epsilon,
                max_relative,
            )
            && self.local_inertial_frame.relative_eq(
                &other.local_inertial_frame,
                epsilon,
                max_relative,
            )
            && self
                .center_of_mass
                .relative_eq(&other.center_of_mass, epsilon, max_relative)
    }
}

#[cfg(any(feature = "approx", test))]
impl approx::UlpsEq for MassProperties3d {
    fn default_max_ulps() -> u32 {
        4
    }
    fn ulps_eq(&self, other: &Self, epsilon: f32, max_ulps: u32) -> bool {
        self.mass.ulps_eq(&other.mass, epsilon, max_ulps)
            && self.principal_angular_inertia.ulps_eq(
                &other.principal_angular_inertia,
                epsilon,
                max_ulps,
            )
            && self
                .local_inertial_frame
                .ulps_eq(&other.local_inertial_frame, epsilon, max_ulps)
            && self
                .center_of_mass
                .ulps_eq(&other.center_of_mass, epsilon, max_ulps)
    }
}
