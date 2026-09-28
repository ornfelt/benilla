//! The one gfx window and its device, a non-send resource: every gfx call is made on the main
//! thread, where the window was created.

use std::ffi::{c_char, CStr, CString};
use std::ptr;

use bevy::prelude::*;

use crate::backend::Backends;
use crate::events;
use crate::ffi::{self, GfxDevice, GfxFormat, GfxWindow};
use crate::shader_loader::{program_names, ShaderLibrary};
use crate::window::{CursorState, Reported};

/// What the window opens with, read off the primary `Window`.
pub struct WindowSpec {
    pub title: String,
    /// The size in logical pixels (bevy's `Window::width` / `height`).
    pub width: f32,
    pub height: f32,
    /// `WindowResolution::scale_factor_override`: the size's scale in place of the display's.
    pub scale_factor_override: Option<f32>,
    pub vsync: bool,
    pub mode: bevy::window::WindowMode,
    pub level: bevy::window::WindowLevel,
    pub position: bevy::window::WindowPosition,
    pub decorations: bool,
    pub resizable: bool,
}

pub struct GfxContext {
    pub window: GfxWindow,
    pub device: GfxDevice,
    pub backends: Backends,
    pub shaders: ShaderLibrary,
    /// The pointer's grab, visibility and cursor, kept by [`crate::window`].
    pub cursor: CursorState,
    /// The size and focus the window last reported.
    pub reported: Reported,
}

impl GfxContext {
    /// Opens the window on `backends` and loads every compiled shader program, so a backend
    /// that cannot run them fails here, at boot, and not at its first draw.
    pub fn open(spec: &WindowSpec, backends: Backends) -> Result<Self, String> {
        // SAFETY: installs a static callback; the library calls it on the thread it errors on.
        unsafe { ffi::gfx_dll_set_error_callback(on_error) };

        let mut props = ffi::GfxWindowProperties {
            window_backend: backends.window,
            device_backend: backends.device,
            color_format: GfxFormat::Unknown,
            depth_stencil_format: GfxFormat::Unknown,
            samples: 0,
        };
        // SAFETY: fills the struct the library owns the defaults of.
        unsafe { ffi::gfx_dll_window_properties_init(&mut props) };
        props.window_backend = backends.window;
        props.device_backend = backends.device;
        props.color_format = GfxFormat::R8G8B8A8Unorm;
        props.depth_stencil_format = GfxFormat::D24UnormS8Uint;

        let title = CString::new(spec.title.replace('\0', "")).unwrap_or_default();
        let asked = physical_size(
            spec.width,
            spec.height,
            spec.scale_factor_override.unwrap_or(1.0),
        );
        // SAFETY: `title` and `props` are live for the call.
        let window =
            unsafe { ffi::gfx_dll_create_window(title.as_ptr(), asked.x, asked.y, &props) };
        if window.is_null() {
            return Err(format!(
                "gfx_dll_create_window failed ({} / {})",
                backends.window_name(),
                backends.device_name()
            ));
        }
        // SAFETY: `window` is the live window just created, on this thread.
        let device = unsafe {
            ffi::gfx_dll_window_set_event_handler(window, events::on_event);
            // bevy_winit asks winit for the logical size, which winit scales by the display's
            // factor (or bevy's override) before it creates the window; the display's factor is
            // known once the window is.
            if spec.scale_factor_override.is_none() {
                let scale = ffi::gfx_dll_window_get_scale_factor(window);
                if scale.is_finite() && scale > 0.0 {
                    let size = physical_size(spec.width, spec.height, scale);
                    if size != asked {
                        ffi::gfx_dll_window_resize(window, size.x, size.y);
                    }
                }
            }
            // Before the map, as winit builds the window in its mode, level, frame, sizing and
            // place.
            ffi::gfx_dll_window_set_mode(window, crate::window::gfx_mode(spec.mode));
            ffi::gfx_dll_window_set_level(window, crate::window::gfx_level(spec.level));
            if !spec.decorations {
                ffi::gfx_dll_window_set_decorations(window, false);
            }
            if !spec.resizable {
                ffi::gfx_dll_window_set_resizable(window, false);
            }
            crate::window::apply_position(window, spec.position);
            ffi::gfx_dll_window_show(window);
            ffi::gfx_dll_set_swap_interval(window, spec.vsync as i32);
            ffi::gfx_dll_get_device(window)
        };
        let mut ctx = Self {
            window,
            device,
            backends,
            shaders: ShaderLibrary::new(device, backends.device, backends.window),
            cursor: CursorState::default(),
            reported: Reported::default(),
        };
        let family = ctx.shaders.family();
        let names = program_names(family);
        for name in &names {
            ctx.shaders.get(name)?;
        }
        let size = ctx.size();
        info!(
            "gfx: window {} / device {} ({}), {}x{} (asked {}x{} logical), shaders {}: {} program(s) loaded",
            backends.window_name(),
            backends.device_name(),
            backends.device_label(),
            size.x,
            size.y,
            spec.width,
            spec.height,
            family.dir_name(),
            names.len()
        );
        Ok(ctx)
    }

    /// The drawable size in pixels.
    pub fn size(&self) -> UVec2 {
        // SAFETY: `self.window` is live for the life of `self`.
        unsafe {
            UVec2::new(
                ffi::gfx_dll_window_get_width(self.window),
                ffi::gfx_dll_window_get_height(self.window),
            )
        }
    }

    /// Physical pixels per logical pixel on the display the window opened on, as winit measures
    /// it for the platform (`gfx_benilla` asks the display the way winit does).
    pub fn scale_factor(&self) -> f32 {
        // SAFETY: `self.window` is live for the life of `self`.
        let scale = unsafe { ffi::gfx_dll_window_get_scale_factor(self.window) };
        if scale.is_finite() && scale > 0.0 {
            scale
        } else {
            1.0
        }
    }

    pub fn poll_events(&self) {
        // SAFETY: the live window, on its thread; the handler only queues.
        unsafe { ffi::gfx_dll_window_poll_events(self.window) };
    }

    /// Takes a pending close (the window manager's close button), clearing it so Bevy decides.
    pub fn take_close_request(&self) -> bool {
        // SAFETY: the live window.
        unsafe {
            let requested = ffi::gfx_dll_window_is_close_requested(self.window);
            if requested {
                ffi::gfx_dll_window_set_close_requested(self.window, false);
            }
            requested
        }
    }

    pub fn present(&self) {
        // SAFETY: the live window, on its thread.
        unsafe { ffi::gfx_dll_swap_buffers(self.window) };
    }
}

impl Drop for GfxContext {
    fn drop(&mut self) {
        // Cursors and programs first: they are the window's and its device's.
        self.cursor.clear(self.window);
        self.shaders.clear();
        // SAFETY: nothing references the window past this point.
        unsafe { ffi::gfx_dll_delete_window(self.window) };
        self.window = ptr::null_mut();
    }
}

unsafe extern "C" fn on_error(msg: *const c_char) {
    if msg.is_null() {
        return;
    }
    // SAFETY: the library passes a NUL-terminated message, live for the call.
    let msg = unsafe { CStr::from_ptr(msg) }.to_string_lossy();
    error!("gfx: {msg}");
}

/// A logical size in physical pixels at `scale`, rounded as winit's `LogicalSize::to_physical`.
fn physical_size(width: f32, height: f32, scale: f32) -> UVec2 {
    let px = |v: f32| ((f64::from(v) * f64::from(scale)).round() as u32).max(1);
    UVec2::new(px(width), px(height))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_logical_size_scales_and_rounds_as_winit() {
        // benilla's 640x700 on a 1.667 display (Xft.dpi 160): winit's 1066.7 x 1166.7.
        assert_eq!(
            physical_size(640.0, 700.0, 160.0 / 96.0),
            UVec2::new(1067, 1167)
        );
        assert_eq!(physical_size(640.0, 700.0, 1.0), UVec2::new(640, 700));
        assert_eq!(physical_size(0.0, 0.0, 2.0), UVec2::ONE);
    }
}
