//! `bevy_heavy` is a crate for computing mass properties ([mass], [angular inertia], and [center of mass])
//! in the [Bevy game engine][Bevy]. This is typically required for things like physics simulations.
//!
//! [mass]: #mass
//! [angular inertia]: #angular-inertia
//! [center of mass]: #center-of-mass
//! [Bevy]: https://bevyengine.org
//!
//! # Usage
//!
//! Implement [`ComputeMassProperties3d`] for a shape to compute its [`MassProperties3d`].
//! The mass property types have several helper methods for various transformations and operations,
//! and can be added and subtracted.
//!
//! # Terminology
//!
//! ## Mass
//!
//! **[Mass](https://en.wikipedia.org/wiki/Mass)** is a scalar value representing resistance
//! to linear acceleration when a force is applied.
//!
//! Mass is commonly measured in kilograms (kg).
//!
//! ## Angular Inertia
//!
//! **[Angular inertia](https://en.wikipedia.org/wiki/Moment_of_inertia)**, also known as
//! the **moment of inertia** or **rotational inertia**, is the rotational analog of mass.
//! It represents resistance to angular acceleration when a torque is applied.
//!
//! An object's angular inertia depends on its mass, shape, and how the mass is distributed
//! relative to a rotational axis. It increases with mass and distance from the axis.
//!
//! In 3D, angular inertia can be represented with a [symmetric], [positive-semidefinite] 3x3 [tensor]
//! ([`AngularInertiaTensor`]) that describes the moment of inertia for rotations about the X, Y, and Z axes.
//! By [diagonalizing] this matrix, it is possible to extract the [principal axes of inertia] (a [`Vec3`])
//! and a local inertial frame (a [`Quat`]) that defines the XYZ axes.
//!
//! The latter diagonalized representation is more compact and often easier to work with,
//! but the full tensor can be more efficient for computations using the angular inertia.
//!
//! Angular inertia is commonly measured in kilograms times meters squared (kg⋅m²).
//!
//! [symmetric]: https://en.wikipedia.org/wiki/Symmetric_matrix
//! [positive-semidefinite]: https://en.wikipedia.org/wiki/Definite_matrix
//! [tensor]: https://en.wikipedia.org/wiki/Moment_of_inertia#Inertia_tensor
//! [diagonalizing]: https://en.wikipedia.org/wiki/Diagonalizable_matrix#Diagonalization
//! [principal axes of inertia]: https://en.wikipedia.org/wiki/Moment_of_inertia#Principal_axes
//! [`Vec3`]: bevy_math::Vec3
//! [`Quat`]: bevy_math::Quat
//!
//! ## Center of Mass
//!
//! The **[center of mass](https://en.wikipedia.org/wiki/Center_of_mass)** is the average position
//! of mass in an object. Applying a force at the center of mass causes linear acceleration
//! without angular acceleration.
//!
//! If an object has uniform density, mass is evenly distributed,
//! and the center of mass is at the [geometric center], also known as the [centroid].
//!
//! The center of mass is commonly measured in meters (m).
//!
//! [geometric center]: https://en.wikipedia.org/wiki/Centroid
//! [centroid]: https://en.wikipedia.org/wiki/Centroid

#![warn(missing_docs)]
#![no_std]

extern crate alloc;

mod dim3;
mod math_ext;

pub use dim3::*;
pub use glam_matrix_extras::{Mat3Ext, MatConversionError, SquareMatExt, SymmetricMat3};
pub use math_ext::RecipOrZero;
