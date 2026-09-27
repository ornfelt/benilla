//! The `.gfx` binary shader format `wc_compiler_rs` writes, ported from
//! `wc_clean_new_rs/src/gfx_shader_def.rs`: a packed header, then the inputs, outputs, structs,
//! samplers and constant blocks, then one code blob per device backend.

use std::mem::size_of;

use crate::ffi::GfxShaderType;

pub const NAME_SIZE: usize = 64;
pub const MAX_MEMBERS: usize = 64;
/// One code slot per `enum gfx_device_backend`, indexed by its value.
pub const MAX_CODE_SLOTS: usize = 8;

const fn magic(tag: &[u8; 4]) -> u32 {
    u32::from_be_bytes(*tag)
}

/// The header magic of a stage: `GFXV`, `GFXF`, `GFXG`.
pub fn stage_magic(stage: GfxShaderType) -> u32 {
    match stage {
        GfxShaderType::Vertex => magic(b"GFXV"),
        GfxShaderType::Fragment => magic(b"GFXF"),
        GfxShaderType::Geometry => magic(b"GFXG"),
    }
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct ShaderDefHeader {
    pub magic: u32,
    pub inputs_count: u8,
    pub outputs_count: u8,
    pub structs_count: u8,
    pub samplers_count: u8,
    pub constants_count: u8,
    pub _padding: [u8; 3],
    pub codes_lengths: [u32; MAX_CODE_SLOTS],
    pub codes_offsets: [u32; MAX_CODE_SLOTS],
}

/// A stage input or output: a vertex attribute or a varying.
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct ShaderIoDef {
    pub name: [u8; NAME_SIZE],
    pub ty: u8,
    pub bind: u8,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct ShaderStructMemberDef {
    pub name: [u8; NAME_SIZE],
    pub array_size: u16,
    pub ty: u8,
    pub _pad: u8,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct ShaderStructDef {
    pub name: [u8; NAME_SIZE],
    pub members: [ShaderStructMemberDef; MAX_MEMBERS],
    pub members_count: u32,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct ShaderSamplerDef {
    pub name: [u8; NAME_SIZE],
    pub ty: u8,
    pub bind: u8,
    pub array_size: u16,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct ShaderConstantMemberDef {
    pub name: [u8; NAME_SIZE],
    pub array_size: u16,
    pub struct_id: u8,
    pub ty: u8,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct ShaderConstantDef {
    pub name: [u8; NAME_SIZE],
    pub members: [ShaderConstantMemberDef; MAX_MEMBERS],
    pub members_count: u32,
    pub bind: u8,
    pub _pad: [u8; 3],
}

/// A NUL-terminated name field as a string.
pub fn name_of(bytes: &[u8; NAME_SIZE]) -> String {
    let len = bytes.iter().position(|&b| b == 0).unwrap_or(NAME_SIZE);
    String::from_utf8_lossy(&bytes[..len]).into_owned()
}

/// Marks the plain-old-data records [`read`] may copy out of a byte buffer.
///
/// # Safety
/// Every bit pattern of `size_of::<Self>()` bytes must be a valid `Self`.
pub unsafe trait Pod: Copy {}
unsafe impl Pod for ShaderDefHeader {}
unsafe impl Pod for ShaderIoDef {}
unsafe impl Pod for ShaderStructDef {}
unsafe impl Pod for ShaderSamplerDef {}
unsafe impl Pod for ShaderConstantDef {}

/// Copies one `T` out of `data` at `offset`, or `None` past the end.
pub fn read<T: Pod>(data: &[u8], offset: usize) -> Option<T> {
    let bytes = data.get(offset..offset.checked_add(size_of::<T>())?)?;
    // SAFETY: `bytes` holds exactly `size_of::<T>()` bytes, `read_unaligned` needs no alignment,
    // and `T: Pod` makes any bit pattern a valid `T`.
    Some(unsafe { bytes.as_ptr().cast::<T>().read_unaligned() })
}

/// One parsed `.gfx` stage.
pub struct ParsedShader {
    pub stage: GfxShaderType,
    pub header: ShaderDefHeader,
    pub inputs: Vec<ShaderIoDef>,
    pub outputs: Vec<ShaderIoDef>,
    pub samplers: Vec<ShaderSamplerDef>,
    pub constants: Vec<ShaderConstantDef>,
    pub data: Vec<u8>,
}

impl ParsedShader {
    pub fn parse(data: &[u8], stage: GfxShaderType) -> Result<Self, String> {
        let header: ShaderDefHeader = read(data, 0).ok_or("shorter than the header")?;
        let found = header.magic;
        if found != stage_magic(stage) {
            return Err(format!(
                "magic {found:#010x}, expected {:#010x}",
                stage_magic(stage)
            ));
        }
        let mut at = size_of::<ShaderDefHeader>();
        fn table<T: Pod>(data: &[u8], at: &mut usize, n: u8) -> Result<Vec<T>, String> {
            (0..n as usize)
                .map(|_| {
                    let v = read(data, *at).ok_or_else(|| format!("truncated at {at}"));
                    *at += size_of::<T>();
                    v
                })
                .collect()
        }
        let inputs = table(data, &mut at, header.inputs_count)?;
        let outputs = table(data, &mut at, header.outputs_count)?;
        let _structs: Vec<ShaderStructDef> = table(data, &mut at, header.structs_count)?;
        let samplers = table(data, &mut at, header.samplers_count)?;
        let constants = table(data, &mut at, header.constants_count)?;
        Ok(Self {
            stage,
            header,
            inputs,
            outputs,
            samplers,
            constants,
            data: data.to_vec(),
        })
    }

    /// The code blob for the device backend at `slot`, or `None` when the file carries none.
    pub fn code(&self, slot: usize) -> Option<&[u8]> {
        let lengths = self.header.codes_lengths;
        let offsets = self.header.codes_offsets;
        let (len, off) = (*lengths.get(slot)? as usize, *offsets.get(slot)? as usize);
        if len == 0 {
            return None;
        }
        self.data.get(off..off.checked_add(len)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_sizes_match_the_c_layout() {
        // The sizes `gfx_dll_print_struct_sizes` reports for the packed C structs.
        assert_eq!(size_of::<ShaderDefHeader>(), 76);
        assert_eq!(size_of::<ShaderIoDef>(), 66);
        assert_eq!(size_of::<ShaderStructDef>(), 4420);
        assert_eq!(size_of::<ShaderSamplerDef>(), 68);
        assert_eq!(size_of::<ShaderConstantDef>(), 4424);
    }

    #[test]
    fn a_short_or_foreign_file_is_an_error() {
        assert!(ParsedShader::parse(&[0; 8], GfxShaderType::Vertex).is_err());
        let mut header = vec![0u8; size_of::<ShaderDefHeader>()];
        // The compiler writes the magic as a native `u32`, so `GFXF` lands on disk as `FXFG`.
        header[..4].copy_from_slice(&stage_magic(GfxShaderType::Fragment).to_ne_bytes());
        assert!(ParsedShader::parse(&header, GfxShaderType::Vertex).is_err());
        assert!(ParsedShader::parse(&header, GfxShaderType::Fragment).is_ok());
    }
}
