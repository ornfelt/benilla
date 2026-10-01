//! The touch input functionality.

use bevy_ecs::{
    entity::Entity,
    message::{Message, MessageReader},
    resource::Resource,
    system::ResMut,
};
use bevy_math::Vec2;
use bevy_platform::collections::HashMap;

/// A touch input event.
///
/// ## Logic
///
/// Every time the user touches the screen, a new [`TouchPhase::Started`] event with an unique
/// identifier for the finger is generated. When the finger is lifted, the [`TouchPhase::Ended`]
/// event is generated with the same finger id.
///
/// After a [`TouchPhase::Started`] event has been emitted, there may be zero or more [`TouchPhase::Moved`]
/// events when the finger is moved or the touch pressure changes.
///
/// The finger id may be reused by the system after an [`TouchPhase::Ended`] event. The user
/// should assume that a new [`TouchPhase::Started`] event received with the same id has nothing
/// to do with the old finger and is a new finger.
///
/// A [`TouchPhase::Canceled`] event is emitted when the system has canceled tracking this
/// touch, such as when the window loses focus, or on iOS if the user moves the
/// device against their face.
///
/// ## Note
///
/// This event is the translated version of the `WindowEvent::Touch` from the `winit` crate.
/// It is available to the end user and can be used for game logic.
#[derive(Message, Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct TouchInput {
    /// The phase of the touch input.
    pub phase: TouchPhase,
    /// The position of the finger on the touchscreen.
    pub position: Vec2,
    /// The window entity registering the touch.
    pub window: Entity,
    /// Describes how hard the screen was pressed.
    ///
    /// May be [`None`] if the platform does not support pressure sensitivity.
    /// This feature is only available on **iOS** 9.0+ and **Windows** 8+.
    pub force: Option<ForceTouch>,
    /// The unique identifier of the finger.
    pub id: u64,
}

/// A force description of a [`Touch`] input.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub enum ForceTouch {
    /// On iOS, the force is calibrated so that the same number corresponds to
    /// roughly the same amount of pressure on the screen regardless of the
    /// device.
    Calibrated {
        /// The force of the touch, where a value of 1.0 represents the force of
        /// an average touch (predetermined by the system, not user-specific).
        ///
        /// The force reported by Apple Pencil is measured along the axis of the
        /// pencil. If you want a force perpendicular to the device, you need to
        /// calculate this value using the `altitude_angle` value.
        force: f64,
        /// The maximum possible force for a touch.
        ///
        /// The value of this field is sufficiently high to provide a wide
        /// dynamic range for values of the `force` field.
        max_possible_force: f64,
        /// The altitude (in radians) of the stylus.
        ///
        /// A value of 0 radians indicates that the stylus is parallel to the
        /// surface. The value of this property is Pi/2 when the stylus is
        /// perpendicular to the surface.
        altitude_angle: Option<f64>,
    },
    /// If the platform reports the force as normalized, we have no way of
    /// knowing how much pressure 1.0 corresponds to – we know it's the maximum
    /// amount of force, but as to how much force, you might either have to
    /// press really hard, or not hard at all, depending on the device.
    Normalized(f64),
}

/// A phase of a [`TouchInput`].
///
/// ## Usage
///
/// It is used to describe the phase of the touch input that is currently active.
/// This includes a phase that indicates that a touch input has started or ended,
/// or that a finger has moved. There is also a canceled phase that indicates that
/// the system canceled the tracking of the finger.
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub enum TouchPhase {
    /// A finger started to touch the touchscreen.
    Started,
    /// A finger moved over the touchscreen.
    Moved,
    /// A finger stopped touching the touchscreen.
    Ended,
    /// The system canceled the tracking of the finger.
    ///
    /// This occurs when the window loses focus, or on iOS if the user moves the
    /// device against their face.
    Canceled,
}

/// A touch input.
///
/// ## Usage
///
/// It is used to store the position of a touch input.
/// The data of the touch input comes from the [`TouchInput`] event and is being stored
/// inside of the [`Touches`] `bevy` resource.
#[derive(Debug, Clone, Copy)]
pub struct Touch {
    /// The current position of the touch input.
    position: Vec2,
}

impl From<&TouchInput> for Touch {
    fn from(input: &TouchInput) -> Touch {
        Touch {
            position: input.position,
        }
    }
}

/// A collection of [`Touch`]es.
///
/// ## Usage
///
/// It is used to create a `bevy` resource that stores the data of the touches on a touchscreen
/// and can be accessed inside of a system.
///
/// ## Updating
///
/// The resource is updated inside of the [`touch_screen_input_system`].
#[derive(Debug, Clone, Default, Resource)]
pub struct Touches {
    /// A collection of every [`Touch`] that is currently being pressed.
    pressed: HashMap<u64, Touch>,
    /// A collection of every [`Touch`] that just got pressed.
    just_pressed: HashMap<u64, Touch>,
    /// A collection of every [`Touch`] that just got released.
    just_released: HashMap<u64, Touch>,
    /// A collection of every [`Touch`] that just got canceled.
    just_canceled: HashMap<u64, Touch>,
}

impl Touches {
    /// Checks if any touch input was just pressed.
    pub fn any_just_pressed(&self) -> bool {
        !self.just_pressed.is_empty()
    }

    /// Checks if any touch input was just released.
    pub fn any_just_released(&self) -> bool {
        !self.just_released.is_empty()
    }

    /// Retrieves the position of the first currently pressed touch, if any
    pub fn first_pressed_position(&self) -> Option<Vec2> {
        // Looking for the position in `pressed`. If nothing is found, also look into `just_pressed`
        // A touch can be in `just_pressed` but not in `pressed` if it ended in the same frame it started
        self.pressed
            .values()
            .next()
            .or_else(|| self.just_pressed.values().next())
            .map(|t| t.position)
    }

    /// Processes a [`TouchInput`] event by updating the `pressed`, `just_pressed`,
    /// `just_released`, and `just_canceled` collections.
    fn process_touch_event(&mut self, event: &TouchInput) {
        match event.phase {
            TouchPhase::Started => {
                self.pressed.insert(event.id, event.into());
                self.just_pressed.insert(event.id, event.into());
            }
            TouchPhase::Moved => {
                if let Some(mut new_touch) = self.pressed.get(&event.id).cloned() {
                    new_touch.position = event.position;
                    self.pressed.insert(event.id, new_touch);
                }
            }
            TouchPhase::Ended => {
                // if touch `just_released`, add related event to it
                // the event position info is inside `pressed`, so use it unless not found
                if let Some((_, v)) = self.pressed.remove_entry(&event.id) {
                    self.just_released.insert(event.id, v);
                } else {
                    self.just_released.insert(event.id, event.into());
                }
            }
            TouchPhase::Canceled => {
                // if touch `just_canceled`, add related event to it
                // the event position info is inside `pressed`, so use it unless not found
                if let Some((_, v)) = self.pressed.remove_entry(&event.id) {
                    self.just_canceled.insert(event.id, v);
                } else {
                    self.just_canceled.insert(event.id, event.into());
                }
            }
        };
    }
}

/// Updates the [`Touches`] resource with the latest [`TouchInput`] events.
///
/// This is not clearing the `pressed` collection, because it could incorrectly mark a touch input
/// as not pressed even though it is pressed. This could happen if the touch input is not moving
/// for a single frame and would therefore be marked as not pressed, because this function is
/// called on every single frame no matter if there was an event or not.
///
/// ## Differences
///
/// The main difference between the [`TouchInput`] event and the [`Touches`] resource is that
/// the latter has convenient functions like [`Touches::any_just_pressed`] and
/// [`Touches::any_just_released`].
pub fn touch_screen_input_system(
    mut touch_state: ResMut<Touches>,
    mut touch_input_reader: MessageReader<TouchInput>,
) {
    if !touch_state.just_pressed.is_empty() {
        touch_state.just_pressed.clear();
    }
    if !touch_state.just_released.is_empty() {
        touch_state.just_released.clear();
    }
    if !touch_state.just_canceled.is_empty() {
        touch_state.just_canceled.clear();
    }

    if !touch_input_reader.is_empty() {
        for event in touch_input_reader.read() {
            touch_state.process_touch_event(event);
        }
    }
}
