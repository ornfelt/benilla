use crate::{
    message::{Message, MessageCursor, MessageMutIterator, Messages},
    system::{Local, ResMut, SystemParam},
};

/// Mutably reads messages of type `T` keeping track of which messages have already been read
/// by each system allowing multiple systems to read the same messages. Ideal for chains of systems
/// that all want to modify the same messages.
///
/// # Usage
///
/// [`MessageMutator`]s are usually declared as a [`SystemParam`].
/// ```
/// # use bevy_ecs::prelude::*;
///
/// #[derive(Message, Debug)]
/// pub struct MyMessage(pub u32); // Custom message type.
/// fn my_system(mut reader: MessageMutator<MyMessage>) {
///     for message in reader.read() {
///         message.0 += 1;
///         println!("received message: {:?}", message);
///     }
/// }
/// ```
///
/// # Concurrency
///
/// Multiple systems with `MessageMutator<T>` of the same message type can not run concurrently.
/// They also can not be executed in parallel with [`MessageReader`] or [`MessageWriter`].
///
/// # Clearing, Reading, and Peeking
///
/// Messages are stored in a double buffered queue that switches each frame. This switch also clears the previous
/// frame's messages. Messages should be read each frame otherwise they may be lost. For manual control over this
/// behavior, see [`Messages`].
///
/// Most of the time systems will want to use [`MessageMutator::read()`]. This function creates an iterator over
/// all messages that haven't been read yet by this system, marking the message as read in the process.
///
/// [`MessageReader`]: super::MessageReader
/// [`MessageWriter`]: super::MessageWriter
#[derive(SystemParam, Debug)]
pub struct MessageMutator<'w, 's, E: Message> {
    pub(super) reader: Local<'s, MessageCursor<E>>,
    #[system_param(validation_message = "Message not initialized")]
    messages: ResMut<'w, Messages<E>>,
}

impl<'w, 's, E: Message> MessageMutator<'w, 's, E> {
    /// Iterates over the messages this [`MessageMutator`] has not seen yet. This updates the
    /// [`MessageMutator`]'s message counter, which means subsequent message reads will not include messages
    /// that happened before now.
    pub fn read(&mut self) -> MessageMutIterator<'_, E> {
        self.reader.read_mut(&mut self.messages)
    }
}
