//! The glyph cache in Bevy: building it, following the window, and uploading each new cell as a
//! sub-rect write into the sheet's texture on the gfx device, never through `Assets<Image>`
//! ([`crate::ui_text::pack`] says why).

use std::sync::{Arc, Mutex};

use bevy::image::Image;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use benilla_assets::WorldAssets;

use super::{TextEngine, UiFontAtlas};
use crate::ui_text::pack::CellUpload;

/// Cells rasterized this frame, waiting for the gfx device.
#[derive(Resource, Default)]
struct GlyphUploadQueue(Vec<CellUpload>);

/// Loads the client's TTFs and drives the on-demand glyph cache.
pub(crate) struct UiTextPlugin;

impl Plugin for UiTextPlugin {
    fn build(&self, app: &mut App) {
        // `init` retries each Update until the patch chain and the window's real `scale_factor`
        // exist. `publish_sheet` runs in `Last`, after every producer of text quads, so a cell
        // rasterized this frame is queued before the frame is drawn.
        app.init_resource::<GlyphUploadQueue>()
            .add_systems(Update, init)
            .add_systems(Last, publish_sheet);
        // The cells go to the gfx device's sub-rect writes.
        app.add_systems(Last, hand_cells_to_gfx.after(publish_sheet));
    }
}

/// Build the engine on the first frame with both the patch chain and the window's `scale_factor`.
fn init(
    mut commands: Commands,
    world_assets: Option<Res<WorldAssets>>,
    images: Res<Assets<Image>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    existing: Option<Res<UiFontAtlas>>,
) {
    if existing.is_some() {
        return;
    }
    let (Some(world_assets), Ok(window)) = (world_assets, windows.single()) else {
        return;
    };
    let Some(engine) = TextEngine::load(&world_assets, &images, window.scale_factor()) else {
        return;
    };
    commands.insert_resource(UiFontAtlas {
        engine: Arc::new(Mutex::new(engine)),
        generation: 0,
        ellipsis: crate::ui_text::EllipsisMemo::default(),
    });
}

/// The frame boundary: follow the window's DPI, carry out a pending reset, create the sheet's
/// texture once, and queue this frame's cells for the device. A reset waits for this boundary
/// so no UV moves while quads that use it are still being pushed.
fn publish_sheet(
    atlas: Option<ResMut<UiFontAtlas>>,
    mut images: ResMut<Assets<Image>>,
    mut queue: ResMut<GlyphUploadQueue>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    let Some(mut atlas) = atlas else {
        return;
    };
    let dpi = windows.single().map_or(1.0, Window::scale_factor);
    let (generation, dpi_moved) = {
        let mut e = atlas.lock();
        let dpi_moved = (e.dpi - dpi).abs() > 1e-6;
        e.dpi = dpi;
        if e.reset_pending {
            e.reset_pending = false;
            e.generation += 1;
            e.stats.resets += 1;
            e.chars.clear();
            e.cells.clear();
            e.sheet.reset();
        }
        let (announce, mut cells) = e.sheet.take_pending();
        if announce {
            // Inserted once on the reserved handle, never `get_mut` after: that would recreate
            // the texture and blank every glyph. A failed insert has no recovery.
            let _ = images.insert(e.sheet.handle().id(), crate::ui_text::pack::sheet_image());
        }
        queue.0.append(&mut cells);
        (e.generation, dpi_moved)
    };
    // A DPI change does stale the ellipsis memo: its answers fit a box at the old raster size.
    if dpi_moved {
        atlas.ellipsis = crate::ui_text::EllipsisMemo::default();
    }
    atlas.generation = generation;
    report_cache(&atlas);
}

/// For gfx: the frame's cells as sub-rect writes into the sheet on the gfx device.
fn hand_cells_to_gfx(
    mut queue: ResMut<GlyphUploadQueue>,
    mut writes: ResMut<benilla_gfx::GfxTextureWrites>,
) {
    writes
        .0
        .extend(queue.0.drain(..).map(|u| benilla_gfx::GfxTextureWrite {
            image: u.image,
            x: u.x,
            y: u.y,
            width: u.w,
            height: u.h,
            data: u.rgba,
        }));
}

/// `WOW_GLYPH_CACHE=1`: one line a second of what the cache holds, the measurement that sizes
/// `super::pack::SHEET_SIZE`.
fn report_cache(atlas: &UiFontAtlas) {
    use std::sync::atomic::{AtomicU64, Ordering};
    static LAST: AtomicU64 = AtomicU64::new(0);
    if std::env::var_os("WOW_GLYPH_CACHE").is_none_or(|v| v == "0") {
        return;
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    if LAST.swap(now, Ordering::Relaxed) == now {
        return;
    }
    let e = atlas.lock();
    let (used, total) = e.sheet.occupancy();
    eprintln!(
        "[glyph-cache] {used}/{total} texels ({:.1}%) · {} chars shaped · \
         {} cells rasterized · {} resets · dpi {}",
        100.0 * used as f64 / total as f64,
        e.stats.chars_shaped,
        e.stats.cells_rasterized,
        e.stats.resets,
        e.dpi,
    );
}
