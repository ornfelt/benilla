#![cfg_attr(docsrs, feature(doc_cfg))]
#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://bevy.org/assets/icon.png",
    html_favicon_url = "https://bevy.org/assets/icon.png"
)]
#![no_std]

//! This crate provides a straightforward solution for integrating diagnostics in the [Bevy game engine](https://bevy.org/).
//! It allows users to easily add diagnostic functionality to their Bevy applications, enhancing
//! their ability to monitor and optimize their game's.

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

mod diagnostic;
mod frame_count_diagnostics_plugin;
#[cfg(feature = "sysinfo_plugin")]
mod system_information_diagnostics_plugin;

pub use diagnostic::*;

pub use frame_count_diagnostics_plugin::{update_frame_count, FrameCount, FrameCountPlugin};
#[cfg(feature = "sysinfo_plugin")]
pub use system_information_diagnostics_plugin::SystemInfo;

use bevy_app::prelude::*;

/// Adds core diagnostics resources to an App.
#[derive(Default)]
pub struct DiagnosticsPlugin;

impl Plugin for DiagnosticsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DiagnosticsStore>();

        #[cfg(feature = "sysinfo_plugin")]
        app.init_resource::<SystemInfo>();
    }
}

/// Default max history length for new diagnostics.
pub const DEFAULT_MAX_HISTORY_LENGTH: usize = 120;
