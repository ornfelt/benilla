mod freely_mutable_state;
mod resources;
mod states;
mod transitions;

pub use bevy_state_macros::*;
pub use freely_mutable_state::*;
pub use resources::*;
pub use states::*;
pub use transitions::*;

#[cfg(test)]
mod tests {
    use bevy_ecs::{message::MessageRegistry, prelude::*};
    use bevy_state_macros::States;

    use super::*;

    #[derive(States, PartialEq, Eq, Debug, Default, Hash, Clone)]
    enum SimpleState {
        #[default]
        A,
    }

    #[derive(Resource, Default, PartialEq, Debug)]
    struct TransitionCounter {
        exit: u8,
        enter: u8,
    }

    #[test]
    fn same_state_transition_should_emit_event_and_run_schedules() {
        let mut world = World::new();
        setup_state_transitions_in_world(&mut world);
        MessageRegistry::register_message::<StateTransitionEvent<SimpleState>>(&mut world);
        world.init_resource::<State<SimpleState>>();
        let mut schedules = world.resource_mut::<Schedules>();
        let apply_changes = schedules.get_mut(StateTransition).unwrap();
        SimpleState::register_state(apply_changes);

        let mut on_exit = Schedule::new(OnExit(SimpleState::A));
        on_exit.add_systems(|mut c: ResMut<TransitionCounter>| c.exit += 1);
        schedules.insert(on_exit);
        let mut on_enter = Schedule::new(OnEnter(SimpleState::A));
        on_enter.add_systems(|mut c: ResMut<TransitionCounter>| c.enter += 1);
        schedules.insert(on_enter);
        world.insert_resource(TransitionCounter::default());

        world.run_schedule(StateTransition);
        assert_eq!(world.resource::<State<SimpleState>>().0, SimpleState::A);
        assert!(world
            .resource::<Messages<StateTransitionEvent<SimpleState>>>()
            .is_empty());

        world.insert_resource(TransitionCounter::default());
        world.insert_resource(NextState::Pending(SimpleState::A));
        world.run_schedule(StateTransition);
        assert_eq!(world.resource::<State<SimpleState>>().0, SimpleState::A);
        assert_eq!(
            *world.resource::<TransitionCounter>(),
            TransitionCounter { exit: 1, enter: 1 }
        );
        assert_eq!(
            world
                .resource::<Messages<StateTransitionEvent<SimpleState>>>()
                .len(),
            1
        );
    }
}
