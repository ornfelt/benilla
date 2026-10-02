use bevy_ecs::{component::Component, entity::Entity};
use bevy_utils::TypeIdMap;

use crate::sync_world::MainEntity;

mod range;
pub use range::*;

/// Collection of entities visible from the current view.
///
/// This component is extracted from [`VisibleEntities`].
#[derive(Clone, Component, Default, Debug)]
pub struct RenderVisibleEntities {
    pub entities: TypeIdMap<Vec<(Entity, MainEntity)>>,
}
