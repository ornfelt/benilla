//! The gfx frame: after each `app.update()` the runner runs [`GfxRender`] on the main world and
//! presents. What it draws grows milestone by milestone; today it clears the window.

use bevy::ecs::schedule::{ExecutorKind, ScheduleLabel};
use bevy::prelude::*;

use crate::context::GfxContext;
use crate::ffi::{self, GfxClearColor};

/// The schedule that draws one frame through gfx, run by the runner after `Update`'s frame.
#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub struct GfxRender;

/// The ordered stages of [`GfxRender`].
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum GfxRenderSystems {
    /// The default framebuffer is bound, sized and cleared.
    Begin,
    /// Everything that draws, after [`Self::Begin`].
    Draw,
}

pub(crate) fn build(app: &mut App) {
    let mut schedule = Schedule::new(GfxRender);
    // Every system here holds the non-send context; one thread, in order.
    schedule.set_executor_kind(ExecutorKind::SingleThreaded);
    app.add_schedule(schedule)
        .configure_sets(
            GfxRender,
            (GfxRenderSystems::Begin, GfxRenderSystems::Draw).chain(),
        )
        .add_systems(GfxRender, begin_frame.in_set(GfxRenderSystems::Begin));
}

/// Binds the window's framebuffer over its full size and clears colour to the app's
/// `ClearColor`, as the wgpu path's first pass loads it, and depth to the far plane.
fn begin_frame(ctx: NonSend<GfxContext>, clear: Option<Res<ClearColor>>) {
    let size = ctx.size();
    let c = clear.map_or(Color::BLACK, |c| c.0).to_srgba();
    // The window's colour buffer is not sRGB-encoding: the clear is written as displayed.
    let color = GfxClearColor {
        r: c.red,
        g: c.green,
        b: c.blue,
        a: c.alpha,
    };
    let device = ctx.device;
    // SAFETY: the live device, on the thread that owns it; a null framebuffer is the window's.
    unsafe {
        ffi::gfx_dll_bind_framebuffer(device, std::ptr::null_mut());
        ffi::gfx_dll_set_viewport(device, 0, 0, size.x, size.y, 0.0, 1.0);
        ffi::gfx_dll_set_scissor(device, 0, 0, size.x, size.y);
        ffi::gfx_dll_clear_color(device, std::ptr::null_mut(), 0, &color);
        ffi::gfx_dll_clear_depth_stencil(device, std::ptr::null_mut(), 1.0, 0);
    }
}
