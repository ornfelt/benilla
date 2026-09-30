#![no_std]

//! In Bevy, states are app-wide finite state machines that are generally used to model the large scale structure of your program: whether a game is paused, if the player is in combat, if assets are loaded and so on.
//!
//! A [`States`](state::States) type can only be changed by manually setting the [`NextState<S>`](state::NextState) resource.
//!
//! Most of the utilities around state involve running systems during transitions between states, or
//! determining whether to run certain systems, though they can be used more directly as well.
//!
//! Specifically, Bevy provides the following utilities:
//!
//! - 2 Transition Schedules - [`OnEnter<S>`](crate::state::OnEnter) and [`OnExit<S>`](crate::state::OnExit) - which are used
//!   to trigger systems specifically during matching transitions.
//! - A [`StateTransitionEvent<S>`](crate::state::StateTransitionEvent) that gets fired when a given state changes.
//! - The [`in_state<S>`](crate::condition::in_state) run condition - which is used
//!   to determine whether a system should run based on the current state.

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

// Required to make proc macros work in bevy itself.
extern crate self as bevy_state;

#[cfg(feature = "bevy_app")]
/// Provides [`App`](bevy_app::App) and [`SubApp`](bevy_app::SubApp) with state installation methods
pub mod app;
/// Provides definitions for the runtime conditions that interact with the state system
pub mod condition;
/// Provides definitions for the basic traits required by the state system
pub mod state;

/// The state prelude.
///
/// This includes the most common types in this crate, re-exported for your convenience.
pub mod prelude {
    #[cfg(feature = "bevy_app")]
    #[doc(hidden)]
    pub use crate::app::AppExtStates;

    #[doc(hidden)]
    pub use crate::{
        condition::*,
        state::{
            last_transition, EnterSchedules, ExitSchedules, NextState, OnEnter, OnExit, State,
            StateTransition, StateTransitionEvent, States, TransitionSchedules,
        },
    };
}

#[cfg(test)]
mod tests {
    use bevy_app::{App, PreStartup};
    use bevy_ecs::{
        resource::Resource,
        system::{Commands, ResMut},
    };
    use bevy_state_macros::States;

    use crate::{
        app::{AppExtStates, StatesPlugin},
        state::OnEnter,
    };

    #[test]
    fn state_transition_runs_before_pre_startup() {
        // This test is not really a "requirement" of states (we could run state transitions after
        // PreStartup), but this is the current policy and it is useful to ensure we are following
        // it if we ever change how we initialize stuff.

        let mut app = App::new();
        app.add_plugins(StatesPlugin);

        #[derive(States, Default, PartialEq, Eq, Hash, Debug, Clone)]
        enum TestState {
            #[default]
            A,
            #[expect(
                dead_code,
                reason = "This struct is used as a compilation test to test the derive macros, and as such is intentionally never constructed."
            )]
            B,
        }

        #[derive(Resource, Default, PartialEq, Eq, Debug)]
        struct Thingy(usize);

        app.init_state::<TestState>();

        app.add_systems(OnEnter(TestState::A), move |mut commands: Commands| {
            commands.init_resource::<Thingy>();
        });

        app.add_systems(PreStartup, move |mut thingy: ResMut<Thingy>| {
            // This system will fail if it runs before OnEnter.
            thingy.0 += 1;
        });

        app.update();

        // This assert only succeeds if first OnEnter(TestState::A) runs, followed by PreStartup.
        assert_eq!(app.world().resource::<Thingy>(), &Thingy(1));
    }
}
