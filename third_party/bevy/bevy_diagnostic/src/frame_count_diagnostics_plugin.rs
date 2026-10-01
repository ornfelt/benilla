use bevy_app::prelude::*;
use bevy_ecs::prelude::*;

/// Maintains a count of frames rendered since the start of the application.
///
/// [`FrameCount`] is incremented during [`Last`], providing predictable
/// behavior: it will be 0 during the first update, 1 during the next, and so forth.
///
/// # Overflows
///
/// [`FrameCount`] will wrap to 0 after exceeding [`u32::MAX`]. Within reasonable
/// assumptions, one may exploit wrapping arithmetic to determine the number of frames
/// that have elapsed between two observations – see [`u32::wrapping_sub()`].
#[derive(Debug, Default, Resource, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FrameCount(pub u32);

/// Adds frame counting functionality to Apps.
#[derive(Default)]
pub struct FrameCountPlugin;

impl Plugin for FrameCountPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FrameCount>();
        app.add_systems(Last, update_frame_count);
    }
}

/// A system used to increment [`FrameCount`] with wrapping addition.
///
/// See [`FrameCount`] for more details.
pub fn update_frame_count(mut frame_count: ResMut<FrameCount>) {
    frame_count.0 = frame_count.0.wrapping_add(1);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_counter_update() {
        let mut app = App::new();
        app.add_plugins(FrameCountPlugin);
        app.update();

        let frame_count = app.world().resource::<FrameCount>();
        assert_eq!(1, frame_count.0);
    }
}
