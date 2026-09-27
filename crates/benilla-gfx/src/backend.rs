//! Which window and device backend the gfx library opens: `WOW_GFX_WINDOW` and `WOW_GFX_DEVICE`,
//! read once at boot. Unset, each is the first one the library was built with, in
//! `wc_clean_new_rs`'s priority order. A name the library lacks fails the boot with the choices it
//! has; there is no silent fallback.

use std::ffi::CStr;

use crate::ffi::{self, GfxDeviceBackend, GfxWindowBackend};

/// Window backends by name, in default-priority order.
pub const WINDOWS: [(GfxWindowBackend, &str); 4] = [
    (GfxWindowBackend::X11, "x11"),
    (GfxWindowBackend::Win32, "win32"),
    (GfxWindowBackend::Glfw, "glfw"),
    (GfxWindowBackend::Sdl, "sdl"),
];

/// Device backends by name, in default-priority order.
pub const DEVICES: [(GfxDeviceBackend, &str); 6] = [
    (GfxDeviceBackend::Gl4, "gl4"),
    (GfxDeviceBackend::Gl3, "gl3"),
    (GfxDeviceBackend::D3d11, "d3d11"),
    (GfxDeviceBackend::Gles3, "gles3"),
    (GfxDeviceBackend::Vk, "vk"),
    (GfxDeviceBackend::D3d12, "d3d12"),
];

/// The pair a run opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Backends {
    pub window: GfxWindowBackend,
    pub device: GfxDeviceBackend,
}

impl Backends {
    /// From the environment, against what the loaded library offers.
    pub fn from_env() -> Result<Self, String> {
        Self::resolve(
            std::env::var("WOW_GFX_WINDOW").ok().as_deref(),
            std::env::var("WOW_GFX_DEVICE").ok().as_deref(),
            // SAFETY: plain queries of the library's compiled-in backend tables.
            |w| unsafe { ffi::gfx_dll_has_window_backend(w) },
            |d| unsafe { ffi::gfx_dll_has_device_backend(d) },
        )
    }

    /// The selection itself, over the two availability queries.
    pub fn resolve(
        window: Option<&str>,
        device: Option<&str>,
        has_window: impl Fn(GfxWindowBackend) -> bool,
        has_device: impl Fn(GfxDeviceBackend) -> bool,
    ) -> Result<Self, String> {
        Ok(Self {
            window: pick("WOW_GFX_WINDOW", window, &WINDOWS, has_window)?,
            device: pick("WOW_GFX_DEVICE", device, &DEVICES, has_device)?,
        })
    }

    pub fn window_name(self) -> &'static str {
        name_in(&WINDOWS, self.window)
    }

    pub fn device_name(self) -> &'static str {
        name_in(&DEVICES, self.device)
    }

    /// The library's own name for the device, for the boot line.
    pub fn device_label(self) -> String {
        // SAFETY: the library returns a static NUL-terminated string or null.
        let p = unsafe { ffi::gfx_dll_device_backend_name(self.device) };
        if p.is_null() {
            return self.device_name().into();
        }
        // SAFETY: non-null, NUL-terminated, static.
        unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned()
    }
}

fn name_in<T: PartialEq + Copy>(table: &[(T, &'static str)], v: T) -> &'static str {
    table.iter().find(|(b, _)| *b == v).map_or("?", |(_, n)| n)
}

fn pick<T: Copy>(
    var: &str,
    requested: Option<&str>,
    table: &[(T, &'static str)],
    has: impl Fn(T) -> bool,
) -> Result<T, String> {
    let available: Vec<_> = table.iter().filter(|(b, _)| has(*b)).collect();
    let choices = || {
        let names: Vec<_> = available.iter().map(|(_, n)| *n).collect();
        if names.is_empty() {
            "none".to_string()
        } else {
            names.join(", ")
        }
    };
    match requested.map(str::trim).filter(|s| !s.is_empty()) {
        None => available.first().map(|(b, _)| *b).ok_or_else(|| {
            format!("{var}: the gfx library was built with no backend of this kind")
        }),
        Some(name) => {
            let name = name.to_ascii_lowercase();
            match table.iter().find(|(_, n)| *n == name) {
                None => Err(format!(
                    "{var}={name}: not a backend name (this library has: {})",
                    choices()
                )),
                Some((b, _)) if has(*b) => Ok(*b),
                Some(_) => Err(format!(
                    "{var}={name}: the gfx library was built without it (it has: {})",
                    choices()
                )),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn linux(w: GfxWindowBackend) -> bool {
        matches!(
            w,
            GfxWindowBackend::X11 | GfxWindowBackend::Glfw | GfxWindowBackend::Sdl
        )
    }

    fn gl_vk(d: GfxDeviceBackend) -> bool {
        matches!(
            d,
            GfxDeviceBackend::Gl3
                | GfxDeviceBackend::Gl4
                | GfxDeviceBackend::Gles3
                | GfxDeviceBackend::Vk
        )
    }

    #[test]
    fn unset_takes_the_first_available_in_priority_order() {
        let b = Backends::resolve(None, None, linux, gl_vk).unwrap();
        assert_eq!(b.window, GfxWindowBackend::X11);
        assert_eq!(b.device, GfxDeviceBackend::Gl4);
        let b = Backends::resolve(Some(""), None, |w| w == GfxWindowBackend::Win32, gl_vk);
        assert_eq!(b.unwrap().window, GfxWindowBackend::Win32);
    }

    #[test]
    fn a_named_backend_is_taken_case_blind() {
        let b = Backends::resolve(Some("SDL"), Some("vk"), linux, gl_vk).unwrap();
        assert_eq!((b.window_name(), b.device_name()), ("sdl", "vk"));
    }

    #[test]
    fn a_missing_or_unknown_backend_fails_naming_the_choices() {
        let e = Backends::resolve(Some("win32"), None, linux, gl_vk).unwrap_err();
        assert!(
            e.contains("built without it") && e.contains("x11, glfw, sdl"),
            "{e}"
        );
        let e = Backends::resolve(None, Some("metal"), linux, gl_vk).unwrap_err();
        assert!(
            e.contains("not a backend name") && e.contains("gl4, gl3, gles3, vk"),
            "{e}"
        );
    }
}
