use alloc::sync::Arc;
use std::sync::Mutex;

use bevy_diagnostic::{Diagnostic, DiagnosticMeasurement, DiagnosticPath, DiagnosticsStore};
use bevy_ecs::resource::Resource;
use bevy_ecs::system::{Res, ResMut};
use bevy_platform::time::Instant;

/// Resource which stores render diagnostics of the most recent frame.
#[derive(Debug, Default, Clone, Resource)]
pub struct RenderDiagnostics(Vec<RenderDiagnostic>);

/// A render diagnostic which has been recorded, but not yet stored in [`DiagnosticsStore`].
#[derive(Debug, Clone, Resource)]
pub struct RenderDiagnostic {
    pub path: DiagnosticPath,
    pub suffix: &'static str,
    pub value: f64,
}

/// Stores render diagnostics before they can be synced with the main app.
///
/// This mutex is locked twice per frame:
///  1. in `PreUpdate`, during [`sync_diagnostics`],
///  2. after rendering has finished and statistics have been downloaded from GPU.
#[derive(Debug, Default, Clone, Resource)]
pub struct RenderDiagnosticsMutex(pub(crate) Arc<Mutex<Option<RenderDiagnostics>>>);

/// Updates render diagnostics measurements.
pub fn sync_diagnostics(mutex: Res<RenderDiagnosticsMutex>, mut store: ResMut<DiagnosticsStore>) {
    let Some(diagnostics) = mutex.0.lock().ok().and_then(|mut v| v.take()) else {
        return;
    };

    let time = Instant::now();

    for diagnostic in &diagnostics.0 {
        if store.get(&diagnostic.path).is_none() {
            store.add(Diagnostic::new(diagnostic.path.clone()).with_suffix(diagnostic.suffix));
        }

        store
            .get_mut(&diagnostic.path)
            .unwrap()
            .add_measurement(DiagnosticMeasurement {
                time,
                value: diagnostic.value,
            });
    }
}
