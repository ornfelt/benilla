//! Utilities for hotpatching code.
extern crate alloc;

use alloc::sync::Arc;

use bevy_ecs::{
    change_detection::DetectChangesMut, message::MessageWriter, system::ResMut, HotPatchChanges,
    HotPatched,
};
use dioxus_devtools::connect_subsecond;
use dioxus_devtools::subsecond;

pub use dioxus_devtools::subsecond::{call, HotFunction};

use crate::{Last, Plugin};

/// Plugin connecting to Dioxus CLI to enable hot patching.
#[derive(Default)]
pub struct HotPatchPlugin;

impl Plugin for HotPatchPlugin {
    fn build(&self, app: &mut crate::App) {
        let (sender, receiver) = crossbeam_channel::bounded::<HotPatched>(1);

        // Connects to the dioxus CLI that will handle rebuilds
        // This will open a connection to the dioxus CLI to receive updated jump tables
        // Sends a `HotPatched` message through the channel when the jump table is updated
        connect_subsecond();
        subsecond::register_handler(Arc::new(move || {
            sender.send(HotPatched).unwrap();
        }));

        // Adds a system that will read the channel for new `HotPatched` messages, send the message, and update change detection.
        app.init_resource::<HotPatchChanges>()
            .add_message::<HotPatched>()
            .add_systems(
                Last,
                move |mut hot_patched_writer: MessageWriter<HotPatched>,
                      mut res: ResMut<HotPatchChanges>| {
                    if receiver.try_recv().is_ok() {
                        hot_patched_writer.write_default();
                        res.set_changed();
                    }
                },
            );
    }
}
