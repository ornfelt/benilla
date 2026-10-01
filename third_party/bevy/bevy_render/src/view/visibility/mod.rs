use core::any::TypeId;

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

impl RenderVisibleEntities {
    pub fn get<QF>(&self) -> &[(Entity, MainEntity)]
    where
        QF: 'static,
    {
        match self.entities.get(&TypeId::of::<QF>()) {
            Some(entities) => &entities[..],
            None => &[],
        }
    }

    pub fn iter<QF>(&self) -> impl DoubleEndedIterator<Item = &(Entity, MainEntity)>
    where
        QF: 'static,
    {
        self.get::<QF>().iter()
    }

    pub fn len<QF>(&self) -> usize
    where
        QF: 'static,
    {
        self.get::<QF>().len()
    }

    pub fn is_empty<QF>(&self) -> bool
    where
        QF: 'static,
    {
        self.get::<QF>().is_empty()
    }
}
