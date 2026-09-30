//! Utilities to walk the UI hierarchy (without the `ghost_nodes` feature, which nothing enables).

use crate::Node;
use bevy_ecs::{prelude::*, system::SystemParam};

pub type UiRootNodes<'w, 's> = Query<'w, 's, Entity, (With<Node>, Without<ChildOf>)>;

/// System param that gives access to UI children utilities.
#[derive(SystemParam)]
pub struct UiChildren<'w, 's> {
    ui_children_query: Query<'w, 's, Option<&'static Children>, With<Node>>,
    changed_children_query: Query<'w, 's, Entity, Changed<Children>>,
}

impl<'w, 's> UiChildren<'w, 's> {
    /// Iterates the children of `entity`.
    pub fn iter_ui_children(&'s self, entity: Entity) -> impl Iterator<Item = Entity> + 's {
        self.ui_children_query
            .get(entity)
            .ok()
            .flatten()
            .map(|children| children.as_ref())
            .unwrap_or(&[])
            .iter()
            .copied()
    }

    /// Given an entity in the UI hierarchy, check if its set of children has changed, e.g if children has been added/removed or if the order has changed.
    pub fn is_changed(&'s self, entity: Entity) -> bool {
        self.changed_children_query.contains(entity)
    }
}
