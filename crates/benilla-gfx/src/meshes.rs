//! `Assets<Mesh>` on the device: one gfx vertex buffer per attribute stream and one index buffer,
//! made on first draw and dropped when the asset changes or goes. A mesh modified after its
//! first upload is re-made as `Dynamic` and rewritten in place from then on while its sizes hold,
//! so a mesh the game rewrites every frame (particles, ribbons, weather) does not recreate buffers.
//!
//! A program reads a fixed list of attributes ([`VertexInput`]); an attribute the mesh lacks is a
//! per-mesh filler stream of the input's default, as the wgpu path's shader defs leave the
//! attribute out and use the same constant. The mask of inputs the mesh has goes to the draw
//! block's tag, for a program whose shader defs change more than the constant.

use std::collections::HashMap;
use std::ptr;

use bevy::asset::AssetId;
use bevy::mesh::{
    Indices, Mesh, MeshVertexAttribute, MeshVertexAttributeId, PrimitiveTopology,
    VertexAttributeValues, VertexFormat,
};

use crate::ffi::{
    self, GfxAttributesState, GfxBuffer, GfxBufferType, GfxBufferUsage, GfxDevice, GfxFormat,
    GfxIndexType, GfxPrimitiveType,
};

/// One attribute a program's vertex stage reads, and what it reads where the mesh has none.
#[derive(Debug, Clone, Copy)]
pub struct VertexInput {
    pub attribute: MeshVertexAttribute,
    pub default: [f32; 4],
    /// The format the program reads the stream as, where it differs from the attribute's own
    /// mapping ([`vertex_format`]); the stream's bytes are the same size.
    pub read_as: Option<GfxFormat>,
}

impl VertexInput {
    pub const fn new(attribute: MeshVertexAttribute, default: [f32; 4]) -> Self {
        Self {
            attribute,
            default,
            read_as: None,
        }
    }

    /// Read as `format`, e.g. a `Uint32` slot as the `int` the shader languages share.
    pub const fn read_as(mut self, format: GfxFormat) -> Self {
        self.read_as = Some(format);
        self
    }
}

struct Stream {
    buffer: GfxBuffer,
    format: VertexFormat,
    bytes: u32,
}

struct IndexBuffer {
    buffer: GfxBuffer,
    ty: GfxIndexType,
    count: u32,
    bytes: u32,
}

/// A mesh on the device.
pub struct GpuMesh {
    streams: HashMap<MeshVertexAttributeId, Stream>,
    /// Filler streams by attribute, for programs that read one the mesh lacks.
    fillers: HashMap<MeshVertexAttributeId, Stream>,
    index: Option<IndexBuffer>,
    pub vertex_count: u32,
    pub primitive: GfxPrimitiveType,
    usage: GfxBufferUsage,
    /// Attribute states by program name: the streams in the program's input order, and the mask
    /// of inputs the mesh has.
    states: Vec<(&'static str, GfxAttributesState, u32)>,
}

impl GpuMesh {
    /// Indices to draw, if the mesh is indexed, else vertices.
    pub fn draw_count(&self) -> u32 {
        self.index.as_ref().map_or(self.vertex_count, |i| i.count)
    }

    pub fn indexed(&self) -> bool {
        self.index.is_some()
    }

    /// The attribute state binding this mesh's streams in `inputs` order, for `program`, and the
    /// mask of inputs the mesh has (bit `i` for input `i`).
    pub(crate) fn attributes_state(
        &mut self,
        device: GfxDevice,
        program: &'static str,
        inputs: &[VertexInput],
    ) -> Option<(GfxAttributesState, u32)> {
        if let Some((_, s, mask)) = self.states.iter().find(|(p, ..)| *p == program) {
            return Some((*s, *mask));
        }
        let mut binds = Vec::with_capacity(inputs.len());
        let mut mask = 0u32;
        for (i, input) in inputs.iter().enumerate() {
            let id = input.attribute.id;
            let own = self
                .streams
                .get(&id)
                .filter(|s| s.format == input.attribute.format);
            let buffer = match own {
                Some(s) => {
                    mask |= 1 << i;
                    s.buffer
                }
                None => {
                    if !self.fillers.contains_key(&id) {
                        let bytes = filler_bytes(input, self.vertex_count.max(1));
                        let buffer = create_buffer(
                            device,
                            GfxBufferType::Vertexes,
                            GfxBufferUsage::Static,
                            &bytes,
                        )?;
                        let stream = Stream {
                            buffer,
                            format: input.attribute.format,
                            bytes: bytes.len() as u32,
                        };
                        self.fillers.insert(id, stream);
                    }
                    self.fillers[&id].buffer
                }
            };
            binds.push(ffi::GfxAttributeBind { buffer });
        }
        let (index_buffer, index_type) = self
            .index
            .as_ref()
            .map_or((ptr::null_mut(), GfxIndexType::UInt16), |i| {
                (i.buffer, i.ty)
            });
        let info = ffi::GfxAttributesStateCreateInfo {
            binds: binds.as_ptr(),
            count: binds.len() as u32,
            index_buffer,
            index_type,
        };
        let mut state: GfxAttributesState = ptr::null_mut();
        // SAFETY: `info` and `binds` are live for the call; every buffer is this mesh's.
        if !unsafe { ffi::gfx_dll_create_attributes_state(device, &info, &mut state) } {
            return None;
        }
        self.states.push((program, state, mask));
        Some((state, mask))
    }

    fn delete(self, device: GfxDevice) {
        // SAFETY: every handle was made on `device` and belongs to this mesh alone.
        unsafe {
            for (_, s, _) in self.states {
                ffi::gfx_dll_delete_attributes_state(device, s);
            }
            for s in self.streams.into_values().chain(self.fillers.into_values()) {
                ffi::gfx_dll_delete_buffer(device, s.buffer);
            }
            if let Some(i) = self.index {
                ffi::gfx_dll_delete_buffer(device, i.buffer);
            }
        }
    }
}

/// Every uploaded mesh by asset id.
pub struct GpuMeshes {
    device: GfxDevice,
    meshes: HashMap<AssetId<Mesh>, GpuMesh>,
    /// Assets changed since their upload: rewritten on next use.
    stale: HashMap<AssetId<Mesh>, ()>,
}

impl GpuMeshes {
    pub fn new(device: GfxDevice) -> Self {
        Self {
            device,
            meshes: HashMap::new(),
            stale: HashMap::new(),
        }
    }

    /// The asset changed: the next use rewrites it.
    pub fn modified(&mut self, id: AssetId<Mesh>) {
        if self.meshes.contains_key(&id) {
            self.stale.insert(id, ());
        }
    }

    /// The asset is gone.
    pub fn removed(&mut self, id: AssetId<Mesh>) {
        self.stale.remove(&id);
        if let Some(m) = self.meshes.remove(&id) {
            m.delete(self.device);
        }
    }

    pub fn len(&self) -> usize {
        self.meshes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.meshes.is_empty()
    }

    /// The mesh on the device as it is, without uploading it.
    pub fn peek(&self, id: AssetId<Mesh>) -> Option<&GpuMesh> {
        self.meshes.get(&id)
    }

    /// The mesh on the device, uploading or rewriting it from `mesh` first when needed.
    pub fn get(&mut self, id: AssetId<Mesh>, mesh: &Mesh) -> Option<&mut GpuMesh> {
        if self.stale.remove(&id).is_some() {
            let rewritten = self
                .meshes
                .get_mut(&id)
                .is_some_and(|gpu| rewrite(self.device, gpu, mesh));
            if !rewritten {
                if let Some(old) = self.meshes.remove(&id) {
                    old.delete(self.device);
                }
                let gpu = upload(self.device, mesh, GfxBufferUsage::Dynamic)?;
                self.meshes.insert(id, gpu);
            }
        } else if !self.meshes.contains_key(&id) {
            let gpu = upload(self.device, mesh, GfxBufferUsage::Static)?;
            self.meshes.insert(id, gpu);
        }
        self.meshes.get_mut(&id)
    }

    /// Deletes every mesh; the device must still be alive.
    pub fn clear(&mut self) {
        for (_, m) in self.meshes.drain() {
            m.delete(self.device);
        }
        self.stale.clear();
    }
}

impl Drop for GpuMeshes {
    fn drop(&mut self) {
        self.clear();
    }
}

fn primitive(topology: PrimitiveTopology) -> GfxPrimitiveType {
    match topology {
        PrimitiveTopology::TriangleList => GfxPrimitiveType::Triangles,
        PrimitiveTopology::TriangleStrip => GfxPrimitiveType::TriangleStrip,
        PrimitiveTopology::LineList => GfxPrimitiveType::Lines,
        PrimitiveTopology::LineStrip => GfxPrimitiveType::LineStrip,
        PrimitiveTopology::PointList => GfxPrimitiveType::Points,
    }
}

fn upload(device: GfxDevice, mesh: &Mesh, usage: GfxBufferUsage) -> Option<GpuMesh> {
    let attributes = mesh.try_attributes().ok()?;
    let mut gpu = GpuMesh {
        streams: HashMap::new(),
        fillers: HashMap::new(),
        index: None,
        vertex_count: mesh.count_vertices() as u32,
        primitive: primitive(mesh.primitive_topology()),
        usage,
        states: Vec::new(),
    };
    for (attribute, values) in attributes {
        let bytes = values.get_bytes();
        if bytes.is_empty() {
            continue;
        }
        let Some(buffer) = create_buffer(device, GfxBufferType::Vertexes, usage, bytes) else {
            gpu.delete(device);
            return None;
        };
        gpu.streams.insert(
            attribute.id,
            Stream {
                buffer,
                format: attribute.format,
                bytes: bytes.len() as u32,
            },
        );
    }
    if let Some(indices) = mesh.try_indices_option().ok().flatten() {
        let (bytes, ty): (&[u8], _) = match indices {
            Indices::U16(v) => (bytemuck_u16(v), GfxIndexType::UInt16),
            Indices::U32(v) => (bytemuck_u32(v), GfxIndexType::UInt32),
        };
        if !bytes.is_empty() {
            let Some(buffer) = create_buffer(device, GfxBufferType::Indices, usage, bytes) else {
                gpu.delete(device);
                return None;
            };
            gpu.index = Some(IndexBuffer {
                buffer,
                ty,
                count: indices.len() as u32,
                bytes: bytes.len() as u32,
            });
        }
    }
    Some(gpu)
}

/// Rewrites a `Dynamic` mesh in place when every stream and the index buffer keep their size and
/// format; `false` when it must be re-made.
fn rewrite(device: GfxDevice, gpu: &mut GpuMesh, mesh: &Mesh) -> bool {
    if gpu.usage != GfxBufferUsage::Dynamic
        || primitive(mesh.primitive_topology()) != gpu.primitive
        || mesh.count_vertices() as u32 != gpu.vertex_count
    {
        return false;
    }
    let Ok(attributes) = mesh.try_attributes() else {
        return false;
    };
    let attributes: Vec<(&MeshVertexAttribute, &VertexAttributeValues)> = attributes.collect();
    let non_empty = attributes
        .iter()
        .filter(|(_, v)| !v.get_bytes().is_empty())
        .count();
    if non_empty != gpu.streams.len() {
        return false;
    }
    for (attribute, values) in &attributes {
        let bytes = values.get_bytes();
        if bytes.is_empty() {
            continue;
        }
        match gpu.streams.get(&attribute.id) {
            Some(s) if s.format == attribute.format && s.bytes == bytes.len() as u32 => {}
            _ => return false,
        }
    }
    let indices = mesh.try_indices_option().ok().flatten();
    let index_bytes: Option<&[u8]> = indices.map(|i| match i {
        Indices::U16(v) => bytemuck_u16(v),
        Indices::U32(v) => bytemuck_u32(v),
    });
    match (&gpu.index, index_bytes) {
        (None, None) => {}
        (Some(i), Some(b)) if i.bytes == b.len() as u32 => {}
        _ => return false,
    }
    for (attribute, values) in &attributes {
        let bytes = values.get_bytes();
        if let Some(s) = gpu.streams.get(&attribute.id) {
            set_buffer(device, s.buffer, bytes);
        }
    }
    if let (Some(i), Some(b)) = (&gpu.index, index_bytes) {
        set_buffer(device, i.buffer, b);
    }
    true
}

fn create_buffer(
    device: GfxDevice,
    ty: GfxBufferType,
    usage: GfxBufferUsage,
    bytes: &[u8],
) -> Option<GfxBuffer> {
    let info = ffi::GfxBufferCreateInfo {
        buffer_type: ty,
        usage,
        data: bytes.as_ptr().cast(),
        size: bytes.len() as u32,
    };
    let mut buffer: GfxBuffer = ptr::null_mut();
    // SAFETY: `info.data` points at `bytes`, live for the call, which copies it.
    unsafe { ffi::gfx_dll_create_buffer(device, &info, &mut buffer) }.then_some(buffer)
}

fn set_buffer(device: GfxDevice, buffer: GfxBuffer, bytes: &[u8]) {
    // SAFETY: `buffer` was made on `device` at this size; the call copies `bytes`.
    unsafe {
        ffi::gfx_dll_set_buffer_data(device, buffer, bytes.as_ptr().cast(), bytes.len() as u32, 0)
    };
}

/// `count` copies of `input.default` in the attribute's format; zeros for a format that is not
/// float-backed.
fn filler_bytes(input: &VertexInput, count: u32) -> Vec<u8> {
    let format = input.attribute.format;
    let lanes = match format {
        VertexFormat::Float32 => 1,
        VertexFormat::Float32x2 => 2,
        VertexFormat::Float32x3 => 3,
        VertexFormat::Float32x4 => 4,
        _ => 0,
    };
    let one: Vec<u8> = if lanes == 0 {
        vec![0; format.size() as usize]
    } else {
        input.default[..lanes]
            .iter()
            .flat_map(|f| f.to_le_bytes())
            .collect()
    };
    one.repeat(count as usize)
}

/// The gfx format a vertex stream is read as.
pub fn vertex_format(format: VertexFormat) -> Option<GfxFormat> {
    Some(match format {
        VertexFormat::Float32 => GfxFormat::R32Sfloat,
        VertexFormat::Float32x2 => GfxFormat::R32G32Sfloat,
        VertexFormat::Float32x3 => GfxFormat::R32G32B32Sfloat,
        VertexFormat::Float32x4 => GfxFormat::R32G32B32A32Sfloat,
        VertexFormat::Uint32 => GfxFormat::R32Uint,
        VertexFormat::Uint32x2 => GfxFormat::R32G32Uint,
        VertexFormat::Uint32x3 => GfxFormat::R32G32B32Uint,
        VertexFormat::Uint32x4 => GfxFormat::R32G32B32A32Uint,
        VertexFormat::Sint32 => GfxFormat::R32Sint,
        VertexFormat::Sint32x4 => GfxFormat::R32G32B32A32Sint,
        VertexFormat::Uint16x2 => GfxFormat::R16G16Uint,
        VertexFormat::Uint16x4 => GfxFormat::R16G16B16A16Uint,
        VertexFormat::Unorm16x2 => GfxFormat::R16G16Unorm,
        VertexFormat::Unorm16x4 => GfxFormat::R16G16B16A16Unorm,
        VertexFormat::Float16x2 => GfxFormat::R16G16Sfloat,
        VertexFormat::Float16x4 => GfxFormat::R16G16B16A16Sfloat,
        VertexFormat::Uint8x2 => GfxFormat::R8G8Uint,
        VertexFormat::Uint8x4 => GfxFormat::R8G8B8A8Uint,
        VertexFormat::Unorm8x2 => GfxFormat::R8G8Unorm,
        VertexFormat::Unorm8x4 => GfxFormat::R8G8B8A8Unorm,
        VertexFormat::Snorm8x4 => GfxFormat::R8G8B8A8Snorm,
        _ => return None,
    })
}

fn bytemuck_u16(v: &[u16]) -> &[u8] {
    // SAFETY: `u16` has no padding and any byte pattern is a valid `u8`.
    unsafe { std::slice::from_raw_parts(v.as_ptr().cast(), std::mem::size_of_val(v)) }
}

fn bytemuck_u32(v: &[u32]) -> &[u8] {
    // SAFETY: `u32` has no padding and any byte pattern is a valid `u8`.
    unsafe { std::slice::from_raw_parts(v.as_ptr().cast(), std::mem::size_of_val(v)) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_filler_repeats_the_default_in_the_attribute_format() {
        let color = VertexInput::new(Mesh::ATTRIBUTE_COLOR, [1.0, 0.5, 0.25, 1.0]);
        let bytes = filler_bytes(&color, 3);
        assert_eq!(bytes.len(), 3 * 16);
        assert_eq!(f32::from_le_bytes(bytes[20..24].try_into().unwrap()), 0.5);
        let uv = VertexInput::new(Mesh::ATTRIBUTE_UV_0, [0.0; 4]);
        assert_eq!(filler_bytes(&uv, 2), vec![0; 16]);
    }

    #[test]
    fn every_float_attribute_has_a_gfx_format() {
        for a in [
            Mesh::ATTRIBUTE_POSITION,
            Mesh::ATTRIBUTE_NORMAL,
            Mesh::ATTRIBUTE_UV_0,
            Mesh::ATTRIBUTE_UV_1,
            Mesh::ATTRIBUTE_TANGENT,
            Mesh::ATTRIBUTE_COLOR,
            Mesh::ATTRIBUTE_JOINT_WEIGHT,
            Mesh::ATTRIBUTE_JOINT_INDEX,
        ] {
            assert!(vertex_format(a.format).is_some(), "{}", a.name);
        }
    }
}
