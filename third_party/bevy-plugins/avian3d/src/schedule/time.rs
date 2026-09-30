//! Clocks used for tracking physics simulation time.

use crate::prelude::*;
use bevy::prelude::*;

/// The clock representing physics time, following the [`Time`] clock used by the schedule that physics runs in.
///
/// In [`FixedPostUpdate`] and other fixed schedules, this uses [`Time<Fixed>`](Fixed), while in schedules such as [`Update`],
/// a variable timestep following [`Time<Virtual>`](Virtual) is used.
///
/// [`Time<Physics>`](Physics) is automatically set as the generic [`Time`] resource for
/// the [`PhysicsSchedule`].
///
/// # Usage
///
/// ## Stepping Physics
///
/// To advance the simulation by a certain amount of time instantly, you can advance the
/// [`Time<Physics>`](Physics) clock and manually run the [`PhysicsSchedule`] in an exclusive system:
///
/// ```
/// use avian3d::prelude::*;
/// use bevy::prelude::*;
/// use core::time::Duration;
///
/// fn run_physics(world: &mut World) {
///     // Advance the simulation by 10 steps at 120 Hz
///     for _ in 0..10 {
///         world
///             .resource_mut::<Time<Physics>>()
///             .advance_by(Duration::from_secs_f64(1.0 / 120.0));
///         world.run_schedule(PhysicsSchedule);
///     }
/// }
/// ```
///
/// ## When to Multiply by Delta Time?
///
/// Schedules like `Update` use a variable timestep, which can often cause frame rate dependent
/// behavior when moving bodies. One way to help address the issue is by multiplying by delta time.
///
/// In general, if you're doing a *continuous* operation, you should always multiply by delta time,
/// but for *instantaneous* operations it's not necessary.
///
/// Continuous operations move or accelerate bodies over time:
///
/// ```
/// # use bevy::math::Vec3;
/// #
/// # let mut position = Vec3::default();
/// # let mut velocity = Vec3::default();
/// # let mut acceleration = Vec3::default();
/// # let mut delta_time = 1.0 / 60.0;
/// #
/// // Move continuously over time
/// position += velocity * delta_time;
/// // Accelerate continuously
/// velocity += acceleration * delta_time;
/// ```
///
/// Instantaneous operations apply a singular sudden burst of velocity
/// or set it to a specific value:
///
/// ```
/// # use bevy::math::Vec3;
/// #
/// # let mut velocity = Vec3::default();
/// # let mut impulse = Vec3::default();
/// # let mut target_velocity = Vec3::default();
/// #
/// // Apply a burst of speed once (jumps, explosions and so on)
/// velocity += impulse;
/// // Set velocity to a specific value
/// velocity = target_velocity;
/// ```
///
/// For systems using a fixed timestep, using delta time is not necessary for frame rate
/// independence, but it's still recommended so that the physical units are more logical.

#[derive(Reflect, Clone, Copy, Debug, PartialEq)]
#[reflect(Debug, PartialEq)]
pub struct Physics {
    paused: bool,
    relative_speed: f64,
}

impl Default for Physics {
    fn default() -> Self {
        Self {
            paused: false,
            relative_speed: 1.0,
        }
    }
}

/// An extension trait for [`Time<Physics>`](Physics).
pub trait PhysicsTime {
    /// Returns the speed of physics relative to your system clock as an `f64`.
    /// This is also known as "time scaling" or "time dilation" in other engines.
    ///
    /// The speed impacts the accuracy of the simulation, and large values may
    /// cause jittering or missed collisions. You can improve simulation consistency
    /// by adjusting your timestep at the cost of performance.
    fn relative_speed_f64(&self) -> f64;

    /// Returns `true` if the physics clock is currently paused.
    fn is_paused(&self) -> bool;
}

impl PhysicsTime for Time<Physics> {
    fn relative_speed_f64(&self) -> f64 {
        self.context().relative_speed
    }

    fn is_paused(&self) -> bool {
        self.context().paused
    }
}

/// The clock representing physics substep time. It is updated based on
/// [`Time<Physics>`](Physics) and the [`SubstepCount`] resource.
///
/// The clock is automatically set as the generic `Time` resource for
/// the [`SubstepSchedule`].
#[derive(Reflect, Clone, Copy, Debug, Default, PartialEq)]
pub struct Substeps;

pub(crate) trait TimePrecisionAdjusted {
    /// Returns how much time has advanced since the last update
    /// as [`Scalar`] seconds.
    fn delta_seconds_adjusted(&self) -> Scalar;
}

impl TimePrecisionAdjusted for Time {
    /// Returns how much time has advanced since the last [`update`](#method.update)
    /// as [`Scalar`] seconds.
    fn delta_seconds_adjusted(&self) -> Scalar {
        self.delta_secs()
    }
}
