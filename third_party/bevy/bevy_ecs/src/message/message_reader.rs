use crate::{
    message::{Message, MessageCursor, MessageIterator, Messages},
    system::{Local, Res, SystemParam},
};

/// Reads [`Message`]s of type `T` in order and tracks which messages have already been read.
///
/// # Concurrency
///
/// Unlike [`MessageWriter<T>`], systems with `MessageReader<T>` param can be executed concurrently
/// (but not concurrently with `MessageWriter<T>` or `MessageMutator<T>` systems for the same message type).
///
/// [`MessageWriter<T>`]: super::MessageWriter
#[derive(SystemParam, Debug)]
pub struct MessageReader<'w, 's, E: Message> {
    pub(super) reader: Local<'s, MessageCursor<E>>,
    #[system_param(validation_message = "Message not initialized")]
    messages: Res<'w, Messages<E>>,
}

impl<'w, 's, E: Message> MessageReader<'w, 's, E> {
    /// Iterates over the messages this [`MessageReader`] has not seen yet. This updates the
    /// [`MessageReader`]'s message counter, which means subsequent message reads will not include messages
    /// that happened before now.
    pub fn read(&mut self) -> MessageIterator<'_, E> {
        self.reader.read(&self.messages)
    }

    /// Returns `true` if there are no messages available to read.
    ///
    /// # Example
    ///
    /// The following example shows a useful pattern where some behavior is triggered if new messages are available.
    /// [`MessageReader::clear()`] is used so the same messages don't re-trigger the behavior the next time the system runs.
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// #[derive(Message)]
    /// struct Collision;
    ///
    /// fn play_collision_sound(mut messages: MessageReader<Collision>) {
    ///     if !messages.is_empty() {
    ///         messages.clear();
    ///         // Play a sound
    ///     }
    /// }
    /// # bevy_ecs::system::assert_is_system(play_collision_sound);
    /// ```
    pub fn is_empty(&self) -> bool {
        self.reader.is_empty(&self.messages)
    }

    /// Consumes all available messages.
    ///
    /// This means these messages will not appear in calls to [`MessageReader::read()`] and
    /// [`MessageReader::is_empty()`] will return `true`.
    ///
    /// For usage, see [`MessageReader::is_empty()`].
    pub fn clear(&mut self) {
        self.reader.clear(&self.messages);
    }
}
