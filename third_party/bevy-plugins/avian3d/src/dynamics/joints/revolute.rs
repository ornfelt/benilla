use crate::{
    dynamics::joints::{EntityConstraint, JointSystems, motor::AngularMotor},
    prelude::*,
};
use bevy::{
    ecs::entity::{EntityMapper, MapEntities},
    prelude::*,
};

/// A revolute [joint](dynamics::joints) or hinge prevents any relative movement between two bodies, except for rotation about the [`hinge_axis`](Self::hinge_axis) at a pivot point defined by the joint anchor.
///
/// This can be useful for things like wheels, fans, doors, and other rotating mechanisms.
///
/// Each revolute joint is defined by a [`JointFrame`] on each body, a [`hinge_axis`](Self::hinge_axis) about which the bodies can rotate,
/// and an optional [`AngleLimit`] that defines the extents of the allowed rotation. The joint aims to keep the anchor point of each frame aligned,
/// while allowing rotation about the hinge axis at the anchor point.
///
#[doc = include_str!("./images/revolute_joint.svg")]
///
/// The joint can also include an [`AngularMotor`] for driving the rotation about the pivot point.
/// Use this to create wheels, fans, servos, or other rotating mechanisms.
#[derive(Component, Clone, Debug, PartialEq)]
#[doc(alias = "HingeJoint")]
pub struct RevoluteJoint {
    /// The first body constrained by the joint.
    pub body1: Entity,
    /// The second body constrained by the joint.
    pub body2: Entity,
    /// The reference frame of the first body, defining the joint anchor and basis
    /// relative to the body transform.
    pub frame1: JointFrame,
    /// The reference frame of the second body, defining the joint anchor and basis
    /// relative to the body transform.
    pub frame2: JointFrame,
    /// The local axis about which the bodies can rotate relative to each other.
    ///
    /// By default, this is the z-axis.
    pub hinge_axis: Vector,
    /// The extents of the allowed relative rotation of the bodies.
    pub angle_limit: Option<AngleLimit>,
    /// The compliance of the point-to-point constraint (inverse of stiffness, m / N).
    pub point_compliance: Scalar,
    /// The compliance used for aligning the bodies along the [`hinge_axis`](Self::hinge_axis) (inverse of stiffness, N * m / rad).
    pub align_compliance: Scalar,
    /// The compliance of the angle limit (inverse of stiffness, N * m / rad).
    pub limit_compliance: Scalar,
    /// A motor for driving the joint.
    pub motor: AngularMotor,
}

impl EntityConstraint<2> for RevoluteJoint {
    fn entities(&self) -> [Entity; 2] {
        [self.body1, self.body2]
    }
}

impl RevoluteJoint {
    /// The default [`hinge_axis`](Self::hinge_axis) for a revolute joint.
    pub const DEFAULT_HINGE_AXIS: Vector = Vector::Z;

    /// Creates a new [`RevoluteJoint`] between two entities.
    #[inline]
    pub const fn new(body1: Entity, body2: Entity) -> Self {
        Self {
            body1,
            body2,
            frame1: JointFrame::IDENTITY,
            frame2: JointFrame::IDENTITY,
            hinge_axis: Self::DEFAULT_HINGE_AXIS,
            angle_limit: None,
            point_compliance: 0.0,
            align_compliance: 0.0,
            limit_compliance: 0.0,
            motor: AngularMotor::new_disabled(MotorModel::DEFAULT),
        }
    }

    /// Returns the local anchor point on the first body.
    ///
    /// If the [`JointAnchor`] is set to [`FromGlobal`](JointAnchor::FromGlobal),
    /// and the local anchor has not yet been computed, this will return `None`.
    #[inline]
    pub const fn local_anchor1(&self) -> Option<Vector> {
        match self.frame1.anchor {
            JointAnchor::Local(anchor) => Some(anchor),
            _ => None,
        }
    }

    /// Returns the local anchor point on the second body.
    ///
    /// If the [`JointAnchor`] is set to [`FromGlobal`](JointAnchor::FromGlobal),
    /// and the local anchor has not yet been computed, this will return `None`.
    #[inline]
    pub const fn local_anchor2(&self) -> Option<Vector> {
        match self.frame2.anchor {
            JointAnchor::Local(anchor) => Some(anchor),
            _ => None,
        }
    }

    /// Returns the local basis of the first body.
    ///
    /// If the [`JointBasis`] is set to [`FromGlobal`](JointBasis::FromGlobal),
    /// and the local basis has not yet been computed, this will return `None`.
    #[inline]
    pub fn local_basis1(&self) -> Option<Rot> {
        match self.frame1.basis {
            JointBasis::Local(basis) => Some(basis),
            _ => None,
        }
    }

    /// Returns the local basis of the second body.
    ///
    /// If the [`JointBasis`] is set to [`FromGlobal`](JointBasis::FromGlobal),
    /// and the local basis has not yet been computed, this will return `None`.
    #[inline]
    pub fn local_basis2(&self) -> Option<Rot> {
        match self.frame2.basis {
            JointBasis::Local(basis) => Some(basis),
            _ => None,
        }
    }

    /// Sets the limits of the allowed relative rotation.
    #[inline]
    pub const fn with_angle_limits(mut self, min: Scalar, max: Scalar) -> Self {
        self.angle_limit = Some(AngleLimit::new(min, max));
        self
    }

    /// Sets the motor for the joint.
    #[inline]
    pub const fn with_motor(mut self, motor: AngularMotor) -> Self {
        self.motor = motor;
        self
    }
}

impl MapEntities for RevoluteJoint {
    fn map_entities<M: EntityMapper>(&mut self, entity_mapper: &mut M) {
        self.body1 = entity_mapper.get_mapped(self.body1);
        self.body2 = entity_mapper.get_mapped(self.body2);
    }
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        PhysicsSchedule,
        update_local_frames.in_set(JointSystems::PrepareLocalFrames),
    );
}

fn update_local_frames(
    mut joints: Query<&mut RevoluteJoint, Changed<RevoluteJoint>>,
    bodies: Query<(&Position, &Rotation)>,
) {
    for mut joint in &mut joints {
        if matches!(joint.frame1.anchor, JointAnchor::Local(_))
            && matches!(joint.frame2.anchor, JointAnchor::Local(_))
            && matches!(joint.frame1.basis, JointBasis::Local(_))
            && matches!(joint.frame2.basis, JointBasis::Local(_))
        {
            continue;
        }

        let Ok([(pos1, rot1), (pos2, rot2)]) = bodies.get_many(joint.entities()) else {
            continue;
        };

        let [frame1, frame2] =
            JointFrame::compute_local(joint.frame1, joint.frame2, pos1.0, pos2.0, rot1, rot2);
        joint.frame1 = frame1;
        joint.frame2 = frame2;
    }
}
