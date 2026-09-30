use bevy_mesh::MissingVertexAttributeError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SpecializedMeshPipelineError {
    #[error(transparent)]
    MissingVertexAttribute(#[from] MissingVertexAttributeError),
}
