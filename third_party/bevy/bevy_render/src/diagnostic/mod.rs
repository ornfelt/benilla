//! Infrastructure for recording render diagnostics.
//!
//! For more info, see [`RenderDiagnosticsPlugin`].

pub(crate) mod internal;

use bevy_app::{App, Plugin, PreUpdate};

use self::internal::{sync_diagnostics, RenderDiagnosticsMutex};

/// Enables collecting render diagnostics, such as CPU/GPU elapsed time per render pass,
/// as well as pipeline statistics (number of primitives, number of shader invocations, etc).
///
/// To access the diagnostics, you can use the [`DiagnosticsStore`](bevy_diagnostic::DiagnosticsStore) resource,
/// add [`LogDiagnosticsPlugin`](bevy_diagnostic::LogDiagnosticsPlugin), or use [Tracy](https://github.com/bevyengine/bevy/blob/main/docs/profiling.md#tracy-renderqueue).
#[derive(Default)]
pub struct RenderDiagnosticsPlugin;

impl Plugin for RenderDiagnosticsPlugin {
    fn build(&self, app: &mut App) {
        // The mutex's other half and the `DiagnosticsRecorder` only reached the RenderApp.
        app.insert_resource(RenderDiagnosticsMutex::default())
            .add_systems(PreUpdate, sync_diagnostics);
    }
}
