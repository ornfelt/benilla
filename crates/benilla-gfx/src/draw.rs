//! The frame's draws. The registered materials turn visible entities into [`DrawItem`]s
//! ([`crate::material`]); [`draw_views`] then walks the active 3D cameras on the window in
//! `order`, and for each: clears as its `ClearColorConfig` says, clears depth to 0 (its own depth,
//! as each Bevy view has), and draws the items its `VisibleEntities` hold (frustum- and
//! `RenderLayers`-culled by bevy_camera) in bevy_core_pipeline's phases: opaque, alpha mask, then
//! transparent sorted back to front by view-space depth of the mesh origin. [`present`] then
//! encodes the scene target into the window.
//!
//! Every draw's constants go into one uniform ring written once per frame, before the first
//! draw, and bound per draw by offset.

use std::ptr;

use bevy::asset::AssetId;
use bevy::camera::visibility::VisibleEntities;
use bevy::camera::{ClearColorConfig, RenderTarget};
use bevy::ecs::entity::EntityHashMap;
use bevy::prelude::*;
use bevy::render::render_resource::Face;
use bevy::window::PrimaryWindow;

use crate::context::GfxContext;
use crate::ffi::{self, GfxBuffer, GfxDevice, GfxDeviceBackend, GfxTexture};
use crate::images::GpuImages;
use crate::material::{GfxAlpha, GfxMaterialDesc};
use crate::meshes::GpuMeshes;
use crate::pipelines::{Blend, PipelineKey, Pipelines};
use crate::target::{Present, SceneTarget};

/// One entity to draw: its mesh, its world matrix and its material description
/// ([`DrawList::desc`]).
#[derive(Debug, Clone, Copy)]
pub struct DrawItem {
    pub mesh: AssetId<Mesh>,
    pub world_from_local: Mat4,
    pub desc: u32,
}

/// This frame's draw items by entity, rebuilt each frame by the material collectors.
#[derive(Resource, Default)]
pub struct DrawList {
    items: Vec<DrawItem>,
    by_entity: EntityHashMap<u32>,
    descs: Vec<GfxMaterialDesc>,
}

impl DrawList {
    pub fn clear(&mut self) {
        self.items.clear();
        self.by_entity.clear();
        self.descs.clear();
    }

    pub fn push_desc(&mut self, desc: GfxMaterialDesc) -> u32 {
        self.descs.push(desc);
        self.descs.len() as u32 - 1
    }

    pub fn push(&mut self, entity: Entity, item: DrawItem) {
        self.by_entity.insert(entity, self.items.len() as u32);
        self.items.push(item);
    }

    pub fn get(&self, entity: Entity) -> Option<&DrawItem> {
        self.by_entity
            .get(&entity)
            .map(|i| &self.items[*i as usize])
    }

    pub fn desc(&self, index: u32) -> &GfxMaterialDesc {
        &self.descs[index as usize]
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

/// Everything the gfx renderer keeps on the device: the asset stores, pipelines, the scene
/// target, the present pass and the uniform ring. A non-send resource, removed before the
/// [`GfxContext`] whose device it lives on.
pub struct GfxRenderer {
    device: GfxDevice,
    backend: GfxDeviceBackend,
    pub meshes: GpuMeshes,
    pub images: GpuImages,
    pipelines: Pipelines,
    target: Option<SceneTarget>,
    present: Option<Present>,
    ring: UniformRing,
    /// Camera kinds not drawn yet, logged once each.
    skipped: Vec<&'static str>,
}

impl GfxRenderer {
    pub fn new(ctx: &mut GfxContext, default_sampler: bevy::image::ImageSamplerDescriptor) -> Self {
        let device = ctx.device;
        let backend = ctx.backends.device;
        let present = match ctx
            .shaders
            .get("present")
            .and_then(|p| Present::new(device, backend, p.state))
        {
            Ok(p) => Some(p),
            Err(e) => {
                error!("gfx: {e}");
                None
            }
        };
        Self {
            device,
            backend,
            meshes: GpuMeshes::new(device),
            images: GpuImages::new(device, default_sampler),
            pipelines: Pipelines::new(device),
            target: None,
            present,
            ring: UniformRing::new(device),
            skipped: Vec::new(),
        }
    }

    fn skip_once(&mut self, what: &'static str) {
        if !self.skipped.contains(&what) {
            info!("gfx: {what} not drawn yet");
            self.skipped.push(what);
        }
    }

    /// The scene target at `size`, re-made (with every pipeline made for it) when the window
    /// changed size.
    fn target(&mut self, size: UVec2) -> Option<&SceneTarget> {
        let size = size.max(UVec2::ONE);
        if self.target.as_ref().is_none_or(|t| t.size != size) {
            self.pipelines.clear_pipelines();
            self.target = None;
            match SceneTarget::new(self.device, size) {
                Ok(t) => self.target = Some(t),
                Err(e) => error!("gfx: {e}"),
            }
        }
        self.target.as_ref()
    }
}

impl Drop for GfxRenderer {
    fn drop(&mut self) {
        // Pipelines before the target they were made for; the stores drop themselves.
        self.pipelines.clear();
        self.present = None;
        self.target = None;
    }
}

/// `std140` sizes of the two constant blocks (`standard.vs.gfxs`).
const VIEW_BLOCK: usize = 80;
const DRAW_BLOCK: usize = 128;
/// d3d11 binds constants in whole 256-byte runs (`d3d11_bind_constant`): room past the last one.
const RING_TAIL: usize = 256;

/// The frame's constants: written on the CPU, uploaded once, bound per draw by offset.
struct UniformRing {
    device: GfxDevice,
    buffer: GfxBuffer,
    capacity: usize,
    align: usize,
    data: Vec<u8>,
}

impl UniformRing {
    fn new(device: GfxDevice) -> Self {
        // SAFETY: a plain query of the device's constant alignment.
        let align = unsafe { ffi::gfx_dll_get_uniform_buffer_size(device, 1) }.max(16) as usize;
        Self {
            device,
            buffer: ptr::null_mut(),
            capacity: 0,
            align,
            data: Vec::new(),
        }
    }

    fn push(&mut self, floats: &[f32]) -> u32 {
        let offset = self.data.len().next_multiple_of(self.align);
        self.data.resize(offset, 0);
        self.data
            .extend(floats.iter().flat_map(|f| f.to_le_bytes()));
        offset as u32
    }

    /// Uploads the frame's constants, growing the buffer first when they no longer fit.
    fn upload(&mut self) -> bool {
        let needed = self.data.len() + RING_TAIL;
        if needed > self.capacity || self.buffer.is_null() {
            let capacity = needed.next_power_of_two().max(64 * 1024);
            if !self.buffer.is_null() {
                // SAFETY: made on this device; nothing is drawn with it past this point.
                unsafe { ffi::gfx_dll_delete_buffer(self.device, self.buffer) };
                self.buffer = ptr::null_mut();
            }
            let info = ffi::GfxBufferCreateInfo {
                buffer_type: ffi::GfxBufferType::Uniform,
                usage: ffi::GfxBufferUsage::Dynamic,
                data: ptr::null(),
                size: capacity as u32,
            };
            // SAFETY: `info` is live for the call.
            if !unsafe { ffi::gfx_dll_create_buffer(self.device, &info, &mut self.buffer) } {
                self.capacity = 0;
                return false;
            }
            self.capacity = capacity;
        }
        self.data.resize(self.data.len() + RING_TAIL, 0);
        // SAFETY: the buffer holds `capacity >= data.len()` bytes; the call copies `data`.
        unsafe {
            ffi::gfx_dll_set_buffer_data(
                self.device,
                self.buffer,
                self.data.as_ptr().cast(),
                self.data.len() as u32,
                0,
            )
        }
    }
}

impl Drop for UniformRing {
    fn drop(&mut self) {
        if !self.buffer.is_null() {
            // SAFETY: made on this device, owned by the ring alone.
            unsafe { ffi::gfx_dll_delete_buffer(self.device, self.buffer) };
        }
    }
}

/// A camera drawing this frame, as the draw needs it.
struct View {
    order: isize,
    viewport: URect,
    clear: Option<LinearRgba>,
    clip_from_world: Mat4,
    view_from_world: Mat4,
    invert_culling: bool,
    entity: Entity,
}

/// One recorded command, executed after the ring is uploaded.
enum Cmd {
    View {
        viewport: URect,
        clear: Option<LinearRgba>,
    },
    Draw {
        pipeline: ffi::GfxPipeline,
        layout: ffi::GfxInputLayout,
        attributes: ffi::GfxAttributesState,
        texture: GfxTexture,
        view_offset: u32,
        draw_offset: u32,
        count: u32,
        indexed: bool,
    },
}

/// GL clips depth to [-1, 1] where Bevy's projections produce [0, 1]: `z' = 2z - w` puts Bevy's
/// reverse-Z range where GL's window transform maps it back to the same [0, 1] depth values.
fn clip_remap(backend: GfxDeviceBackend) -> Mat4 {
    match backend {
        GfxDeviceBackend::Gl3 | GfxDeviceBackend::Gl4 | GfxDeviceBackend::Gles3 => Mat4::from_cols(
            Vec4::X,
            Vec4::Y,
            Vec4::new(0.0, 0.0, 2.0, 0.0),
            Vec4::new(0.0, 0.0, -1.0, 1.0),
        ),
        _ => Mat4::IDENTITY,
    }
}

/// Resets the draw list and applies this frame's asset changes to the device stores.
pub(crate) fn prepare(
    mut renderer: NonSendMut<GfxRenderer>,
    mut list: ResMut<DrawList>,
    mut mesh_events: MessageReader<AssetEvent<Mesh>>,
    mut image_events: MessageReader<AssetEvent<Image>>,
) {
    list.clear();
    for e in mesh_events.read() {
        match *e {
            AssetEvent::Modified { id } => renderer.meshes.modified(id),
            AssetEvent::Removed { id } | AssetEvent::Unused { id } => renderer.meshes.removed(id),
            _ => {}
        }
    }
    for e in image_events.read() {
        match *e {
            AssetEvent::Modified { id } => renderer.images.modified(id),
            AssetEvent::Removed { id } | AssetEvent::Unused { id } => renderer.images.removed(id),
            _ => {}
        }
    }
}

type CameraItem = (
    Entity,
    &'static Camera,
    &'static RenderTarget,
    &'static GlobalTransform,
    &'static VisibleEntities,
    Has<Camera3d>,
);

/// Draws every active 3D camera on the primary window into the scene target, in `order`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_views(
    mut ctx: NonSendMut<GfxContext>,
    mut renderer: NonSendMut<GfxRenderer>,
    list: Res<DrawList>,
    meshes: Res<Assets<Mesh>>,
    images: Res<Assets<Image>>,
    clear_color: Option<Res<ClearColor>>,
    primary: Query<Entity, With<PrimaryWindow>>,
    cameras: Query<CameraItem>,
) {
    let size = ctx.size();
    let default_clear = clear_color.map_or(Color::BLACK, |c| c.0).to_linear();
    let primary = primary.single().ok();
    let remap = clip_remap(renderer.backend);

    let mut views = Vec::new();
    for (entity, camera, target, transform, _, is_3d) in &cameras {
        if !camera.is_active {
            continue;
        }
        let on_window = matches!(
            target.normalize(primary),
            Some(bevy::camera::NormalizedRenderTarget::Window(w)) if Some(w.entity()) == primary
        );
        if !on_window {
            renderer.skip_once("a camera on an image or other target");
            continue;
        }
        if !is_3d {
            renderer.skip_once("a 2D or UI camera");
            continue;
        }
        let Some(viewport) = camera.physical_viewport_rect() else {
            continue;
        };
        let view_from_world = transform.to_matrix().inverse();
        views.push(View {
            order: camera.order,
            viewport,
            clear: match camera.clear_color {
                ClearColorConfig::Default => Some(default_clear),
                ClearColorConfig::Custom(c) => Some(c.to_linear()),
                ClearColorConfig::None => None,
            },
            clip_from_world: remap * camera.clip_from_view() * view_from_world,
            view_from_world,
            invert_culling: camera.invert_culling,
            entity,
        });
    }
    views.sort_by_key(|v| v.order);

    let renderer = &mut *renderer;
    let Some(framebuffer) = renderer.target(size).map(|t| t.framebuffer) else {
        return;
    };
    let device = renderer.device;
    // SAFETY: the live device and scene target, on the device's thread.
    unsafe {
        ffi::gfx_dll_bind_framebuffer(device, framebuffer);
        ffi::gfx_dll_set_viewport(device, 0, 0, size.x, size.y, 0.0, 1.0);
        ffi::gfx_dll_set_scissor(device, 0, 0, size.x, size.y);
        // With no camera on the window Bevy still clears it to `ClearColor`.
        ffi::gfx_dll_clear_color(device, framebuffer, 0, &gfx_color(default_clear));
        ffi::gfx_dll_clear_depth_stencil(device, framebuffer, 0.0, 0);
    }

    renderer.ring.data.clear();
    let mut cmds = Vec::new();
    for view in &views {
        let Ok((_, _, _, _, visible, _)) = cameras.get(view.entity) else {
            continue;
        };
        let eye = view.view_from_world.inverse().w_axis;
        let mut block = [0.0f32; VIEW_BLOCK / 4];
        block[..16].copy_from_slice(&view.clip_from_world.to_cols_array());
        block[16..20].copy_from_slice(&eye.to_array());
        let view_offset = renderer.ring.push(&block);
        cmds.push(Cmd::View {
            viewport: view.viewport,
            clear: view.clear,
        });

        // Opaque and mask first (sorted by pipeline, texture, mesh, as bins batch), then
        // transparent back to front.
        let (mut opaque, mut transparent) = (Vec::new(), Vec::new());
        for entity in visible.iter(std::any::TypeId::of::<Mesh3d>()) {
            let Some(item) = list.get(*entity) else {
                continue;
            };
            let desc = list.desc(item.desc);
            let Some(mesh) = meshes.get(item.mesh) else {
                continue;
            };
            let Some(gpu) = renderer.meshes.get(item.mesh, mesh) else {
                continue;
            };
            let primitive = gpu.primitive;
            let (count, indexed) = (gpu.draw_count(), gpu.indexed());
            let Ok(program) = ctx.shaders.get(desc.program.name) else {
                renderer.skip_once("a material whose program failed to load");
                continue;
            };
            let shader_state = program.state;
            let Some(attributes) =
                gpu.attributes_state(device, desc.program.name, desc.program.inputs)
            else {
                continue;
            };
            let Some(layout) = renderer.pipelines.layout(&desc.program, shader_state) else {
                continue;
            };
            let transparent_phase = desc.alpha.is_transparent();
            let key = PipelineKey {
                program: desc.program.name,
                blend: Blend::of(desc.alpha),
                cull: cull(desc.cull, view.invert_culling),
                depth_write: !transparent_phase,
                depth_test: true,
                primitive,
            };
            let Some(pipeline) = renderer
                .pipelines
                .get(key, shader_state, layout, framebuffer)
            else {
                renderer.skip_once("a pipeline gfx refused");
                continue;
            };
            let texture = match desc.texture {
                Some(id) => match images.get(id).and_then(|i| renderer.images.get(id, i)) {
                    Some(t) => t.texture,
                    // Bevy skips a material whose texture is not loaded yet.
                    None => continue,
                },
                None => renderer.images.white,
            };
            let draw_offset = renderer.ring.push(&draw_block(item, desc));
            let cmd = Cmd::Draw {
                pipeline,
                layout,
                attributes,
                texture,
                view_offset,
                draw_offset,
                count,
                indexed,
            };
            if transparent_phase {
                let z = view
                    .view_from_world
                    .transform_point3(item.world_from_local.w_axis.truncate())
                    .z;
                transparent.push((z, cmd));
            } else {
                let rank = match desc.alpha {
                    GfxAlpha::Mask(_) => 1u8,
                    _ => 0,
                };
                opaque.push(((rank, pipeline as usize, texture as usize, item.mesh), cmd));
            }
        }
        opaque.sort_by_key(|(key, _)| *key);
        cmds.extend(opaque.into_iter().map(|(_, c)| c));
        // Farthest first: the most negative view-space z.
        transparent.sort_by(|a, b| a.0.total_cmp(&b.0));
        cmds.extend(transparent.into_iter().map(|(_, c)| c));
    }

    if !renderer.ring.upload() {
        error!("gfx: the uniform ring could not be uploaded");
        return;
    }
    let ring = renderer.ring.buffer;
    execute(device, framebuffer, ring, &cmds);
}

fn execute(device: GfxDevice, framebuffer: ffi::GfxFramebuffer, ring: GfxBuffer, cmds: &[Cmd]) {
    let mut bound = ptr::null_mut();
    for cmd in cmds {
        match *cmd {
            Cmd::View { viewport, clear } => {
                let size = viewport.size();
                // SAFETY: the live device and scene target, on the device's thread.
                unsafe {
                    ffi::gfx_dll_set_viewport(
                        device,
                        viewport.min.x as i32,
                        viewport.min.y as i32,
                        size.x,
                        size.y,
                        0.0,
                        1.0,
                    );
                    ffi::gfx_dll_set_scissor(
                        device,
                        viewport.min.x as i32,
                        viewport.min.y as i32,
                        size.x,
                        size.y,
                    );
                    if let Some(c) = clear {
                        ffi::gfx_dll_clear_color(device, framebuffer, 0, &gfx_color(c));
                    }
                    ffi::gfx_dll_clear_depth_stencil(device, framebuffer, 0.0, 0);
                }
            }
            Cmd::Draw {
                pipeline,
                layout,
                attributes,
                texture,
                view_offset,
                draw_offset,
                count,
                indexed,
            } => {
                let mut textures = [texture];
                // SAFETY: every handle is live and made on `device`; the ring holds both blocks.
                unsafe {
                    if pipeline != bound {
                        ffi::gfx_dll_bind_pipeline(device, pipeline);
                        bound = pipeline;
                    }
                    ffi::gfx_dll_bind_attributes_state(device, attributes, layout);
                    ffi::gfx_dll_bind_constant(device, 0, ring, VIEW_BLOCK as u32, view_offset);
                    ffi::gfx_dll_bind_constant(device, 1, ring, DRAW_BLOCK as u32, draw_offset);
                    ffi::gfx_dll_bind_samplers(device, 0, 1, textures.as_mut_ptr());
                    if indexed {
                        ffi::gfx_dll_draw_indexed(device, count, 0);
                    } else {
                        ffi::gfx_dll_draw(device, count, 0);
                    }
                }
            }
        }
    }
}

/// `draw_block` of `standard.vs.gfxs`.
fn draw_block(item: &DrawItem, desc: &GfxMaterialDesc) -> [f32; DRAW_BLOCK / 4] {
    let mut b = [0.0f32; DRAW_BLOCK / 4];
    b[..16].copy_from_slice(&item.world_from_local.to_cols_array());
    b[16..20].copy_from_slice(&desc.base_color.to_f32_array());
    let m = desc.uv_transform.matrix2;
    b[20..24].copy_from_slice(&[m.x_axis.x, m.x_axis.y, m.y_axis.x, m.y_axis.y]);
    let t = desc.uv_transform.translation;
    b[24..28].copy_from_slice(&[t.x, t.y, desc.alpha.cutoff(), desc.alpha.shader_mode()]);
    b
}

/// The face culled: the material's, swapped under the camera's `invert_culling`.
fn cull(face: Option<Face>, invert: bool) -> Option<Face> {
    match (face, invert) {
        (Some(Face::Back), true) => Some(Face::Front),
        (Some(Face::Front), true) => Some(Face::Back),
        (f, _) => f,
    }
}

fn gfx_color(c: LinearRgba) -> ffi::GfxClearColor {
    ffi::GfxClearColor {
        r: c.red,
        g: c.green,
        b: c.blue,
        a: c.alpha,
    }
}

/// Encodes the scene target into the window.
pub(crate) fn present(ctx: NonSend<GfxContext>, renderer: NonSend<GfxRenderer>) {
    let size = ctx.size();
    if let (Some(present), Some(target)) = (&renderer.present, &renderer.target) {
        present.draw(target.color, size);
    }
}

/// Logs the counts once a second at `debug`, for the run log.
pub(crate) fn log_stats(
    renderer: NonSend<GfxRenderer>,
    list: Res<DrawList>,
    time: Res<Time<Real>>,
    mut last: Local<f32>,
) {
    let now = time.elapsed_secs();
    if now - *last >= 1.0 {
        *last = now;
        debug!(
            "gfx: {} draw items, {} meshes and {} images on the device",
            list.len(),
            renderer.meshes.len(),
            renderer.images.len()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gl_remap_keeps_bevy_depth_values() {
        let remap = clip_remap(GfxDeviceBackend::Gl4);
        for z in [0.0f32, 0.25, 1.0] {
            let clip = remap * Vec4::new(0.0, 0.0, z * 2.0, 2.0);
            // GL window depth = (ndc + 1) / 2.
            let depth = (clip.z / clip.w + 1.0) / 2.0;
            assert!((depth - z).abs() < 1e-6, "{z} -> {depth}");
        }
        assert_eq!(clip_remap(GfxDeviceBackend::Vk), Mat4::IDENTITY);
    }

    #[test]
    fn inverted_culling_swaps_the_face() {
        assert_eq!(cull(Some(Face::Back), true), Some(Face::Front));
        assert_eq!(cull(None, true), None);
        assert_eq!(cull(Some(Face::Back), false), Some(Face::Back));
    }

    #[test]
    fn the_ring_aligns_every_block() {
        let mut ring = UniformRing {
            device: ptr::null_mut(),
            buffer: ptr::null_mut(),
            capacity: 0,
            align: 256,
            data: Vec::new(),
        };
        assert_eq!(ring.push(&[1.0; VIEW_BLOCK / 4]), 0);
        assert_eq!(ring.push(&[2.0; DRAW_BLOCK / 4]), 256);
        assert_eq!(ring.push(&[3.0; DRAW_BLOCK / 4]), 512);
        assert_eq!(ring.data.len(), 512 + DRAW_BLOCK);
        assert_eq!(
            f32::from_le_bytes(ring.data[256..260].try_into().unwrap()),
            2.0
        );
    }
}
