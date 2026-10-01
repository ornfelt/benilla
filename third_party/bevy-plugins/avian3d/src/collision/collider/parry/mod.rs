#![allow(clippy::unnecessary_cast)]

pub mod contact_query;

mod primitives3d;

use super::EnlargedAabb;
use crate::{make_pose, prelude::*};
use bevy::{log, prelude::*};
use contact_query::UnsupportedShape;
use itertools::Either;
use parry::shape::{RoundShape, SharedShape, TypedShape};

impl<T: IntoCollider<Collider>> From<T> for Collider {
    fn from(value: T) -> Self {
        value.collider()
    }
}

/// An error indicating an inconsistency when building a triangle mesh collider.
pub type TrimeshBuilderError = parry::shape::TriMeshBuilderError;

/// A collider used for detecting collisions and generating contacts.
///
/// # Creation
///
/// `Collider` has tons of methods for creating colliders of various shapes:
///
/// ```
/// # use avian3d::prelude::*;
/// # use bevy::prelude::*;
/// #
/// # fn setup(mut commands: Commands) {
/// // Create a ball collider with a given radius
/// commands.spawn(Collider::sphere(0.5));
/// // Create a capsule collider with a given radius and height
/// commands.spawn(Collider::capsule(0.5, 2.0));
/// # }
/// ```
///
/// Colliders on their own only detect contacts.
/// To make colliders apply contact forces, they have to be attached
/// to [rigid bodies](RigidBody):
///
/// ```
/// use avian3d::prelude::*;
/// use bevy::prelude::*;
///
/// // Spawn a dynamic body that falls onto a static platform
/// fn setup(mut commands: Commands) {
///     commands.spawn((
///         RigidBody::Dynamic,
///         Collider::sphere(0.5),
///         Transform::from_xyz(0.0, 2.0, 0.0),
///     ));
///     commands.spawn((RigidBody::Static, Collider::cuboid(5.0, 0.5, 5.0)));
/// }
/// ```
///
/// Colliders can be further configured using various components like [`Sensor`],
/// [`CollisionLayers`], and [`ColliderDensity`].
///
/// ## Multiple Colliders
///
/// It can often be useful to attach multiple colliders to the same rigid body.
///
/// This can be done by spawning several collider entities as the children of a rigid body:
///
/// ```
/// use avian3d::prelude::*;
/// use bevy::prelude::*;
///
/// fn setup(mut commands: Commands) {
///     // Spawn a rigid body with one collider on the same entity and two as children
///     commands
///         .spawn((RigidBody::Dynamic, Collider::sphere(0.5)))
///         .with_children(|children| {
///             // Spawn the child colliders positioned relative to the rigid body
///             children.spawn((Collider::sphere(0.5), Transform::from_xyz(2.0, 0.0, 0.0)));
///             children.spawn((Collider::sphere(0.5), Transform::from_xyz(-2.0, 0.0, 0.0)));
///         });
/// }
/// ```
///
/// Colliders can be arbitrarily nested and transformed relative to the parent.
/// The rigid body that a collider is attached to can be accessed using the [`ColliderOf`] component.
///
/// The benefit of using separate entities for the colliders is that each collider can have its own
/// [collision layers](CollisionLayers) and other configuration options.
///
/// # See More
///
/// - [Rigid bodies](RigidBody)
/// - [Density](ColliderDensity)
/// - [Collision layers](CollisionLayers)
/// - [Sensors](Sensor)
/// - [Filtering and modifying contacts with hooks](CollisionHooks)
/// - [Manual contact queries](contact_query)
///
/// # Advanced Usage
///
/// Internally, `Collider` uses the shapes provided by `parry`. If you want to create a collider
/// using these shapes, you can simply use `Collider::from(SharedShape::some_method())`.
///
/// To get a reference to the internal [`SharedShape`], you can use the [`Collider::shape()`]
/// or [`Collider::shape_scaled()`] methods.
///
/// `Collider` is currently not `Reflect`.
#[derive(Clone, Component, Debug)]
#[require(
    ColliderMarker,
    ColliderAabb,
    CollisionLayers,
    EnlargedAabb,
    ColliderDensity,
    ColliderMassProperties
)]
pub struct Collider {
    /// The raw unscaled collider shape.
    shape: SharedShape,
    /// The scaled version of the collider shape.
    ///
    /// If the scale is `Vector::ONE`, this will be `None` and `unscaled_shape`
    /// will be used instead.
    scaled_shape: SharedShape,
    /// The global scale used for the collider shape.
    scale: Vector,
}

impl From<SharedShape> for Collider {
    fn from(value: SharedShape) -> Self {
        Self {
            shape: value.clone(),
            scaled_shape: value,
            scale: Vector::ONE,
        }
    }
}

impl Default for Collider {
    fn default() -> Self {
        Self::cuboid(0.5, 0.5, 0.5)
    }
}

impl AnyCollider for Collider {
    type Context = ();

    fn aabb_with_context(
        &self,
        position: Vector,
        rotation: impl Into<Rotation>,
        _: AabbContext<Self::Context>,
    ) -> ColliderAabb {
        let aabb = self
            .shape_scaled()
            .compute_aabb(&make_pose(position, rotation));
        ColliderAabb {
            min: aabb.mins,
            max: aabb.maxs,
        }
    }

    fn contact_manifolds_with_context(
        &self,
        other: &Self,
        position1: Vector,
        rotation1: impl Into<Rotation>,
        position2: Vector,
        rotation2: impl Into<Rotation>,
        prediction_distance: Scalar,
        manifolds: &mut Vec<ContactManifold>,
        _: ContactManifoldContext<Self::Context>,
    ) {
        contact_query::contact_manifolds(
            self,
            position1,
            rotation1,
            other,
            position2,
            rotation2,
            prediction_distance,
            manifolds,
        )
    }
}

// TODO: `bevy_heavy` supports computing the individual mass properties efficiently for Bevy's primitive shapes,
//       but Parry doesn't support it for its own shapes, so we have to compute all mass properties in each method :(

impl ComputeMassProperties for Collider {
    fn mass(&self, density: f32) -> f32 {
        let props = self.shape_scaled().mass_properties(density as Scalar);
        props.mass() as f32
    }

    fn unit_principal_angular_inertia(&self) -> Vec3 {
        self.principal_angular_inertia(1.0)
    }

    fn principal_angular_inertia(&self, mass: f32) -> Vec3 {
        let props = self.shape_scaled().mass_properties(mass as Scalar);
        props.principal_inertia().f32()
    }

    fn local_inertial_frame(&self) -> Quat {
        let props = self.shape_scaled().mass_properties(1.0);
        props.principal_inertia_local_frame.f32()
    }

    fn center_of_mass(&self) -> Vec3 {
        let props = self.shape_scaled().mass_properties(1.0);
        props.local_com.f32()
    }

    fn mass_properties(&self, density: f32) -> MassProperties {
        let props = self.shape_scaled().mass_properties(density as Scalar);

        MassProperties {
            mass: props.mass() as f32,
            principal_angular_inertia: props.principal_inertia().f32(),
            local_inertial_frame: props.principal_inertia_local_frame.f32(),
            center_of_mass: props.local_com.f32(),
        }
    }
}

impl ScalableCollider for Collider {
    fn scale(&self) -> Vector {
        self.scale()
    }

    fn set_scale(&mut self, scale: Vector, detail: u32) {
        self.set_scale(scale, detail)
    }
}

impl Collider {
    /// Returns the raw unscaled shape of the collider.
    pub fn shape(&self) -> &SharedShape {
        &self.shape
    }

    /// Returns the shape of the collider with the scale from its `GlobalTransform` applied.
    pub fn shape_scaled(&self) -> &SharedShape {
        &self.scaled_shape
    }

    /// Returns the global scale of the collider.
    pub fn scale(&self) -> Vector {
        self.scale
    }

    /// Set the global scaling factor of this shape.
    ///
    /// If the scaling factor is not uniform, and the scaled shape can’t be
    /// represented as a supported shape, the shape is approximated as
    /// a convex polygon or polyhedron using `num_subdivisions`.
    ///
    /// For example, if a ball was scaled to an ellipse, the new shape would be approximated.
    pub fn set_scale(&mut self, scale: Vector, num_subdivisions: u32) {
        if scale == self.scale {
            return;
        }

        if scale == Vector::ONE {
            // Trivial case.
            self.scaled_shape = self.shape.clone();
            self.scale = Vector::ONE;
            return;
        }

        if let Ok(scaled) = scale_shape(&self.shape, scale, num_subdivisions) {
            self.scaled_shape = scaled;
            self.scale = scale;
        } else {
            log::error!("Failed to create convex hull for scaled collider.");
        }
    }

    /// Computes the distance and normal between the given ray and `self`
    /// transformed by `translation` and `rotation`.
    ///
    /// The returned tuple is in the format `(distance, normal)`.
    ///
    /// # Arguments
    ///
    /// - `ray_origin`: Where the ray is cast from.
    /// - `ray_direction`: What direction the ray is cast in.
    /// - `max_distance`: The maximum distance the ray can travel.
    /// - `solid`: If true and the ray origin is inside of a collider, the hit point will be the ray origin itself.
    ///   Otherwise, the collider will be treated as hollow, and the hit point will be at the collider's boundary.
    pub fn cast_ray(
        &self,
        translation: impl Into<Position>,
        rotation: impl Into<Rotation>,
        ray_origin: Vector,
        ray_direction: Vector,
        max_distance: Scalar,
        solid: bool,
    ) -> Option<(Scalar, Vector)> {
        let hit = self.shape_scaled().cast_ray_and_get_normal(
            &make_pose(translation, rotation),
            &parry::query::Ray::new(ray_origin, ray_direction),
            max_distance,
            solid,
        );
        hit.map(|hit| (hit.time_of_impact, hit.normal))
    }

    /// Creates a collider with a sphere shape defined by its radius.
    pub fn sphere(radius: Scalar) -> Self {
        SharedShape::ball(radius).into()
    }

    /// Creates a collider with a cuboid shape defined by its extents.
    pub fn cuboid(x_length: Scalar, y_length: Scalar, z_length: Scalar) -> Self {
        SharedShape::cuboid(x_length * 0.5, y_length * 0.5, z_length * 0.5).into()
    }

    /// Creates a collider with a cone shape defined by the radius of its base
    /// on the `XZ` plane and its height along the `Y` axis.
    pub fn cone(radius: Scalar, height: Scalar) -> Self {
        SharedShape::cone(height * 0.5, radius).into()
    }

    /// Creates a collider with a capsule shape defined by its radius
    /// and its height along the `Y` axis, excluding the hemispheres.
    pub fn capsule(radius: Scalar, length: Scalar) -> Self {
        SharedShape::capsule(
            Vector::Y * length * 0.5,
            Vector::NEG_Y * length * 0.5,
            radius,
        )
        .into()
    }

    /// Creates a collider with a segment shape defined by its endpoints `a` and `b`.
    pub fn segment(a: Vector, b: Vector) -> Self {
        SharedShape::segment(a, b).into()
    }

    /// Creates a collider with a polyline shape defined by its vertices and optionally an index buffer.
    pub fn polyline(vertices: Vec<Vector>, indices: Option<Vec<[u32; 2]>>) -> Self {
        SharedShape::polyline(vertices, indices).into()
    }

    /// Creates a collider with a triangle mesh shape defined by its vertex and index buffers.
    ///
    /// Note that the resulting collider will be hollow and have no interior.
    /// This makes it more prone to tunneling and other collision issues.
    ///
    /// # Panics
    ///
    /// Panics if the given vertex and index buffers do not contain any triangles,
    /// there are duplicate vertices, or if at least two adjacent triangles have opposite orientations.
    pub fn trimesh(vertices: Vec<Vector>, indices: Vec<[u32; 3]>) -> Self {
        Self::try_trimesh(vertices, indices)
            .unwrap_or_else(|error| panic!("Trimesh creation failed: {error:?}"))
    }

    /// Tries to create a collider with a triangle mesh shape defined by its vertex and index buffers.
    ///
    /// Note that the resulting collider will be hollow and have no interior.
    /// This makes it more prone to tunneling and other collision issues.
    ///
    /// # Errors
    ///
    /// Returns a [`TrimeshBuilderError`] if the given vertex and index buffers do not contain any triangles,
    /// there are duplicate vertices, or if at least two adjacent triangles have opposite orientations.
    pub fn try_trimesh(
        vertices: Vec<Vector>,
        indices: Vec<[u32; 3]>,
    ) -> Result<Self, TrimeshBuilderError> {
        SharedShape::trimesh(vertices, indices).map(|trimesh| trimesh.into())
    }
}

fn scale_shape(
    shape: &SharedShape,
    scale: Vector,
    num_subdivisions: u32,
) -> Result<SharedShape, UnsupportedShape> {
    let scale = scale.abs();
    match shape.as_typed_shape() {
        TypedShape::Cuboid(s) => Ok(SharedShape::new(s.scaled(scale.abs()))),
        TypedShape::RoundCuboid(s) => Ok(SharedShape::new(RoundShape {
            border_radius: s.border_radius,
            inner_shape: s.inner_shape.scaled(scale.abs()),
        })),
        TypedShape::Capsule(c) => match c.scaled(scale.abs(), num_subdivisions) {
            None => {
                log::error!("Failed to apply scale {} to Capsule shape.", scale);
                Ok(SharedShape::ball(0.0))
            }
            Some(Either::Left(b)) => Ok(SharedShape::new(b)),
            Some(Either::Right(b)) => Ok(SharedShape::new(b)),
        },
        TypedShape::Ball(b) => match b.scaled(scale.abs(), num_subdivisions) {
            None => {
                log::error!("Failed to apply scale {} to Ball shape.", scale);
                Ok(SharedShape::ball(0.0))
            }
            Some(Either::Left(b)) => Ok(SharedShape::new(b)),
            Some(Either::Right(b)) => Ok(SharedShape::new(b)),
        },
        TypedShape::Segment(s) => Ok(SharedShape::new(s.scaled(scale))),
        TypedShape::Triangle(t) => Ok(SharedShape::new(t.scaled(scale))),
        TypedShape::RoundTriangle(t) => Ok(SharedShape::new(RoundShape {
            border_radius: t.border_radius,
            inner_shape: t.inner_shape.scaled(scale),
        })),
        TypedShape::TriMesh(t) => Ok(SharedShape::new(t.clone().scaled(scale))),
        TypedShape::Polyline(p) => Ok(SharedShape::new(p.clone().scaled(scale))),
        TypedShape::HalfSpace(h) => match h.scaled(scale) {
            None => {
                log::error!("Failed to apply scale {} to HalfSpace shape.", scale);
                Ok(SharedShape::ball(0.0))
            }
            Some(scaled) => Ok(SharedShape::new(scaled)),
        },
        TypedShape::Voxels(v) => Ok(SharedShape::new(v.clone().scaled(scale))),
        TypedShape::HeightField(h) => Ok(SharedShape::new(h.clone().scaled(scale))),
        TypedShape::ConvexPolyhedron(cp) => match cp.clone().scaled(scale) {
            None => {
                log::error!("Failed to apply scale {} to ConvexPolyhedron shape.", scale);
                Ok(SharedShape::ball(0.0))
            }
            Some(scaled) => Ok(SharedShape::new(scaled)),
        },
        TypedShape::RoundConvexPolyhedron(cp) => match cp.clone().inner_shape.scaled(scale) {
            None => {
                log::error!(
                    "Failed to apply scale {} to RoundConvexPolyhedron shape.",
                    scale
                );
                Ok(SharedShape::ball(0.0))
            }
            Some(scaled) => Ok(SharedShape::new(RoundShape {
                border_radius: cp.border_radius,
                inner_shape: scaled,
            })),
        },
        TypedShape::Cylinder(c) => match c.scaled(scale.abs(), num_subdivisions) {
            None => {
                log::error!("Failed to apply scale {} to Cylinder shape.", scale);
                Ok(SharedShape::ball(0.0))
            }
            Some(Either::Left(b)) => Ok(SharedShape::new(b)),
            Some(Either::Right(b)) => Ok(SharedShape::new(b)),
        },
        TypedShape::RoundCylinder(c) => match c.inner_shape.scaled(scale.abs(), num_subdivisions) {
            None => {
                log::error!("Failed to apply scale {} to RoundCylinder shape.", scale);
                Ok(SharedShape::ball(0.0))
            }
            Some(Either::Left(scaled)) => Ok(SharedShape::new(RoundShape {
                border_radius: c.border_radius,
                inner_shape: scaled,
            })),
            Some(Either::Right(scaled)) => Ok(SharedShape::new(RoundShape {
                border_radius: c.border_radius,
                inner_shape: scaled,
            })),
        },
        TypedShape::Cone(c) => match c.scaled(scale, num_subdivisions) {
            None => {
                log::error!("Failed to apply scale {} to Cone shape.", scale);
                Ok(SharedShape::ball(0.0))
            }
            Some(Either::Left(b)) => Ok(SharedShape::new(b)),
            Some(Either::Right(b)) => Ok(SharedShape::new(b)),
        },
        TypedShape::RoundCone(c) => match c.inner_shape.scaled(scale, num_subdivisions) {
            None => {
                log::error!("Failed to apply scale {} to RoundCone shape.", scale);
                Ok(SharedShape::ball(0.0))
            }
            Some(Either::Left(scaled)) => Ok(SharedShape::new(RoundShape {
                border_radius: c.border_radius,
                inner_shape: scaled,
            })),
            Some(Either::Right(scaled)) => Ok(SharedShape::new(RoundShape {
                border_radius: c.border_radius,
                inner_shape: scaled,
            })),
        },
        TypedShape::Compound(c) => {
            let mut scaled = Vec::with_capacity(c.shapes().len());

            for (pose, shape) in c.shapes() {
                scaled.push((
                    make_pose(pose.translation * scale, pose.rotation),
                    scale_shape(shape, scale, num_subdivisions)?,
                ));
            }
            Ok(SharedShape::compound(scaled))
        }
        TypedShape::Custom(_shape) => Err(parry::query::Unsupported),
    }
}
