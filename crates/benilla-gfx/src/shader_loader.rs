//! Loads the compiled `.gfx` shaders and creates their shader states, ported from
//! `wc_clean_new_rs/src/gfx_shader_loader.rs`. The four families `shaders/compile.sh` writes are
//! embedded; `WOW_GFX_SHADERS=<dir>` reads `<dir>/<family>/` from disk instead, to iterate on a
//! shader without a rebuild.

use std::collections::HashMap;
use std::ffi::CString;
use std::path::PathBuf;
use std::ptr;

use include_dir::{include_dir, Dir};

use crate::ffi::{self, GfxDevice, GfxDeviceBackend, GfxShader, GfxShaderState, GfxShaderType};
use crate::ffi::{GfxShaderBinding, GfxWindowBackend};
use crate::shader_def::ParsedShader;

static GL: Dir = include_dir!("$CARGO_MANIFEST_DIR/shaders/shaders");
static VK: Dir = include_dir!("$CARGO_MANIFEST_DIR/shaders/shaders_vk");
static D3: Dir = include_dir!("$CARGO_MANIFEST_DIR/shaders/shaders_d3");
static GLES3_DARK: Dir = include_dir!("$CARGO_MANIFEST_DIR/shaders/shaders_gles3_dark");

/// The shader family a device reads, as `gfx_shader_loader.rs` picks it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaderFamily {
    Gl,
    Vk,
    D3,
    Gles3Dark,
}

impl ShaderFamily {
    pub const ALL: [ShaderFamily; 4] = [Self::Gl, Self::Vk, Self::D3, Self::Gles3Dark];

    pub fn for_backends(device: GfxDeviceBackend, window: GfxWindowBackend) -> Self {
        match device {
            GfxDeviceBackend::Vk => Self::Vk,
            // A native (non-GLFW) gles3 window off Linux reads its own variants.
            GfxDeviceBackend::Gles3
                if cfg!(not(target_os = "linux")) && window != GfxWindowBackend::Glfw =>
            {
                Self::Gles3Dark
            }
            GfxDeviceBackend::D3d9 | GfxDeviceBackend::D3d11 | GfxDeviceBackend::D3d12 => Self::D3,
            _ => Self::Gl,
        }
    }

    pub fn dir_name(self) -> &'static str {
        match self {
            Self::Gl => "shaders",
            Self::Vk => "shaders_vk",
            Self::D3 => "shaders_d3",
            Self::Gles3Dark => "shaders_gles3_dark",
        }
    }

    fn embedded(self) -> &'static Dir<'static> {
        match self {
            Self::Gl => &GL,
            Self::Vk => &VK,
            Self::D3 => &D3,
            Self::Gles3Dark => &GLES3_DARK,
        }
    }

    /// The bytes of `<name>.<vs|fs>.gfx` in this family.
    pub fn read(self, name: &str, stage: GfxShaderType) -> Result<Vec<u8>, String> {
        let file = format!("{name}.{}.gfx", stage_ext(stage));
        if let Some(root) = std::env::var_os("WOW_GFX_SHADERS") {
            let path = PathBuf::from(root).join(self.dir_name()).join(&file);
            return std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()));
        }
        self.embedded()
            .get_file(&file)
            .map(|f| f.contents().to_vec())
            .ok_or_else(|| format!("{}/{file}: not compiled", self.dir_name()))
    }
}

fn stage_ext(stage: GfxShaderType) -> &'static str {
    match stage {
        GfxShaderType::Vertex => "vs",
        GfxShaderType::Fragment => "fs",
        GfxShaderType::Geometry => "gs",
    }
}

/// A loaded vertex + fragment pair and the names its resources bind by.
pub struct ShaderProgram {
    pub state: GfxShaderState,
    shaders: [GfxShader; 2],
    /// Constant blocks, `(name, bind)`, vertex stage first.
    pub constants: Vec<(String, u32)>,
    /// Samplers, `(name, bind)`, one per array element.
    pub samplers: Vec<(String, u32)>,
}

/// Every shader program the device has loaded, by base name; deletes them on drop.
pub struct ShaderLibrary {
    device: GfxDevice,
    backend: GfxDeviceBackend,
    family: ShaderFamily,
    programs: HashMap<String, ShaderProgram>,
}

impl ShaderLibrary {
    pub fn new(device: GfxDevice, backend: GfxDeviceBackend, window: GfxWindowBackend) -> Self {
        Self {
            device,
            backend,
            family: ShaderFamily::for_backends(backend, window),
            programs: HashMap::new(),
        }
    }

    pub fn family(&self) -> ShaderFamily {
        self.family
    }

    /// The program `name`, loading it on first use.
    pub fn get(&mut self, name: &str) -> Result<&ShaderProgram, String> {
        if !self.programs.contains_key(name) {
            let program = self.load(name).map_err(|e| format!("shader {name}: {e}"))?;
            self.programs.insert(name.to_owned(), program);
        }
        Ok(&self.programs[name])
    }

    fn load(&self, name: &str) -> Result<ShaderProgram, String> {
        let vs = ParsedShader::parse(
            &self.family.read(name, GfxShaderType::Vertex)?,
            GfxShaderType::Vertex,
        )?;
        let fs = ParsedShader::parse(
            &self.family.read(name, GfxShaderType::Fragment)?,
            GfxShaderType::Fragment,
        )?;
        check_io(&vs, &fs)?;

        let vs_shader = self.create_shader(&vs)?;
        let fs_shader = match self.create_shader(&fs) {
            Ok(s) => s,
            Err(e) => {
                // SAFETY: `vs_shader` was created on this device and is referenced nowhere else.
                unsafe { ffi::gfx_dll_delete_shader(self.device, vs_shader) };
                return Err(e);
            }
        };

        let attributes: Vec<(String, u32)> = vs
            .inputs
            .iter()
            .map(|i| (crate::shader_def::name_of(&i.name), i.bind as u32))
            .collect();
        let mut samplers = Vec::new();
        for s in vs.samplers.iter().chain(&fs.samplers) {
            let base = crate::shader_def::name_of(&s.name);
            let (bind, count) = (s.bind as u32, s.array_size);
            if count == 0 {
                samplers.push((base, bind));
            } else {
                samplers.extend((0..count as u32).map(|j| (format!("{base}[{j}]"), bind + j)));
            }
        }
        let constants: Vec<(String, u32)> = vs
            .constants
            .iter()
            .chain(&fs.constants)
            .map(|c| (crate::shader_def::name_of(&c.name), c.bind as u32))
            .collect();

        // NUL-terminated binding lists; the `CString`s outlive the call, which copies them.
        let owned: Vec<Vec<(CString, u32)>> = [&attributes, &constants, &samplers]
            .iter()
            .map(|list| {
                list.iter()
                    .map(|(n, b)| (CString::new(n.as_str()).unwrap_or_default(), *b))
                    .collect()
            })
            .collect();
        let lists: Vec<Vec<GfxShaderBinding>> = owned
            .iter()
            .map(|list| {
                list.iter()
                    .map(|(n, b)| GfxShaderBinding {
                        name: n.as_ptr(),
                        bind: *b,
                    })
                    .chain(std::iter::once(GfxShaderBinding {
                        name: ptr::null(),
                        bind: 0,
                    }))
                    .collect()
            })
            .collect();
        let info = ffi::GfxShaderStateCreateInfo {
            vertex_shader: vs_shader,
            fragment_shader: fs_shader,
            geometry_shader: ptr::null_mut(),
            attributes: lists[0].as_ptr(),
            constants: lists[1].as_ptr(),
            samplers: lists[2].as_ptr(),
        };
        let mut state: GfxShaderState = ptr::null_mut();
        // SAFETY: `info` and the lists it points into are live for the call.
        let ok = unsafe { ffi::gfx_dll_create_shader_state(self.device, &info, &mut state) };
        if !ok {
            // SAFETY: both shaders were created on this device and are referenced nowhere else.
            unsafe {
                ffi::gfx_dll_delete_shader(self.device, vs_shader);
                ffi::gfx_dll_delete_shader(self.device, fs_shader);
            }
            return Err("gfx_dll_create_shader_state failed".into());
        }
        Ok(ShaderProgram {
            state,
            shaders: [vs_shader, fs_shader],
            constants,
            samplers,
        })
    }

    fn create_shader(&self, parsed: &ParsedShader) -> Result<GfxShader, String> {
        let slot = self.backend as usize;
        // d3d12 compiles the d3d11 HLSL when the file carries no blob of its own.
        let code = parsed
            .code(slot)
            .or_else(|| {
                (self.backend == GfxDeviceBackend::D3d12)
                    .then(|| parsed.code(GfxDeviceBackend::D3d11 as usize))
                    .flatten()
            })
            .ok_or_else(|| format!("no code for {:?} in {:?}", self.backend, self.family))?;
        let info = ffi::GfxShaderCreateInfo {
            shader_type: parsed.stage,
            data: code.as_ptr().cast(),
            size: code.len() as u32,
        };
        let mut shader: GfxShader = ptr::null_mut();
        // SAFETY: `info.data` points at `code`, live for the call.
        if unsafe { ffi::gfx_dll_create_shader(self.device, &info, &mut shader) } {
            Ok(shader)
        } else {
            Err(format!("gfx_dll_create_shader failed ({:?})", parsed.stage))
        }
    }

    /// Deletes every program; the device must still be alive.
    pub fn clear(&mut self) {
        for (_, p) in self.programs.drain() {
            // SAFETY: each handle was created on `self.device` and is dropped from the map here.
            unsafe {
                ffi::gfx_dll_delete_shader_state(self.device, p.state);
                for s in p.shaders {
                    ffi::gfx_dll_delete_shader(self.device, s);
                }
            }
        }
    }
}

impl Drop for ShaderLibrary {
    fn drop(&mut self) {
        self.clear();
    }
}

/// The vertex outputs must be the fragment inputs, in order, by name, type and bind.
fn check_io(vs: &ParsedShader, fs: &ParsedShader) -> Result<(), String> {
    if vs.outputs.len() != fs.inputs.len() {
        return Err(format!(
            "{} vertex outputs, {} fragment inputs",
            vs.outputs.len(),
            fs.inputs.len()
        ));
    }
    for (o, i) in vs.outputs.iter().zip(&fs.inputs) {
        if (o.ty, o.bind, o.name) != (i.ty, i.bind, i.name) {
            return Err(format!(
                "vertex output {} does not match fragment input {}",
                crate::shader_def::name_of(&o.name),
                crate::shader_def::name_of(&i.name)
            ));
        }
    }
    Ok(())
}

/// The base names of every compiled program in `family`.
pub fn program_names(family: ShaderFamily) -> Vec<String> {
    let mut names: Vec<String> = family
        .embedded()
        .files()
        .filter_map(|f| f.path().file_name()?.to_str()?.strip_suffix(".vs.gfx"))
        .map(str::to_owned)
        .collect();
    names.sort();
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every `.gfxs` source has both stages compiled in every family, and each parses and
    /// carries code for the backends its family serves: `compile.sh` was run after the last edit.
    #[test]
    fn every_source_is_compiled_in_every_family() {
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("shaders/src");
        let mut sources: Vec<String> = std::fs::read_dir(&src)
            .unwrap()
            .filter_map(|e| {
                let name = e.ok()?.file_name().into_string().ok()?;
                name.strip_suffix(".vs.gfxs").map(str::to_owned)
            })
            .collect();
        sources.sort();
        assert!(!sources.is_empty());
        for family in ShaderFamily::ALL {
            assert_eq!(program_names(family), sources, "{family:?}");
            let slots: &[GfxDeviceBackend] = match family {
                ShaderFamily::Gl => &[
                    GfxDeviceBackend::Gl3,
                    GfxDeviceBackend::Gl4,
                    GfxDeviceBackend::Gles3,
                ],
                ShaderFamily::Vk => &[GfxDeviceBackend::Vk],
                ShaderFamily::D3 => &[GfxDeviceBackend::D3d11],
                ShaderFamily::Gles3Dark => &[GfxDeviceBackend::Gles3],
            };
            for name in &sources {
                let vs = ParsedShader::parse(
                    &family.read(name, GfxShaderType::Vertex).unwrap(),
                    GfxShaderType::Vertex,
                )
                .unwrap();
                let fs = ParsedShader::parse(
                    &family.read(name, GfxShaderType::Fragment).unwrap(),
                    GfxShaderType::Fragment,
                )
                .unwrap();
                check_io(&vs, &fs).unwrap();
                for b in slots {
                    assert!(vs.code(*b as usize).is_some(), "{family:?} {name}.vs {b:?}");
                    assert!(fs.code(*b as usize).is_some(), "{family:?} {name}.fs {b:?}");
                }
            }
        }
    }

    #[test]
    fn the_family_follows_the_device() {
        use GfxDeviceBackend as D;
        use GfxWindowBackend as W;
        assert_eq!(ShaderFamily::for_backends(D::Gl4, W::X11), ShaderFamily::Gl);
        assert_eq!(ShaderFamily::for_backends(D::Vk, W::Sdl), ShaderFamily::Vk);
        assert_eq!(
            ShaderFamily::for_backends(D::D3d12, W::Win32),
            ShaderFamily::D3
        );
        assert_eq!(
            ShaderFamily::for_backends(D::Gles3, W::Glfw),
            ShaderFamily::Gl
        );
    }
}
