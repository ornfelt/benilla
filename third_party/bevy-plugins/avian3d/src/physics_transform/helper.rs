use bevy::{
    ecs::system::{Query, SystemParam, lifetimeless::Write},
    transform::helper::TransformHelper,
};

use crate::prelude::{Position, Rotation};

/// A system parameter for computing up-to-date [`Position`] and [`Rotation`] components
/// of entities based on their [`Transform`]s.
///
/// This can be useful to ensure that physics transforms are immediately updated after changes
/// to the [`Transform`], before transform propagation systems are run.
///
/// Computing the global transform of each entity individually can be expensive,
/// so it is recommended to only use this for specific entities that require immediate updates,
/// such as right after teleporting an entity.
///
/// [`Transform`]: bevy::transform::components::Transform
#[derive(SystemParam)]
pub struct PhysicsTransformHelper<'w, 's> {
    /// The [`TransformHelper`] used to compute the global transform.
    pub transform_helper: TransformHelper<'w, 's>,
    /// A query for the [`Position`] and [`Rotation`] components.
    pub query: Query<'w, 's, (Write<Position>, Write<Rotation>)>,
}
