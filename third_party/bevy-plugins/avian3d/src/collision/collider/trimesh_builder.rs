//! Types used for creating triangle meshes from [`Collider`]s.

use core::num::NonZeroU32;

use bevy::prelude::*;
use parry::shape::SharedShape;
use thiserror::Error;

use crate::prelude::*;

/// An ergonomic builder for triangle meshes from [`Collider`]s.
///
/// The builder can configure different subdivision levels for different shapes.
/// If a shape was not explicitly configured, the builder will use [`Self::fallback_subdivisions`].
///
/// Shapes with rounded corners such as [`Collider::round_cuboid`] will be subdivided as if they were not rounded.
///
/// # Example
///
/// ```
/// # use avian3d::{prelude::*, math::Vector};
///
/// let collider = Collider::sphere(1.0);
///
/// // Using default settings
/// let trimesh = collider.trimesh_builder().build().unwrap();
///
/// // Using extra subdivisions
/// let trimesh = collider
///     .trimesh_builder()
///     .sphere_subdivisions(20, 20)
///     .build()
///     .unwrap();
///
/// // Setting different subdivisions for different shapes
/// let trimesh = collider
///     .trimesh_builder()
///     .sphere_subdivisions(20, 20)
///     .capsule_subdivisions(10, 5)
///     .fallback_subdivisions(15)
///     .build()
///     .unwrap();
///
/// // Generating the trimesh with a transformation
/// let trimesh = collider
///     .trimesh_builder()
///     .translated(Vector::new(1.0, 0.0, 0.0))
///     .build()
///     .unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct TrimeshBuilder {
    /// The shape to be converted into a triangle mesh.
    pub shape: SharedShape,
    /// The position of the shape. The default is [0, 0, 0].
    pub position: Position,
    /// The rotation of the shape. The default is the identity rotation.
    pub rotation: Rotation,
    /// Whether a failure to trimesh a subshape in a compound shape should fail the entire build process.
    /// Default is true.
    pub fail_on_compound_error: bool,
    /// The number of subdivisions to use for shapes that do not have a specific subdivision count.
    /// Default is 16.
    pub fallback_subdivisions: NonZeroU32,
    /// The number of subdivisions for shapes that derive from a sphere.
    /// Default is None.
    pub sphere_subdivisions: Option<(NonZeroU32, NonZeroU32)>,
    /// The number of subdivisions for shapes that derive from a capsule.
    /// Default is None.
    pub capsule_subdivision: Option<(NonZeroU32, NonZeroU32)>,
    /// The number of subdivisions for shapes that derive from a cylinder.
    /// Default is None.
    pub cylinder_subdivisions: Option<NonZeroU32>,
    /// The number of subdivisions for shapes that derive from a cone.
    /// Default is None.
    pub cone_subdivisions: Option<NonZeroU32>,
}

/// A generic triangle mesh representation.
#[derive(Debug, Clone, PartialEq, Reflect, Default)]
pub struct Trimesh {
    /// The vertices in
    pub vertices: Vec<Vector>,
    /// The indices in counter-clockwise winding
    pub indices: Vec<[u32; 3]>,
}

/// An error that can occur when building a triangle mesh with a [`TrimeshBuilder`].
#[derive(Debug, Error)]
pub enum TrimeshBuilderError {
    /// The shape is not supported by the builder.
    #[error("Unsupported shape type: {0}")]
    UnsupportedShape(String),
}

impl From<Trimesh> for Mesh {
    fn from(trimesh: Trimesh) -> Self {
        use bevy::asset::RenderAssetUsages;
        use bevy::mesh::{Indices, PrimitiveTopology, VertexAttributeValues, prelude::*};

        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_POSITION,
            VertexAttributeValues::Float32x3(
                trimesh
                    .vertices
                    .into_iter()
                    .map(|v| v.f32().to_array())
                    .collect(),
            ),
        );
        mesh.insert_indices(Indices::U32(
            trimesh.indices.into_iter().flatten().collect(),
        ));
        mesh.compute_normals();
        if let Err(err) = mesh.generate_tangents() {
            warn!("Failed to generate tangents for mesh: {err}");
        }

        mesh
    }
}
