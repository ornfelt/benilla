//! The one gfx window and its device, a non-send resource: every gfx call is made on the main
//! thread, where the window was created.

use std::ffi::{c_char, CStr, CString};
use std::ptr;

use bevy::prelude::*;

use crate::backend::Backends;
use crate::events;
use crate::ffi::{self, GfxDevice, GfxFormat, GfxWindow};
use crate::shader_loader::{program_names, ShaderLibrary};

/// What the window opens with, read off the primary `Window`.
pub struct WindowSpec {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub vsync: bool,
}

pub struct GfxContext {
    pub window: GfxWindow,
    pub device: GfxDevice,
    pub backends: Backends,
    pub shaders: ShaderLibrary,
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
        // SAFETY: `title` and `props` are live for the call.
        let window = unsafe {
            ffi::gfx_dll_create_window(
                title.as_ptr(),
                spec.width.max(1),
                spec.height.max(1),
                &props,
            )
        };
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
            ffi::gfx_dll_window_show(window);
            ffi::gfx_dll_set_swap_interval(window, spec.vsync as i32);
            ffi::gfx_dll_get_device(window)
        };
        let mut ctx = Self {
            window,
            device,
            backends,
            shaders: ShaderLibrary::new(device, backends.device, backends.window),
        };
        let family = ctx.shaders.family();
        let names = program_names(family);
        for name in &names {
            ctx.shaders.get(name)?;
        }
        let size = ctx.size();
        info!(
            "gfx: window {} / device {} ({}), {}x{} (asked {}x{}), shaders {}: {} program(s) loaded",
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
        // Programs first: they are the device's, which the window owns.
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
