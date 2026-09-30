use dynamics::solver::solver_body::SolverBody;

use crate::{dynamics::solver::solver_body::SolverBodyInertia, prelude::*};

/// A positional constraint applies a positional correction
/// with a given direction and magnitude at the local contact points `r1` and  `r2`.
pub trait PositionConstraint {
    /// Applies a positional impulse to two bodies.
    fn apply_positional_impulse(
        &self,
        body1: &mut SolverBody,
        body2: &mut SolverBody,
        inertia1: &SolverBodyInertia,
        inertia2: &SolverBodyInertia,
        impulse: Vector,
        r1: Vector,
        r2: Vector,
    ) {
        let inv_mass1 = inertia1.effective_inv_mass();
        let inv_mass2 = inertia2.effective_inv_mass();
        let inv_angular_inertia1 = inertia1.effective_inv_angular_inertia();
        let inv_angular_inertia2 = inertia2.effective_inv_angular_inertia();

        body1.delta_position += impulse * inv_mass1;

        {
            let delta_quat = Self::get_delta_rot(inv_angular_inertia1, r1, impulse);
            body1.delta_rotation.0 = delta_quat * body1.delta_rotation.0;
        }

        body2.delta_position -= impulse * inv_mass2;

        {
            let delta_quat = Self::get_delta_rot(inv_angular_inertia2, r2, -impulse);
            body2.delta_rotation.0 = delta_quat * body2.delta_rotation.0;
        }
    }

    /// Computes the generalized inverse mass of a body when applying a positional correction
    /// at point `r` along the vector `n`.
    fn compute_generalized_inverse_mass(
        &self,
        inverse_mass: Scalar,
        inverse_angular_inertia: SymmetricTensor,
        r: Vector,
        n: Vector,
    ) -> Scalar {
        let r_cross_n = r.cross(n); // Compute the cross product only once

        // The line below is equivalent to Eq (2) because the component-wise multiplication of a transposed vector and another vector is equal to the dot product of the two vectors.
        // a^T * b = a • b
        inverse_mass + r_cross_n.dot(inverse_angular_inertia * r_cross_n)
    }

    /// Computes the update in rotation when applying a positional correction `p` at point `r`.
    fn get_delta_rot(inverse_angular_inertia: SymmetricTensor, r: Vector, p: Vector) -> Quaternion {
        // Equation 8/9
        Quaternion::from_scaled_axis(inverse_angular_inertia * r.cross(p))
    }

    /// Computes the force acting along the constraint using the equation f = lambda * n / h^2
    fn compute_force(&self, lagrange: Scalar, direction: Vector, dt: Scalar) -> Vector {
        lagrange * direction / dt.powi(2)
    }
}
