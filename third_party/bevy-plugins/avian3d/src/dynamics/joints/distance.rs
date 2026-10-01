use crate::{
    dynamics::joints::{EntityConstraint, JointSystems},
    prelude::*,
};
use bevy::{
    ecs::entity::{EntityMapper, MapEntities},
    prelude::*,
};

/// A distance [joint](dynamics::joints) maintains an upper and/or lower bound on the distance
/// between anchor points on two bodies.
///
/// This can be useful for things like springs, muscles, and mass-spring networks.
///
/// A distance joint is defined by a [`JointAnchor`] on each body, and a [`DistanceLimit`]. The joint aims to keep
/// the distance between the two anchor points within the specified limits.
///
#[doc = include_str!("./images/distance_joint.svg")]
#[derive(Component, Clone, Debug, PartialEq)]
pub struct DistanceJoint {
    /// The first body constrained by the joint.
    pub body1: Entity,
    /// The second body constrained by the joint.
    pub body2: Entity,
    /// The joint anchor point on the first body.
    pub anchor1: JointAnchor,
    /// The joint anchor point on the second body.
    pub anchor2: JointAnchor,
    /// The extents of the allowed relative translation between the attached bodies.
    pub limits: DistanceLimit,
    /// The joint's compliance, the inverse of stiffness (m / N).
    pub compliance: Scalar,
}

impl EntityConstraint<2> for DistanceJoint {
    fn entities(&self) -> [Entity; 2] {
        [self.body1, self.body2]
    }
}

impl MapEntities for DistanceJoint {
    fn map_entities<M: EntityMapper>(&mut self, entity_mapper: &mut M) {
        self.body1 = entity_mapper.get_mapped(self.body1);
        self.body2 = entity_mapper.get_mapped(self.body2);
    }
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        PhysicsSchedule,
        update_local_anchors.in_set(JointSystems::PrepareLocalFrames),
    );
}

fn update_local_anchors(
    mut joints: Query<&mut DistanceJoint, Changed<DistanceJoint>>,
    bodies: Query<(&Position, &Rotation)>,
) {
    for mut joint in &mut joints {
        if matches!(joint.anchor1, JointAnchor::Local(_))
            && matches!(joint.anchor2, JointAnchor::Local(_))
        {
            continue;
        }

        let Ok([(pos1, rot1), (pos2, rot2)]) = bodies.get_many(joint.entities()) else {
            continue;
        };

        let [anchor1, anchor2] =
            JointAnchor::compute_local(joint.anchor1, joint.anchor2, pos1.0, pos2.0, rot1, rot2);
        joint.anchor1 = anchor1;
        joint.anchor2 = anchor2;
    }
}
