//! A [`JointGraph`] for tracking how [rigid bodies] are connected by [joints].
//!
//! [rigid bodies]: crate::dynamics::RigidBody
//! [joints]: crate::dynamics::joints

mod plugin;
pub use plugin::{JointComponentId, JointGraphPlugin};

use crate::{
    data_structures::{
        graph::{EdgeIndex, NodeIndex},
        sparse_secondary_map::SparseSecondaryEntityMap,
        stable_graph::StableUnGraph,
    },
    dynamics::solver::islands::IslandNode,
};
use bevy::prelude::*;

// TODO: Once we have many-to-many relationships, we could potentially represent the joint graph in the ECS.

/// A resource for the joint graph, tracking how [rigid bodies] are connected by [joints].
///
/// [rigid bodies]: crate::dynamics::RigidBody
/// [joints]: crate::dynamics::joints
#[derive(Resource, Clone, Debug, Default)]
pub struct JointGraph {
    graph: StableUnGraph<Entity, JointGraphEdge>,
    entity_to_body: SparseSecondaryEntityMap<NodeIndex>,
    entity_to_joint: SparseSecondaryEntityMap<EdgeIndex>,
}

/// A stable identifier for a [`JointGraphEdge`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Reflect)]
#[reflect(Debug, PartialEq)]
pub struct JointId(pub u32);

impl JointId {
    /// A placeholder identifier for a [`JointGraphEdge`].
    pub const PLACEHOLDER: Self = Self(u32::MAX);
}

impl From<JointId> for EdgeIndex {
    fn from(id: JointId) -> Self {
        Self(id.0)
    }
}

impl From<EdgeIndex> for JointId {
    fn from(id: EdgeIndex) -> Self {
        Self(id.0)
    }
}

impl core::fmt::Display for JointId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "JointId({})", self.0)
    }
}

/// An edge in the [`JointGraph`].
#[derive(Clone, Debug, Reflect)]
#[reflect(Debug)]
pub struct JointGraphEdge {
    /// The stable identifier of this joint edge.
    pub id: JointId,

    /// The entity of the joint.
    pub entity: Entity,

    /// The first [rigid body] connected by this joint.
    ///
    /// [rigid body]: crate::dynamics::RigidBody
    pub body1: Entity,

    /// The second [rigid body] connected by this joint.
    ///
    /// [rigid body]: crate::dynamics::RigidBody
    pub body2: Entity,

    /// The [`IslandNode`] associated with this joint.
    pub island: IslandNode<JointId>,
}

impl JointGraphEdge {
    /// Creates a new [`JointGraphEdge`].
    #[inline]
    pub fn new(entity: Entity, body1: Entity, body2: Entity) -> Self {
        Self {
            // This gets set to a valid ID when the joint is added to the `JointGraph`.
            id: JointId::PLACEHOLDER,
            entity,
            body1,
            body2,
            island: IslandNode::default(),
        }
    }
}

impl JointGraph {
    /// Returns the [`NodeIndex`] of the given entity in the joint graph.
    ///
    /// If the entity is not in the graph, `None` is returned.
    #[inline]
    pub fn entity_to_body(&self, entity: Entity) -> Option<NodeIndex> {
        self.entity_to_body.get(entity).copied()
    }

    /// Returns the [`JointId`] of the joint edge for the given entity.
    ///
    /// If the entity is not in the graph, `None` is returned.
    #[inline]
    pub fn entity_to_joint(&self, entity: Entity) -> Option<JointId> {
        self.entity_to_joint.get(entity).copied().map(JointId::from)
    }

    /// Returns the [`JointGraphEdge`] for the given joint entity.
    /// If the joint is not in the graph, `None` is returned.
    #[inline]
    pub fn get(&self, joint: Entity) -> Option<&JointGraphEdge> {
        let joint_index = self.entity_to_joint(joint)?;
        self.get_by_id(joint_index)
    }

    /// Returns a reference to the [`JointGraphEdge`] for the given [`JointId`].
    /// If the joint is not in the graph, `None` is returned.
    #[inline]
    pub fn get_by_id(&self, joint_id: JointId) -> Option<&JointGraphEdge> {
        self.graph.edge_weight(joint_id.into())
    }

    /// Returns a mutable reference to the [`JointGraphEdge`] for the given [`JointId`].
    /// If the joint is not in the graph, `None` is returned.
    #[inline]
    pub fn get_mut_by_id(&mut self, joint_id: JointId) -> Option<&mut JointGraphEdge> {
        self.graph.edge_weight_mut(joint_id.into())
    }

    /// Returns an iterator yielding immutable access to all joint edges involving the given entity.
    #[inline]
    pub fn joints_of(&self, body: Entity) -> impl Iterator<Item = &JointGraphEdge> {
        let index = self.entity_to_body(body);
        if let Some(index) = index {
            itertools::Either::Left(self.graph.edge_weights(index))
        } else {
            itertools::Either::Right(core::iter::empty())
        }
    }

    /// Creates a [`JointGraphEdge`] between two entities if it does not already exist,
    /// and returns the [`EdgeIndex`] of the created joint edge.
    ///
    /// # Warning
    ///
    /// Creating a joint edge with this method will *not* wake up the entities involved
    /// or do any other clean-up. Only use this method if you know what you are doing.
    #[inline]
    pub fn add_joint(
        &mut self,
        body1: Entity,
        body2: Entity,
        joint_edge: JointGraphEdge,
    ) -> JointId {
        // Get the indices of the entities in the graph.
        let body1_index = self
            .entity_to_body
            .get_or_insert_with(body1, || self.graph.add_node(body1));
        let body2_index = self
            .entity_to_body
            .get_or_insert_with(body2, || self.graph.add_node(body2));

        // Add the edge to the graph.
        let joint_entity = joint_edge.entity;
        let edge_id = JointId(self.graph.add_edge(body1_index, body2_index, joint_edge).0);

        // Insert the joint edge into the entity-to-joint mapping.
        self.entity_to_joint
            .get_or_insert_with(joint_entity, || edge_id.into());

        // Set the joint ID in the joint edge.
        let edge = self.graph.edge_weight_mut(edge_id.into()).unwrap();
        edge.id = edge_id;

        edge_id
    }

    /// Removes a [`JointGraphEdge`] between two entites and returns its value.
    ///
    /// # Warning
    ///
    /// Removing a joint edge with this method will *not* wake up the entities involved
    /// or do any other clean-up. Only use this method if you know what you are doing.
    #[inline]
    pub fn remove_joint(&mut self, joint_entity: Entity) -> Option<JointGraphEdge> {
        let joint_index = self.entity_to_joint.remove(joint_entity)?;
        self.graph.remove_edge(joint_index)
    }
}
