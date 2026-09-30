use bevy_app::{App, MainScheduleOrder, Plugin, PreStartup, PreUpdate, SubApp};
use bevy_ecs::{message::Messages, world::FromWorld};
use bevy_utils::once;
use log::warn;

use crate::state::{
    setup_state_transitions_in_world, FreelyMutableState, NextState, State, StateTransition,
    StateTransitionEvent,
};

/// State installation methods for [`App`] and [`SubApp`].
pub trait AppExtStates {
    /// Initializes a [`State`] with standard starting values.
    ///
    /// This method is idempotent: it has no effect when called again using the same generic type.
    ///
    /// Adds [`State<S>`] and [`NextState<S>`] resources, and enables use of the [`OnEnter`](crate::state::OnEnter)
    /// and [`OnExit`](crate::state::OnExit) schedules.
    /// These schedules are triggered before [`Update`](bevy_app::Update) and at startup.
    ///
    /// If you would like to control how other systems run based on the current state, you can
    /// emulate this behavior using the [`in_state`](crate::condition::in_state) [`SystemCondition`](bevy_ecs::prelude::SystemCondition).
    ///
    /// Note that you can also apply state transitions at other points in the schedule
    /// by triggering the [`StateTransition`](struct@StateTransition) schedule manually.
    ///
    /// The use of any states requires the presence of [`StatesPlugin`] (which is included in `DefaultPlugins`).
    fn init_state<S: FreelyMutableState + FromWorld>(&mut self) -> &mut Self;

    /// Inserts a specific [`State`] to the current [`App`] and overrides any [`State`] previously
    /// added of the same type.
    ///
    /// Adds [`State<S>`] and [`NextState<S>`] resources, and enables use of the [`OnEnter`](crate::state::OnEnter)
    /// and [`OnExit`](crate::state::OnExit) schedules.
    /// These schedules are triggered before [`Update`](bevy_app::Update) and at startup.
    ///
    /// If you would like to control how other systems run based on the current state, you can
    /// emulate this behavior using the [`in_state`](crate::condition::in_state) [`SystemCondition`](bevy_ecs::prelude::SystemCondition).
    ///
    /// Note that you can also apply state transitions at other points in the schedule
    /// by triggering the [`StateTransition`](struct@StateTransition) schedule manually.
    fn insert_state<S: FreelyMutableState>(&mut self, state: S) -> &mut Self;
}

/// Separate function to only warn once for all state installation methods.
fn warn_if_no_states_plugin_installed(app: &SubApp) {
    if !app.is_plugin_added::<StatesPlugin>() {
        once!(warn!(
            "States were added to the app, but `StatesPlugin` is not installed."
        ));
    }
}

impl AppExtStates for SubApp {
    fn init_state<S: FreelyMutableState + FromWorld>(&mut self) -> &mut Self {
        warn_if_no_states_plugin_installed(self);
        if !self.world().contains_resource::<State<S>>() {
            self.init_resource::<State<S>>()
                .init_resource::<NextState<S>>()
                .add_message::<StateTransitionEvent<S>>();
            let schedule = self.get_schedule_mut(StateTransition).expect(
                "The `StateTransition` schedule is missing. Did you forget to add StatesPlugin or DefaultPlugins before calling init_state?"
            );
            S::register_state(schedule);
            let state = self.world().resource::<State<S>>().get().clone();
            self.world_mut().write_message(StateTransitionEvent {
                exited: None,
                entered: Some(state),
                // makes no difference: the state didn't exist before anyways
                allow_same_state_transitions: true,
            });
        } else {
            let name = core::any::type_name::<S>();
            warn!("State {name} is already initialized.");
        }

        self
    }

    fn insert_state<S: FreelyMutableState>(&mut self, state: S) -> &mut Self {
        warn_if_no_states_plugin_installed(self);
        if !self.world().contains_resource::<State<S>>() {
            self.insert_resource::<State<S>>(State::new(state.clone()))
                .init_resource::<NextState<S>>()
                .add_message::<StateTransitionEvent<S>>();
            let schedule = self.get_schedule_mut(StateTransition).expect(
                "The `StateTransition` schedule is missing. Did you forget to add StatesPlugin or DefaultPlugins before calling insert_state?"
            );
            S::register_state(schedule);
            self.world_mut().write_message(StateTransitionEvent {
                exited: None,
                entered: Some(state),
                // makes no difference: the state didn't exist before anyways
                allow_same_state_transitions: true,
            });
        } else {
            // Overwrite previous state and initial event
            self.insert_resource::<State<S>>(State::new(state.clone()));
            self.world_mut()
                .resource_mut::<Messages<StateTransitionEvent<S>>>()
                .clear();
            self.world_mut().write_message(StateTransitionEvent {
                exited: None,
                entered: Some(state),
                // Not configurable for the moment. This controls whether inserting a state with the same value as a pre-existing state should run state transitions.
                // Leaving it at `true` makes state insertion idempotent. Neat!
                allow_same_state_transitions: true,
            });
        }

        self
    }
}

impl AppExtStates for App {
    fn init_state<S: FreelyMutableState + FromWorld>(&mut self) -> &mut Self {
        self.main_mut().init_state::<S>();
        self
    }

    fn insert_state<S: FreelyMutableState>(&mut self, state: S) -> &mut Self {
        self.main_mut().insert_state::<S>(state);
        self
    }
}

/// Registers the [`StateTransition`] schedule in the [`MainScheduleOrder`] to enable state processing.
#[derive(Default)]
pub struct StatesPlugin;

impl Plugin for StatesPlugin {
    fn build(&self, app: &mut App) {
        let mut schedule = app.world_mut().resource_mut::<MainScheduleOrder>();
        schedule.insert_after(PreUpdate, StateTransition);
        schedule.insert_startup_before(PreStartup, StateTransition);
        setup_state_transitions_in_world(app.world_mut());
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        app::StatesPlugin,
        state::{State, StateTransition, StateTransitionEvent},
    };
    use bevy_app::App;
    use bevy_ecs::message::Messages;
    use bevy_state_macros::States;

    use super::AppExtStates;

    #[derive(States, Default, PartialEq, Eq, Hash, Debug, Clone)]
    enum TestState {
        #[default]
        A,
        B,
        C,
    }

    #[test]
    fn insert_state_can_overwrite_init_state() {
        let mut app = App::new();
        app.add_plugins(StatesPlugin);

        app.init_state::<TestState>();
        app.insert_state(TestState::B);

        let world = app.world_mut();
        world.run_schedule(StateTransition);

        assert_eq!(world.resource::<State<TestState>>().0, TestState::B);
        let events = world.resource::<Messages<StateTransitionEvent<TestState>>>();
        assert_eq!(events.len(), 1);
        let mut reader = events.get_cursor();
        let last = reader.read(events).last().unwrap();
        assert_eq!(last.exited, None);
        assert_eq!(last.entered, Some(TestState::B));
    }

    #[test]
    fn insert_state_can_overwrite_insert_state() {
        let mut app = App::new();
        app.add_plugins(StatesPlugin);

        app.insert_state(TestState::B);
        app.insert_state(TestState::C);

        let world = app.world_mut();
        world.run_schedule(StateTransition);

        assert_eq!(world.resource::<State<TestState>>().0, TestState::C);
        let events = world.resource::<Messages<StateTransitionEvent<TestState>>>();
        assert_eq!(events.len(), 1);
        let mut reader = events.get_cursor();
        let last = reader.read(events).last().unwrap();
        assert_eq!(last.exited, None);
        assert_eq!(last.entered, Some(TestState::C));
    }
}
