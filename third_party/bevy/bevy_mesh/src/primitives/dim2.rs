use crate::{Indices, Mesh};
use bevy_asset::RenderAssetUsages;

use super::{MeshBuilder, Meshable};
use bevy_math::{primitives::Rectangle, Vec2};
use wgpu_types::PrimitiveTopology;

/// A builder used for creating a [`Mesh`] with a [`Rectangle`] shape.
#[derive(Clone, Copy, Debug)]
pub struct RectangleMeshBuilder {
    half_size: Vec2,
}

impl Default for RectangleMeshBuilder {
    /// Returns the default [`RectangleMeshBuilder`] with a half-width and half-height of `0.5`.
    fn default() -> Self {
        Self {
            half_size: Vec2::splat(0.5),
        }
    }
}

impl MeshBuilder for RectangleMeshBuilder {
    fn build(&self) -> Mesh {
        let [hw, hh] = [self.half_size.x, self.half_size.y];
        let positions = vec![
            [hw, hh, 0.0],
            [-hw, hh, 0.0],
            [-hw, -hh, 0.0],
            [hw, -hh, 0.0],
        ];
        let normals = vec![[0.0, 0.0, 1.0]; 4];
        let uvs = vec![[1.0, 0.0], [0.0, 0.0], [0.0, 1.0], [1.0, 1.0]];
        let indices = Indices::U32(vec![0, 1, 2, 0, 2, 3]);

        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_indices(indices)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    }
}

impl Meshable for Rectangle {
    type Output = RectangleMeshBuilder;

    fn mesh(&self) -> Self::Output {
        RectangleMeshBuilder {
            half_size: self.half_size,
        }
    }
}

impl From<Rectangle> for Mesh {
    fn from(rectangle: Rectangle) -> Self {
        rectangle.mesh().build()
    }
}
