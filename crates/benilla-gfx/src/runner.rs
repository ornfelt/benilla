//! The app runner in place of winit's: opens the gfx window for the primary `Window`, then per
//! frame polls the window, runs the app, draws through [`GfxRender`] and presents, until the app
//! exits. The window is deleted before the runner returns.

use bevy::app::PluginsState;
use bevy::prelude::*;
use bevy::window::{PresentMode, PrimaryWindow, WindowCloseRequested};

use crate::backend::Backends;
use crate::context::{GfxContext, WindowSpec};
use crate::events;
use crate::render::GfxRender;

pub fn run(mut app: App) -> AppExit {
    if app.plugins_state() != PluginsState::Cleaned {
        while app.plugins_state() == PluginsState::Adding {
            bevy::tasks::tick_global_task_pools_on_main_thread();
        }
        app.finish();
        app.cleanup();
    }

    crate::noop_device::insert(app.world_mut());
    let ctx = match open(app.world_mut()) {
        Ok(ctx) => ctx,
        Err(e) => {
            error!("gfx: {e}");
            eprintln!("gfx: {e}");
            return AppExit::error();
        }
    };
    app.world_mut().insert_non_send_resource(ctx);

    let mut queued = Vec::new();
    let started = std::time::Instant::now();
    let mut presented = 0u64;
    let exit = loop {
        pump(app.world_mut(), &mut queued);
        app.update();
        if let Some(exit) = app.should_exit() {
            break exit;
        }
        app.world_mut().run_schedule(GfxRender);
        app.world().non_send_resource::<GfxContext>().present();
        presented += 1;
    };
    let secs = started.elapsed().as_secs_f64();
    info!(
        "gfx: {presented} frames presented in {secs:.2} s ({:.1}/s); exit {exit:?}",
        presented as f64 / secs.max(f64::EPSILON)
    );
    // The window and every device object go before the app does.
    drop(app.world_mut().remove_non_send_resource::<GfxContext>());
    exit
}

fn open(world: &mut World) -> Result<GfxContext, String> {
    let backends = Backends::from_env()?;
    let mut windows = world.query_filtered::<&Window, With<PrimaryWindow>>();
    let window = windows
        .single(world)
        .map_err(|_| "the app has no primary window to open".to_string())?;
    let spec = WindowSpec {
        title: window.title.clone(),
        width: window.resolution.physical_width(),
        height: window.resolution.physical_height(),
        vsync: !matches!(
            window.present_mode,
            PresentMode::AutoNoVsync | PresentMode::Immediate | PresentMode::Mailbox
        ),
    };
    GfxContext::open(&spec, backends)
}

/// Polls the window and hands what it saw to the app. A close from the window manager becomes
/// `WindowCloseRequested`, so the app's own close path (and its shutdown saves) runs.
fn pump(world: &mut World, queued: &mut Vec<crate::ffi::GfxEvent>) {
    let close = {
        let ctx = world.non_send_resource::<GfxContext>();
        ctx.poll_events();
        ctx.take_close_request()
    };
    queued.clear();
    events::drain_into(queued);
    if close {
        let mut primary = world.query_filtered::<Entity, With<PrimaryWindow>>();
        if let Ok(window) = primary.single(world) {
            world.write_message(WindowCloseRequested { window });
        }
    }
}
