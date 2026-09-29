//! The app runner in place of winit's: opens the gfx window for the primary `Window`, then per
//! frame polls the window, runs the app, draws through [`GfxRender`] and presents, until the app
//! exits. The window is deleted before the runner returns.

use bevy::app::PluginsState;
use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowCloseRequested};

use crate::backend::Backends;
use crate::context::{GfxContext, WindowSpec};
use crate::draw::GfxRenderer;
use crate::events;
use crate::render::GfxRender;

/// The `ImagePlugin` default sampler, handed from `GfxPlugin::finish` to the renderer.
#[derive(Resource)]
pub(crate) struct DefaultSampler(pub bevy::image::ImageSamplerDescriptor);

/// The sample counts the gfx device offers a multisampled target
/// ([`crate::target::supported_sample_counts`]), ascending. Inserted once the device opens, before
/// the first update (`PreStartup`).
#[derive(Resource, Debug, Clone)]
pub struct GfxMsaaCounts(pub Vec<u32>);

pub fn run(mut app: App) -> AppExit {
    // The device opens before the plugins finish, as wgpu's does in `RenderPlugin::build`:
    // `finish` reads what it can do (the BLP loader's BC lane).
    let finished = app.plugins_state() == PluginsState::Cleaned;
    if !finished {
        while app.plugins_state() == PluginsState::Adding {
            bevy::tasks::tick_global_task_pools_on_main_thread();
        }
    }
    let ctx = match open(app.world_mut()) {
        Ok(ctx) => ctx,
        Err(e) => {
            error!("gfx: {e}");
            eprintln!("gfx: {e}");
            return AppExit::error();
        }
    };
    let bc = crate::images::bc_supported(ctx.device);
    info!(
        "gfx: BC texture formats {}",
        if bc { "sampled" } else { "not supported" }
    );
    if finished {
        warn!("gfx: the plugins finished before the device opened; BC support stays guessed");
    } else {
        app.world_mut()
            .insert_resource(bevy::image::CompressedImageFormatSupport(if bc {
                bevy::image::CompressedImageFormats::BC
            } else {
                bevy::image::CompressedImageFormats::NONE
            }));
        app.finish();
        app.cleanup();
    }

    let counts = crate::target::supported_sample_counts(ctx.device);
    app.world_mut().insert_resource(GfxMsaaCounts(counts));
    app.world_mut().insert_non_send_resource(ctx);
    {
        let world = app.world_mut();
        let sampler = world
            .remove_resource::<DefaultSampler>()
            .map(|s| s.0)
            .unwrap_or_default();
        let mut ctx = world.non_send_resource_mut::<GfxContext>();
        let renderer = GfxRenderer::new(&mut ctx, sampler);
        world.insert_non_send_resource(renderer);
    }
    {
        let world = app.world_mut();
        let mut primary = world.query_filtered::<Entity, With<PrimaryWindow>>();
        if let Ok(entity) = primary.single(world) {
            crate::window::opened(world, entity);
        }
    }

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
    // The window and every device object go before the app does; the renderer's objects first,
    // while their device lives.
    drop(app.world_mut().remove_non_send_resource::<GfxRenderer>());
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
        width: window.width(),
        height: window.height(),
        scale_factor_override: window.resolution.scale_factor_override(),
        vsync: crate::window::swap_interval(window.present_mode) != 0,
        mode: window.mode,
        level: window.window_level,
        position: window.position,
        decorations: window.decorations,
        resizable: window.resizable,
    };
    GfxContext::open(&spec, backends)
}

/// Polls the window and hands what it saw to the app ([`crate::window::pump`]). A close from the
/// window manager becomes `WindowCloseRequested`, so the app's own close path (and its shutdown
/// saves) runs.
fn pump(world: &mut World, queued: &mut Vec<crate::ffi::GfxEvent>) {
    let close = {
        let ctx = world.non_send_resource::<GfxContext>();
        ctx.poll_events();
        ctx.take_close_request()
    };
    queued.clear();
    events::drain_into(queued);
    world.resource_mut::<crate::window::InputTrace>().frame += 1;
    crate::window::pump(world, queued);
    if close {
        let mut primary = world.query_filtered::<Entity, With<PrimaryWindow>>();
        if let Ok(window) = primary.single(world) {
            world.write_message(WindowCloseRequested { window });
        }
    }
}
