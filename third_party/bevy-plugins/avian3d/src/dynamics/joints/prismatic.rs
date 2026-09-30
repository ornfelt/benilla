use crate::{
    dynamics::joints::{EntityConstraint, JointSystems, motor::LinearMotor},
    prelude::*,
};
use bevy::{
    ecs::{
        entity::{EntityMapper, MapEntities},
        reflect::ReflectMapEntities,
    },
    prelude::*,
};

/// A prismatic [joint](dynamics::joints) prevents any relative movement between two bodies,
/// except for translation along the [`slider_axis`](Self::slider_axis).
///
/// This can be useful for things like elevators, pistons, sliding doors and moving platforms.
///
/// Each prismatic joint is defined by a [`JointFrame`] on each body, a [`slider_axis`](Self::slider_axis)
/// along which the bodies can translate, and an optional [`DistanceLimit`] that defines the extents
/// of the allowed translation.
///
#[doc = include_str!("./images/prismatic_joint.svg")]
///
/// The joint can also include a [`LinearMotor`] for driving the translation along the [`slider_axis`](Self::slider_axis).
/// Use this to create pistons, elevators, or other linear motion mechanisms.
#[derive(Component, Clone, Debug, PartialEq, Reflect)]
#[reflect(Component, Debug, MapEntities, PartialEq)]
#[doc(alias = "SliderJoint")]
pub struct PrismaticJoint {
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
    /// The local axis along which the bodies can translate relative to each other.
    ///
    /// By default, this is the x-axis.
    pub slider_axis: Vector,
    /// The extents of the allowed relative translation along the [`slider_axis`](Self::slider_axis).
    pub limits: Option<DistanceLimit>,
    /// The compliance used for aligning the positions of the bodies to the [`slider_axis`](Self::slider_axis) (inverse of stiffness, m / N).
    pub align_compliance: Scalar,
    /// The compliance of the angular constraint (inverse of stiffness, N * m / rad).
    pub angle_compliance: Scalar,
    /// The compliance of the distance limit (inverse of stiffness, m / N).
    pub limit_compliance: Scalar,
    /// A motor for driving the joint.
    pub motor: LinearMotor,
}

impl EntityConstraint<2> for PrismaticJoint {
    fn entities(&self) -> [Entity; 2] {
        [self.body1, self.body2]
    }
}

impl PrismaticJoint {
    /// The default [`slider_axis`](Self::slider_axis) for a prismatic joint.
    pub const DEFAULT_SLIDER_AXIS: Vector = Vector::X;

    /// Creates a new [`PrismaticJoint`] between two entities.
    #[inline]
    pub const fn new(body1: Entity, body2: Entity) -> Self {
        Self {
            body1,
            body2,
            frame1: JointFrame::IDENTITY,
            frame2: JointFrame::IDENTITY,
            slider_axis: Self::DEFAULT_SLIDER_AXIS,
            limits: None,
            align_compliance: 0.0,
            angle_compliance: 0.0,
            limit_compliance: 0.0,
            motor: LinearMotor::new_disabled(MotorModel::DEFAULT),
        }
    }

    /// Sets the local anchor point on the first body.
    ///
    /// This configures the [`JointAnchor`] of the first [`JointFrame`].
    #[inline]
    pub const fn with_local_anchor1(mut self, anchor: Vector) -> Self {
        self.frame1.anchor = JointAnchor::Local(anchor);
        self
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

    /// Sets the translational limits along the [`slider_axis`](Self::slider_axis).
    #[inline]
    pub const fn with_limits(mut self, min: Scalar, max: Scalar) -> Self {
        self.limits = Some(DistanceLimit::new(min, max));
        self
    }

    /// Sets the motor for the joint.
    #[inline]
    pub const fn with_motor(mut self, motor: LinearMotor) -> Self {
        self.motor = motor;
        self
    }
}

impl MapEntities for PrismaticJoint {
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
    mut joints: Query<&mut PrismaticJoint, Changed<PrismaticJoint>>,
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
