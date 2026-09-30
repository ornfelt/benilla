use crate::state::{State, States};
use bevy_ecs::system::Res;

/// Generates a [`SystemCondition`](bevy_ecs::prelude::SystemCondition)-satisfying closure that returns `true`
/// if the state machine is currently in `state`.
///
/// Will return `false` if the state does not exist or if not in `state`.
///
/// # Example
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_state::prelude::*;
/// # use bevy_app::{App, Update};
/// # use bevy_state::app::StatesPlugin;
/// # #[derive(Resource, Default)]
/// # struct Counter(u8);
/// # let mut app = App::new();
/// # app
/// #   .init_resource::<Counter>()
/// #   .add_plugins(StatesPlugin);
/// #[derive(States, Clone, Copy, Default, Eq, PartialEq, Hash, Debug)]
/// enum GameState {
///     #[default]
///     Playing,
///     Paused,
/// }
///
/// app
///     .init_state::<GameState>()
///     .add_systems(Update, (
///         // `in_state` will only return true if the
///         // given state equals the given value
///         play_system.run_if(in_state(GameState::Playing)),
///         pause_system.run_if(in_state(GameState::Paused)),
///     ));
///
/// fn play_system(mut counter: ResMut<Counter>) {
///     counter.0 += 1;
/// }
///
/// fn pause_system(mut counter: ResMut<Counter>) {
///     counter.0 -= 1;
/// }
///
/// // We default to `GameState::Playing` so `play_system` runs
/// app.update();
/// assert_eq!(app.world().resource::<Counter>().0, 1);
///
/// app.insert_state(GameState::Paused);
///
/// // Now that we are in `GameState::Pause`, `pause_system` will run
/// app.update();
/// assert_eq!(app.world().resource::<Counter>().0, 0);
/// ```
pub fn in_state<S: States>(state: S) -> impl FnMut(Option<Res<State<S>>>) -> bool + Clone {
    move |current_state: Option<Res<State<S>>>| match current_state {
        Some(current_state) => *current_state == state,
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use bevy_ecs::schedule::{IntoScheduleConfigs, Schedule, SystemCondition};

    use crate::prelude::*;
    use bevy_state_macros::States;

    #[derive(States, PartialEq, Eq, Debug, Default, Hash, Clone)]
    enum TestState {
        #[default]
        A,
        B,
    }

    fn test_system() {}

    // Ensure distributive_run_if compiles with the common conditions.
    #[test]
    fn distributive_run_if_compiles() {
        Schedule::default().add_systems(
            (test_system, test_system)
                .distributive_run_if(in_state(TestState::A).or(in_state(TestState::B))),
        );
    }
}
