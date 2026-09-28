//! The frame's draws. The registered materials turn visible entities into [`DrawItem`]s
//! ([`crate::material`]); [`draw_views`] then walks the active 3D cameras on the window in
//! `order`, and for each: clears as its `ClearColorConfig` says, clears depth to 0 (its own depth,
//! as each Bevy view has), and draws the items its `VisibleEntities` hold (frustum- and
//! `RenderLayers`-culled by bevy_camera) in bevy_core_pipeline's phases: opaque, alpha mask, then
//! transparent sorted back to front by view-space depth of the mesh origin. [`present`] then
//! encodes the scene target into the window.
//!
//! A system can also place draws itself ([`DrawList::push_early`]): an index range of a mesh with
//! a description, drawn for one camera before its opaque phase in the order pushed, as a render
//! graph node between bevy's `StartMainPass` and `MainOpaquePass` draws (benilla's static-gx pass).
//! [`DrawList::push_sorted`] places one in the camera's transparent phase instead, sorted with the
//! entities by its own point and bias, as a render-world lane queues its own `Transparent3d`
//! items (benilla's effect lane). [`DrawList::push_late`] places one after a UI lane's `Mesh2d`
//! draws, in the order pushed, through a projection of its own: bevy_ui's pass, which runs after
//! the 2D main pass on its own UI view ([`crate::bevy_ui`]). A 2D camera on the window with a
//! [`GfxOverlays`] frame draws that into its own target and blends it over the scene through its
//! output blend ([`crate::overlay`], bevy_egui's pass).
//!
//! A camera on an image draws into that image's [`crate::target::ImageTarget`]: its main pair, shared by every
//! camera on the image as bevy shares main textures per target, cleared whole, drawn over the
//! camera's viewport, glowed, then copied into the image over the viewport by bevy's `upscaling`
//! blit, the image's first blit of the frame clearing it with the camera's output clear colour. A
//! 3D camera draws its phases there, a 2D one its `Mesh2d` items as a UI lane does.
//!
//! Every draw's constants go into one uniform ring written once per frame, before the first
//! draw, and bound per draw by offset: the view block (bevy_render's `View` fields the programs
//! read) and a draw block of the world matrix, the tag row and the program's parameter rows.

use std::collections::HashMap;
use std::ops::Range;
use std::ptr;

use bevy::asset::AssetId;
use bevy::camera::visibility::VisibleEntities;
use bevy::camera::{
    CameraOutputMode, ClearColorConfig, Exposure, NormalizedRenderTarget, RenderTarget,
};
use bevy::ecs::entity::EntityHashMap;
use bevy::light::{AmbientLight, GlobalAmbientLight};
use bevy::prelude::*;
use bevy::render::camera::MipBias;
use bevy::render::render_resource::Face;
use bevy::render::view::Msaa;
use bevy::window::PrimaryWindow;

use crate::context::GfxContext;
use crate::data::{GfxDataTextures, GpuDataTextures};
use crate::ffi::{self, GfxBuffer, GfxDevice, GfxDeviceBackend, GfxTexture};
use crate::images::{GfxTextureWrites, GpuImages};
use crate::material::{GfxAlpha, GfxMaterialDesc, GfxTextureSlot, MAX_TEXTURES};
use crate::meshes::GpuMeshes;
use crate::overlay::{GfxOverlays, OverlayPass};
use crate::pipelines::{Blend, PipelineKey, Pipelines};
use crate::post::{FfxPost, GfxFfxGlow};
use crate::shader_loader::ShaderLibrary;
use crate::target::{Present, SceneTarget, TargetClass, UiTarget, UI_FORMAT};
use crate::ui::GfxUiLane;

/// One entity to draw: its mesh, its world matrix, the world point it sorts by, its `MeshTag`
/// and its material description ([`DrawList::desc`]).
#[derive(Debug, Clone, Copy)]
pub struct DrawItem {
    pub mesh: AssetId<Mesh>,
    pub world_from_local: Mat4,
    pub center: Vec3,
    pub tag: u32,
    pub desc: u32,
}

/// A draw placed by a system rather than an entity: `indices` of an indexed mesh, drawn with a
/// description before the camera's opaque phase ([`DrawList::push_early`]).
#[derive(Debug, Clone)]
pub struct EarlyDraw {
    pub mesh: AssetId<Mesh>,
    pub indices: Range<u32>,
    pub world_from_local: Mat4,
    pub desc: u32,
}

/// A draw placed by a system in a camera's transparent phase ([`DrawList::push_sorted`]):
/// `indices` of an indexed mesh with a description, sorted by the view z of `anchor` plus `bias`,
/// the `Transparent3d` distance, whatever the description's alpha.
#[derive(Debug, Clone)]
pub struct SortedDraw {
    pub mesh: AssetId<Mesh>,
    pub indices: Range<u32>,
    pub world_from_local: Mat4,
    pub desc: u32,
    pub anchor: Vec3,
    pub bias: f32,
}

/// A draw placed after a UI lane camera's `Mesh2d` draws ([`DrawList::push_late`]): `indices` of
/// an indexed mesh with a description, its positions through `clip_from_world` in place of the
/// camera's (the GL depth remap is the renderer's to add).
#[derive(Debug, Clone)]
pub struct LateDraw {
    pub mesh: AssetId<Mesh>,
    pub indices: Range<u32>,
    pub clip_from_world: Mat4,
    pub desc: u32,
}

/// This frame's draw items by entity, rebuilt each frame by the material collectors.
#[derive(Resource, Default)]
pub struct DrawList {
    items: Vec<DrawItem>,
    by_entity: EntityHashMap<u32>,
    descs: Vec<GfxMaterialDesc>,
    early: Vec<(Entity, EarlyDraw)>,
    sorted: Vec<(Entity, SortedDraw)>,
    late: Vec<(Entity, LateDraw)>,
}

impl DrawList {
    pub fn clear(&mut self) {
        self.items.clear();
        self.by_entity.clear();
        self.descs.clear();
        self.early.clear();
        self.sorted.clear();
        self.late.clear();
    }

    /// Draws `draw` for `camera` before its opaque phase, after the early draws pushed before it.
    pub fn push_early(&mut self, camera: Entity, draw: EarlyDraw) {
        self.early.push((camera, draw));
    }

    /// Draws `draw` in `camera`'s transparent phase, sorted with its entities.
    pub fn push_sorted(&mut self, camera: Entity, draw: SortedDraw) {
        self.sorted.push((camera, draw));
    }

    /// Draws `draw` after UI lane `camera`'s `Mesh2d` draws, after the late draws pushed before it.
    pub fn push_late(&mut self, camera: Entity, draw: LateDraw) {
        self.late.push((camera, draw));
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
    pub data: GpuDataTextures,
    pipelines: Pipelines,
    target: Option<SceneTarget>,
    /// The UI lane's byte target, made at the scene target's size while a lane draws.
    ui: Option<UiTarget>,
    /// The overlay cameras' pass and target ([`crate::overlay`]).
    overlay: OverlayPass,
    pub(crate) present: Option<Present>,
    /// The screenshot capture texture ([`crate::screenshot`]).
    pub(crate) capture: Option<crate::screenshot::CaptureTarget>,
    post: Option<FfxPost>,
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
            data: GpuDataTextures::new(device),
            pipelines: Pipelines::new(device),
            target: None,
            ui: None,
            overlay: OverlayPass::new(device),
            present,
            capture: None,
            post: FfxPost::new(device, backend),
            ring: UniformRing::new(device),
            skipped: Vec::new(),
        }
    }

    pub(crate) fn device(&self) -> GfxDevice {
        self.device
    }

    pub(crate) fn backend(&self) -> GfxDeviceBackend {
        self.backend
    }

    /// The scene target's finished colour, the one the present encodes.
    pub(crate) fn scene_color(&self) -> Option<GfxTexture> {
        self.target.as_ref().map(|t| t.color())
    }

    fn skip_once(&mut self, what: &'static str) {
        if !self.skipped.contains(&what) {
            info!("gfx: {what} not drawn yet");
            self.skipped.push(what);
        }
    }

    /// The scene target at `size`, re-made (with every pipeline made for it) when the window
    /// changed size; each frame starts on its first colour.
    fn target(&mut self, size: UVec2) -> Option<&SceneTarget> {
        let size = size.max(UVec2::ONE);
        if self.target.as_ref().is_none_or(|t| t.size != size) {
            self.pipelines.clear_pipelines();
            if let Some(post) = &mut self.post {
                post.drop_targets();
            }
            self.target = None;
            self.ui = None;
            self.overlay.drop_target();
            match SceneTarget::new(self.device, size) {
                Ok(t) => self.target = Some(t),
                Err(e) => error!("gfx: {e}"),
            }
        }
        if let Some(t) = &mut self.target {
            t.current = 0;
        }
        self.target.as_ref()
    }
}

impl GfxRenderer {
    /// The UI lane's target at the scene target's `size`, made on first use.
    fn ui_target(&mut self, size: UVec2) -> Option<&UiTarget> {
        if self.ui.is_none() {
            match UiTarget::new(self.device, size.max(UVec2::ONE)) {
                Ok(t) => self.ui = Some(t),
                Err(e) => {
                    error!("gfx: {e}");
                    return None;
                }
            }
        }
        self.ui.as_ref()
    }
}

impl Drop for GfxRenderer {
    fn drop(&mut self) {
        // Pipelines before the target they were made for; the stores drop themselves.
        self.pipelines.clear();
        self.post = None;
        self.capture = None;
        self.present = None;
        self.ui = None;
        self.overlay.drop_target();
        self.target = None;
    }
}

/// `std140` size of the view block every program declares: `clip_from_world`,
/// `view_from_world`, `clip_from_view` (each GL-remapped where it reaches clip space), then
/// `world_position`, `viewport` (x, y, width, height in pixels), `misc` (x = the mip bias, y = 1
/// where the clip matrices carry the GL remap and the target's rows run bottom-up, 2 where they
/// carry it over a target drawn top-down (an image target on GL), z = the target height in pixels,
/// w = bevy's
/// `globals.time`, the wrapped elapsed seconds) and `ambient` (bevy_pbr's `lights.ambient_color`:
/// the view's ambient colour times its brightness; w = the view's exposure). A program may declare
/// the block without its trailing rows.
const VIEW_BLOCK: usize = 256;
/// The draw block's fixed head: `world_from_local` and the tag row (`uvec4`: x = the `MeshTag`,
/// y = the mask of program inputs the mesh has); the parameter rows follow.
const DRAW_HEAD: usize = 80;
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
#[derive(Clone)]
struct View {
    order: isize,
    viewport: URect,
    clear: Option<LinearRgba>,
    clip_from_world: Mat4,
    clip_from_view: Mat4,
    view_from_world: Mat4,
    mip_bias: f32,
    gl_remap: bool,
    time: f32,
    target_height: u32,
    invert_culling: bool,
    glow: Option<GfxFfxGlow>,
    /// A UI lane's display gamma: this 2D view draws into the lane's byte target and decodes.
    ui_lane: Option<f32>,
    /// A world view a UI lane claims: its combine grounds the lane.
    claimed: bool,
    ambient: [f32; 4],
    entity: Entity,
    /// Where it draws: the scene target, or the image of a `RenderTarget::Image` camera.
    dest: Dest,
    /// Drawn upside down, so the target's rows run top-down on GL (`ImageTarget`).
    top_down: bool,
    /// The target's size in pixels.
    target_size: UVec2,
    is_3d: bool,
    hdr: bool,
    /// An image camera's `upscaling` clear, and whether it set a viewport (the blit's scissor).
    output_clear: Option<LinearRgba>,
    has_viewport: bool,
    /// The camera's `Msaa` sample count: above 1, a 3D view draws into its target's multisampled
    /// pair and resolves.
    samples: u32,
    /// A 2D window camera drawing its [`GfxOverlays`] frame.
    overlay: bool,
    /// A window camera's output blend (bevy's `upscaling` over the window).
    output_blend: Blend,
}

/// A view's destination.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Dest {
    Scene,
    Image(AssetId<Image>),
}

/// A pixel rect in gfx's convention, y counted from the bottom of the target.
type GfxRect = (i32, i32, UVec2);

/// `rect` (bevy's, top-down) on a `height` target as gfx counts it: from the bottom, except on a
/// target drawn top-down, whose rows already run as bevy's.
fn gfx_rect(rect: URect, height: u32, top_down: bool) -> GfxRect {
    let size = rect.size();
    let y = if top_down {
        rect.min.y as i32
    } else {
        height as i32 - rect.max.y as i32
    };
    (rect.min.x as i32, y, size)
}

/// One recorded command, executed after the ring is uploaded.
enum Cmd {
    /// A window camera starts on the scene target, or its multisampled target when `msaa`.
    View {
        viewport: URect,
        clear: Option<LinearRgba>,
        msaa: bool,
    },
    /// An image camera starts: its main pair (or its multisampled target when `msaa`) bound over
    /// `rect`, cleared whole as a wgpu clear op clears its attachment (colour when `clear`, depth
    /// when the pair has one).
    ImageView {
        image: AssetId<Image>,
        rect: GfxRect,
        clear: Option<LinearRgba>,
        msaa: bool,
    },
    /// A multisampled view's end: its colour resolved into `dest`'s current colour.
    Resolve { dest: Dest },
    /// The camera's FFXGlow chain, its four blocks at these ring offsets, and the wave LUT of an
    /// armed underwater warp; `into_ui` clears the UI lane's target and combines into it. Run on
    /// `dest`'s colour, its quarter targets a quarter of `viewport`.
    Glow {
        dest: Dest,
        viewport: UVec2,
        offsets: [u32; 4],
        wave: Option<GfxTexture>,
        into_ui: bool,
    },
    /// Bevy's `upscaling` of an image camera: its finished main colour into the image, over
    /// `scissor` (none: the whole image), the image's first blit of the frame clearing it first.
    Blit {
        image: AssetId<Image>,
        scissor: Option<GfxRect>,
        clear: Option<LinearRgba>,
    },
    /// A UI lane view starts: its target bound, cleared unless a claimed combine grounded it.
    UiBegin {
        viewport: URect,
        clear: Option<LinearRgba>,
    },
    /// The UI lane's decode into the scene target, its block at this ring offset.
    UiDecode { offset: u32, viewport: URect },
    /// An overlay camera: the staged draws `draws` (start, end) into the overlay target, cleared
    /// with `clear`, through the transform block at `offset`; then its `upscaling` over the frame,
    /// its block at `composite`.
    Overlay {
        draws: (usize, usize),
        offset: u32,
        clear: Option<LinearRgba>,
        composite: u32,
    },
    Draw {
        pipeline: ffi::GfxPipeline,
        layout: ffi::GfxInputLayout,
        attributes: ffi::GfxAttributesState,
        textures: [GfxTexture; MAX_TEXTURES],
        texture_count: u32,
        view_offset: u32,
        draw_offset: u32,
        draw_size: u32,
        count: u32,
        first: u32,
        indexed: bool,
    },
}

/// A draw's device state, resolved from its mesh and description.
struct Resolved {
    pipeline: ffi::GfxPipeline,
    layout: ffi::GfxInputLayout,
    attributes: ffi::GfxAttributesState,
    textures: [GfxTexture; MAX_TEXTURES],
    texture_count: u32,
    count: u32,
    indexed: bool,
    /// The mask of program inputs the mesh has.
    present: u32,
}

impl Resolved {
    fn cmd(
        &self,
        view_offset: u32,
        draw_offset: u32,
        draw_size: u32,
        indices: Option<&Range<u32>>,
    ) -> Cmd {
        let (count, first) = indices.map_or((self.count, 0), |r| (r.len() as u32, r.start));
        Cmd::Draw {
            pipeline: self.pipeline,
            layout: self.layout,
            attributes: self.attributes,
            textures: self.textures,
            texture_count: self.texture_count,
            view_offset,
            draw_offset,
            draw_size,
            count,
            first,
            indexed: self.indexed,
        }
    }
}

/// Resolves one draw of `mesh` with `desc` for `view`; `None`, and not drawn, while anything it
/// needs is not on the device yet.
#[allow(clippy::too_many_arguments)]
fn resolve(
    renderer: &mut GfxRenderer,
    shaders: &mut ShaderLibrary,
    (framebuffer, class): (ffi::GfxFramebuffer, TargetClass),
    view: &View,
    mesh_id: AssetId<Mesh>,
    desc: &GfxMaterialDesc,
    meshes: &Assets<Mesh>,
    images: &Assets<Image>,
) -> Option<Resolved> {
    let device = renderer.device;
    let mesh = meshes.get(mesh_id)?;
    let gpu = renderer.meshes.get(mesh_id, mesh)?;
    let primitive = gpu.primitive;
    let (count, indexed) = (gpu.draw_count(), gpu.indexed());
    let Ok(program) = shaders.get(desc.program.name) else {
        renderer.skip_once("a material whose program failed to load");
        return None;
    };
    let shader_state = program.state;
    let (attributes, present) =
        gpu.attributes_state(device, desc.program.name, desc.program.inputs)?;
    let layout = renderer.pipelines.layout(&desc.program, shader_state)?;
    let transparent_phase = desc.alpha.is_transparent();
    let key = PipelineKey {
        program: desc.program.name,
        blend: desc.state.blend.unwrap_or(Blend::of(desc.alpha)),
        color_write: desc.state.color_write,
        cull: cull(desc.cull, view.invert_culling),
        depth_write: desc.state.depth_write.unwrap_or(!transparent_phase),
        depth_test: desc.state.depth_test,
        depth_always: desc.state.depth_always,
        depth_strict: desc.state.depth_strict,
        depth_bias: (desc.state.raster_bias, desc.state.raster_slope.to_bits()),
        primitive,
        target: class,
    };
    let Some(pipeline) = renderer
        .pipelines
        .get(key, shader_state, layout, framebuffer)
    else {
        renderer.skip_once("a pipeline gfx refused");
        return None;
    };
    let mut textures = [ptr::null_mut(); MAX_TEXTURES];
    for (slot, texture) in desc.textures.iter().zip(&mut textures) {
        *texture = match *slot {
            GfxTextureSlot::White => renderer.images.white,
            // Bevy skips a material whose texture is not loaded yet.
            GfxTextureSlot::Image(id) => renderer.images.get(id, images.get(id)?)?.texture,
            GfxTextureSlot::ImageSampled(id, sampler) => {
                renderer
                    .images
                    .get_sampled(id, images.get(id)?, sampler)?
                    .texture
            }
            GfxTextureSlot::ImageSampledLike(id, like) => {
                renderer
                    .images
                    .get_sampled_like(id, images.get(id)?, images.get(like)?)?
                    .texture
            }
            GfxTextureSlot::Data(id) => match renderer.data.get(id) {
                Some(t) => t,
                None => {
                    renderer.skip_once("a material whose data texture is not kept");
                    return None;
                }
            },
        };
    }
    Some(Resolved {
        pipeline,
        layout,
        attributes,
        textures,
        texture_count: desc.program.samplers.min(MAX_TEXTURES) as u32,
        count,
        indexed,
        present,
    })
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

/// Resets the draw list and applies this frame's asset and data-texture changes to the device
/// stores.
pub(crate) fn prepare(
    mut renderer: NonSendMut<GfxRenderer>,
    mut list: ResMut<DrawList>,
    mut data: ResMut<GfxDataTextures>,
    mut writes: ResMut<GfxTextureWrites>,
    images: Res<Assets<Image>>,
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
    renderer.images.write(&mut writes, &images);
    renderer.data.sync(&mut data);
}

type CameraItem = (
    Entity,
    &'static Camera,
    &'static RenderTarget,
    &'static GlobalTransform,
    &'static VisibleEntities,
    Has<Camera3d>,
    Option<&'static MipBias>,
    Option<&'static GfxFfxGlow>,
    Option<&'static Exposure>,
    Option<&'static AmbientLight>,
    Option<&'static GfxUiLane>,
    Has<bevy::render::view::Hdr>,
    Option<&'static Msaa>,
);

/// Draws every active camera in `order`: the 3D cameras on the primary window into the scene
/// target, with the UI lane ([`crate::ui`]): its `Camera2d` draws into the lane's byte target over
/// the combine of the world view it claims, wherever that view targets, and decodes into the scene
/// target; and every camera on an image into that image's target.
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_views(
    mut ctx: NonSendMut<GfxContext>,
    mut renderer: NonSendMut<GfxRenderer>,
    list: Res<DrawList>,
    meshes: Res<Assets<Mesh>>,
    images: Res<Assets<Image>>,
    clear_color: Option<Res<ClearColor>>,
    global_ambient: Option<Res<GlobalAmbientLight>>,
    time: Res<Time>,
    primary: Query<Entity, With<PrimaryWindow>>,
    cameras: Query<CameraItem>,
    overlays: Res<GfxOverlays>,
    mut last_shape: Local<Vec<String>>,
) {
    let size = ctx.size();
    let default_clear = clear_color.map_or(Color::BLACK, |c| c.0).to_linear();
    let primary = primary.single().ok();
    let remap = clip_remap(renderer.backend);
    let gl = remap != Mat4::IDENTITY;

    let on_window = |target: &RenderTarget| {
        matches!(
            target.normalize(primary),
            Some(NormalizedRenderTarget::Window(w)) if Some(w.entity()) == primary
        )
    };
    // The world views an active lane on the window claims.
    let claimed: Vec<Entity> = cameras
        .iter()
        .filter(|c| c.1.is_active && on_window(c.2))
        .filter_map(|c| c.10.and_then(|lane| lane.backdrop))
        .collect();

    let mut views = Vec::new();
    for (
        entity,
        camera,
        target,
        transform,
        _,
        is_3d,
        mip_bias,
        glow,
        exposure,
        ambient,
        lane,
        hdr,
        msaa,
    ) in &cameras
    {
        if !camera.is_active {
            continue;
        }
        let claimed = is_3d && claimed.contains(&entity);
        let image = match target.normalize(primary) {
            Some(NormalizedRenderTarget::Image(t)) if !claimed => Some(t.handle.id()),
            _ => None,
        };
        if !on_window(target) && !claimed && image.is_none() {
            renderer.skip_once("a camera on a texture view or another window");
            continue;
        }
        let ui_lane = lane.filter(|_| !is_3d && image.is_none()).map(|l| l.gamma);
        let overlay =
            !is_3d && ui_lane.is_none() && image.is_none() && overlays.0.contains_key(&entity);
        if !is_3d && ui_lane.is_none() && image.is_none() && !overlay {
            renderer.skip_once("a 2D camera outside the UI lane");
            continue;
        }
        let (dest, target_size, top_down) = match image {
            Some(id) => {
                let Some(img) = images.get(id) else {
                    continue;
                };
                let d = img.texture_descriptor.size;
                (Dest::Image(id), UVec2::new(d.width, d.height), gl)
            }
            None => (Dest::Scene, size, false),
        };
        let output_blend = match &camera.output_mode {
            CameraOutputMode::Write {
                blend_state: Some(b),
                ..
            } => Blend::of_state(b).unwrap_or_else(|| {
                renderer.skip_once(
                    "a window camera's output blend other than bevy's (as premultiplied)",
                );
                Blend::Premultiplied
            }),
            _ => Blend::Replace,
        };
        let output_clear = match &camera.output_mode {
            CameraOutputMode::Write {
                blend_state,
                clear_color,
            } => {
                if blend_state.is_some() && image.is_some() {
                    renderer.skip_once("an image camera's output blend (blitted as a replace)");
                }
                match clear_color {
                    ClearColorConfig::Default => Some(default_clear),
                    ClearColorConfig::Custom(c) => Some(c.to_linear()),
                    ClearColorConfig::None => None,
                }
            }
            CameraOutputMode::Skip => {
                if image.is_some() {
                    renderer.skip_once("an image camera whose output mode is Skip");
                    continue;
                }
                None
            }
        };
        let Some(viewport) = camera.physical_viewport_rect() else {
            continue;
        };
        let view_from_world = transform.to_matrix().inverse();
        // An image target on GL is drawn upside down so its rows run top-down (`ImageTarget`);
        // the flip turns the winding over too.
        let flip = if top_down {
            Mat4::from_scale(Vec3::new(1.0, -1.0, 1.0))
        } else {
            Mat4::IDENTITY
        };
        let clip_from_view = flip * remap * camera.clip_from_view();
        views.push(View {
            order: camera.order,
            viewport,
            clear: match camera.clear_color {
                ClearColorConfig::Default => Some(default_clear),
                ClearColorConfig::Custom(c) => Some(c.to_linear()),
                ClearColorConfig::None => None,
            },
            clip_from_world: clip_from_view * view_from_world,
            clip_from_view,
            view_from_world,
            mip_bias: mip_bias.map_or(0.0, |b| b.0),
            gl_remap: gl,
            time: time.elapsed_secs_wrapped(),
            target_height: target_size.y,
            invert_culling: camera.invert_culling != top_down,
            glow: glow.copied(),
            ui_lane,
            claimed,
            ambient: ambient_row(
                ambient.map_or_else(
                    || {
                        let g = global_ambient.as_deref().cloned().unwrap_or_default();
                        (g.color, g.brightness)
                    },
                    |a| (a.color, a.brightness),
                ),
                exposure.copied().unwrap_or_default(),
            ),
            entity,
            dest,
            top_down,
            target_size,
            is_3d,
            hdr,
            output_clear,
            has_viewport: camera.viewport.is_some(),
            samples: msaa.map_or(1, |m| m.samples()),
            overlay,
            output_blend,
        });
    }
    views.sort_by_key(|v| v.order);
    // The frame's view set, logged whenever it changes.
    let shape: Vec<String> = views
        .iter()
        .map(|v| {
            let kind = match (v.ui_lane, v.claimed, v.dest, v.is_3d) {
                _ if v.overlay => "overlay",
                (Some(_), ..) => "UI lane",
                (None, true, ..) => "world, claimed",
                (None, false, Dest::Image(_), true) => "3D on an image",
                (None, false, Dest::Image(_), false) => "2D on an image",
                (None, false, Dest::Scene, _) => "3D",
            };
            let glow = if v.glow.is_some() { ", glow" } else { "" };
            let msaa = if v.is_3d && v.samples > 1 {
                format!(", {}x", v.samples)
            } else {
                String::new()
            };
            let visible = cameras.get(v.entity).map_or(0, |c| {
                c.4.iter(std::any::TypeId::of::<Mesh3d>()).count()
                    + c.4.iter(std::any::TypeId::of::<Mesh2d>()).count()
            });
            let early = list.early.iter().filter(|(c, _)| *c == v.entity).count();
            format!(
                "{} {kind}{glow}{msaa} {:?}, {} visible, {} early",
                v.order,
                v.viewport.size(),
                visible / 64 * 64,
                early / 64 * 64
            )
        })
        .collect();
    if *last_shape != shape {
        info!("gfx: views [{}]", shape.join("; "));
        *last_shape = shape;
    }

    let renderer = &mut *renderer;
    let Some(framebuffer) = renderer.target(size).map(|t| t.framebuffers[0]) else {
        return;
    };
    renderer.images.begin_frame();
    renderer.overlay.begin_frame();
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

    let ui_framebuffer = if views.iter().any(|v| v.ui_lane.is_some()) {
        renderer.ui_target(size).map(|t| t.framebuffer)
    } else {
        None
    };
    let ui_class = TargetClass {
        format: UI_FORMAT,
        depth: false,
        samples: 1,
    };

    renderer.ring.data.clear();
    let mut cmds = Vec::new();
    // Whether a claimed combine has grounded the UI lane this frame.
    let mut grounded = false;
    for view in &views {
        let Ok((_, _, _, _, visible, ..)) = cameras.get(view.entity) else {
            continue;
        };
        let view_offset = renderer.ring.push(&view_block(view));
        if view.overlay {
            let Some(frame) = overlays.0.get(&view.entity) else {
                continue;
            };
            if renderer.overlay.target(size).is_none() {
                continue;
            }
            let GfxRenderer {
                overlay,
                images: gpu_images,
                ..
            } = &mut *renderer;
            let draws = overlay.stage(frame, size.y, |id| {
                Some(gpu_images.get(id, images.get(id)?)?.texture)
            });
            // Nothing drawn composites nothing: the overlay's clear is transparent under a
            // blending output (egui's camera). Anything else would still clear and composite.
            if draws.is_empty()
                && view.output_blend != Blend::Replace
                && view.output_clear.is_none()
                && view.clear.is_none_or(|c| c.alpha == 0.0)
            {
                continue;
            }
            let offset = renderer.ring.push(&frame.transform);
            let composite = renderer.ring.push(&FfxPost::composite_block(
                view.output_blend,
                view.output_clear,
            ));
            cmds.push(Cmd::Overlay {
                draws: (draws.start, draws.end),
                offset,
                clear: view.clear,
                composite,
            });
            continue;
        }
        if let Some(gamma) = view.ui_lane {
            let Some(ui_framebuffer) = ui_framebuffer else {
                continue;
            };
            cmds.push(Cmd::UiBegin {
                viewport: view.viewport,
                clear: if grounded { None } else { view.clear },
            });
            cmds.extend(mesh2d_cmds(
                renderer,
                &mut ctx.shaders,
                (ui_framebuffer, ui_class),
                view,
                view_offset,
                visible,
                &list,
                &meshes,
                &images,
            ));
            // The late draws, each projection's view block pushed once.
            let mut late_views: HashMap<[u32; 16], u32> = HashMap::new();
            for (camera, late) in &list.late {
                if *camera != view.entity {
                    continue;
                }
                let desc = list.desc(late.desc);
                let Some(r) = resolve(
                    renderer,
                    &mut ctx.shaders,
                    (ui_framebuffer, ui_class),
                    view,
                    late.mesh,
                    desc,
                    &meshes,
                    &images,
                ) else {
                    continue;
                };
                if !r.indexed {
                    continue;
                }
                let key = late.clip_from_world.to_cols_array().map(f32::to_bits);
                let late_view = *late_views.entry(key).or_insert_with(|| {
                    let mut v = view.clone();
                    v.clip_from_world = remap * late.clip_from_world;
                    renderer.ring.push(&view_block(&v))
                });
                let item = DrawItem {
                    mesh: late.mesh,
                    world_from_local: Mat4::IDENTITY,
                    center: Vec3::ZERO,
                    tag: 0,
                    desc: late.desc,
                };
                let block = draw_block(&item, desc, r.present);
                let draw_offset = renderer.ring.push(&block);
                cmds.push(r.cmd(
                    late_view,
                    draw_offset,
                    (block.len() * 4) as u32,
                    Some(&late.indices),
                ));
            }
            let offset = renderer.ring.push(&FfxPost::decode_block(gamma));
            cmds.push(Cmd::UiDecode {
                offset,
                viewport: view.viewport,
            });
            continue;
        }
        // A 2D view keeps to its target's own samples: no 2D camera here multisamples.
        let samples = if view.is_3d { view.samples } else { 1 };
        if samples > 1 && view.clear.is_none() {
            renderer.skip_once("an MSAA camera's writeback of an unclear target (drawn over none)");
        }
        let (target, msaa) = match view.dest {
            Dest::Scene => {
                let Some(scene) = renderer.target.as_mut() else {
                    continue;
                };
                let (target, msaa) = msaa_or_main(scene, samples);
                cmds.push(Cmd::View {
                    viewport: view.viewport,
                    clear: view.clear,
                    msaa,
                });
                (target, msaa)
            }
            Dest::Image(id) => {
                let Some(image) = images.get(id) else {
                    continue;
                };
                let Some(t) = renderer.images.target(id, image, view.hdr, view.is_3d) else {
                    continue;
                };
                let (target, msaa) = msaa_or_main(&mut t.main, samples);
                cmds.push(Cmd::ImageView {
                    image: id,
                    rect: gfx_rect(view.viewport, view.target_size.y, view.top_down),
                    clear: view.clear,
                    msaa,
                });
                (target, msaa)
            }
        };

        if !view.is_3d {
            cmds.extend(mesh2d_cmds(
                renderer,
                &mut ctx.shaders,
                target,
                view,
                view_offset,
                visible,
                &list,
                &meshes,
                &images,
            ));
        } else {
            view_3d_cmds(
                renderer,
                &mut ctx.shaders,
                target,
                view,
                view_offset,
                visible,
                &list,
                &meshes,
                &images,
                &mut cmds,
            );
            if msaa {
                cmds.push(Cmd::Resolve { dest: view.dest });
            }
            if let (Some(glow), Some(post)) = (&view.glow, &renderer.post) {
                let into_ui = view.claimed && ui_framebuffer.is_some();
                // The scene target's views keep the window's size for their quarter targets.
                let (full, quarter_of) = match view.dest {
                    Dest::Scene => (size.max(UVec2::ONE), size.max(UVec2::ONE)),
                    Dest::Image(_) => (
                        view.target_size.max(UVec2::ONE),
                        view.viewport.size().max(UVec2::ONE),
                    ),
                };
                let blocks = post.blocks(glow, full, quarter_of, into_ui, view.top_down);
                let offsets = blocks.map(|b| renderer.ring.push(&b));
                let wave = glow.wave_lut.and_then(|id| {
                    let image = images.get(id)?;
                    Some(renderer.images.get(id, image)?.texture)
                });
                cmds.push(Cmd::Glow {
                    dest: view.dest,
                    viewport: quarter_of,
                    offsets,
                    wave,
                    into_ui,
                });
                grounded |= into_ui;
            }
        }
        if let Dest::Image(image) = view.dest {
            cmds.push(Cmd::Blit {
                image,
                scissor: view
                    .has_viewport
                    .then(|| gfx_rect(view.viewport, view.target_size.y, view.top_down)),
                clear: view.output_clear,
            });
        }
    }

    if !renderer.ring.upload() {
        error!("gfx: the uniform ring could not be uploaded");
        return;
    }
    let ring = renderer.ring.buffer;
    let overlay_ready = cmds.iter().any(|c| matches!(c, Cmd::Overlay { .. }))
        && renderer.overlay.upload(&mut ctx.shaders);
    // A target re-made while recording took its framebuffers with it.
    if let Some(post) = &mut renderer.post {
        for fb in renderer.images.take_dropped() {
            post.forget_framebuffer(fb);
        }
    }
    let GfxRenderer {
        target,
        post,
        ui,
        images,
        overlay,
        ..
    } = renderer;
    let Some(target) = target else {
        return;
    };
    execute(
        device,
        target,
        ui.as_ref(),
        images,
        post.as_mut(),
        overlay_ready.then_some(overlay),
        &mut ctx.shaders,
        ring,
        &cmds,
    );
}

/// A 2D view's `Mesh2d` draws in `Transparent2d` order: the mesh's world z ascending, stable over
/// the visible order.
#[allow(clippy::too_many_arguments)]
fn mesh2d_cmds(
    renderer: &mut GfxRenderer,
    shaders: &mut ShaderLibrary,
    target: (ffi::GfxFramebuffer, TargetClass),
    view: &View,
    view_offset: u32,
    visible: &VisibleEntities,
    list: &DrawList,
    meshes: &Assets<Mesh>,
    images: &Assets<Image>,
) -> Vec<Cmd> {
    let mut sorted = Vec::new();
    for entity in visible.iter(std::any::TypeId::of::<Mesh2d>()) {
        let Some(item) = list.get(*entity) else {
            continue;
        };
        let desc = list.desc(item.desc);
        let Some(r) = resolve(
            renderer, shaders, target, view, item.mesh, desc, meshes, images,
        ) else {
            continue;
        };
        let block = draw_block(item, desc, r.present);
        let draw_offset = renderer.ring.push(&block);
        let cmd = r.cmd(view_offset, draw_offset, (block.len() * 4) as u32, None);
        sorted.push((item.center.z, cmd));
    }
    sorted.sort_by(|a, b| a.0.total_cmp(&b.0));
    sorted.into_iter().map(|(_, c)| c).collect()
}

/// A 3D view's draws: the early draws, then opaque and mask, then transparent.
#[allow(clippy::too_many_arguments)]
fn view_3d_cmds(
    renderer: &mut GfxRenderer,
    shaders: &mut ShaderLibrary,
    target: (ffi::GfxFramebuffer, TargetClass),
    view: &View,
    view_offset: u32,
    visible: &VisibleEntities,
    list: &DrawList,
    meshes: &Assets<Mesh>,
    images: &Assets<Image>,
    cmds: &mut Vec<Cmd>,
) {
    // The early draws, in the order pushed; draws sharing a description and world matrix
    // share one block.
    let mut early_blocks: HashMap<(u32, [u32; 16]), (u32, u32)> = HashMap::new();
    for (camera, early) in &list.early {
        if *camera != view.entity {
            continue;
        }
        let desc = list.desc(early.desc);
        let Some(r) = resolve(
            renderer, shaders, target, view, early.mesh, desc, meshes, images,
        ) else {
            continue;
        };
        if !r.indexed {
            continue;
        }
        let matrix = early.world_from_local.to_cols_array().map(f32::to_bits);
        let (draw_offset, draw_size) =
            *early_blocks.entry((early.desc, matrix)).or_insert_with(|| {
                let item = DrawItem {
                    mesh: early.mesh,
                    world_from_local: early.world_from_local,
                    center: Vec3::ZERO,
                    tag: 0,
                    desc: early.desc,
                };
                let block = draw_block(&item, desc, r.present);
                (renderer.ring.push(&block), (block.len() * 4) as u32)
            });
        cmds.push(r.cmd(view_offset, draw_offset, draw_size, Some(&early.indices)));
    }

    // Opaque and mask first (sorted by pipeline, texture, mesh, as bins batch), then
    // transparent back to front.
    let (mut opaque, mut transparent) = (Vec::new(), Vec::new());
    for entity in visible.iter(std::any::TypeId::of::<Mesh3d>()) {
        let Some(item) = list.get(*entity) else {
            continue;
        };
        let desc = list.desc(item.desc);
        let Some(r) = resolve(
            renderer, shaders, target, view, item.mesh, desc, meshes, images,
        ) else {
            continue;
        };
        let block = draw_block(item, desc, r.present);
        let draw_offset = renderer.ring.push(&block);
        let cmd = r.cmd(view_offset, draw_offset, (block.len() * 4) as u32, None);
        if desc.alpha.is_transparent() {
            // bevy_pbr's `Transparent3d` distance: the view z of the AABB centre plus the
            // material's depth bias, sorted ascending.
            let z = view.view_from_world.transform_point3(item.center).z + desc.state.sort_bias;
            transparent.push((z, cmd));
        } else {
            let rank = match desc.alpha {
                GfxAlpha::Mask(_) => 1u8,
                _ => 0,
            };
            opaque.push((
                (rank, r.pipeline as usize, r.textures[0] as usize, item.mesh),
                cmd,
            ));
        }
    }
    // The placed transparent draws, after the entities, as a lane's queue system runs after
    // bevy_pbr's; the sort is stable.
    for (camera, sorted) in &list.sorted {
        if *camera != view.entity {
            continue;
        }
        let desc = list.desc(sorted.desc);
        let Some(r) = resolve(
            renderer,
            shaders,
            target,
            view,
            sorted.mesh,
            desc,
            meshes,
            images,
        ) else {
            continue;
        };
        if !r.indexed {
            continue;
        }
        let item = DrawItem {
            mesh: sorted.mesh,
            world_from_local: sorted.world_from_local,
            center: sorted.anchor,
            tag: 0,
            desc: sorted.desc,
        };
        let block = draw_block(&item, desc, r.present);
        let draw_offset = renderer.ring.push(&block);
        let cmd = r.cmd(
            view_offset,
            draw_offset,
            (block.len() * 4) as u32,
            Some(&sorted.indices),
        );
        let z = view.view_from_world.transform_point3(sorted.anchor).z + sorted.bias;
        transparent.push((z, cmd));
    }
    opaque.sort_by_key(|(key, _)| *key);
    cmds.extend(opaque.into_iter().map(|(_, c)| c));
    // Farthest first: the most negative view-space z.
    transparent.sort_by(|a, b| a.0.total_cmp(&b.0));
    cmds.extend(transparent.into_iter().map(|(_, c)| c));
}

#[allow(clippy::too_many_arguments)]
fn execute(
    device: GfxDevice,
    target: &mut SceneTarget,
    ui: Option<&UiTarget>,
    images: &mut GpuImages,
    mut post: Option<&mut FfxPost>,
    mut overlay: Option<&mut OverlayPass>,
    shaders: &mut ShaderLibrary,
    ring: GfxBuffer,
    cmds: &[Cmd],
) {
    let mut bound = ptr::null_mut();
    for cmd in cmds {
        match *cmd {
            Cmd::Glow {
                dest,
                viewport,
                offsets,
                wave,
                into_ui,
            } => {
                let into = ui.filter(|_| into_ui).map(|u| u.framebuffer);
                if let Some(fb) = into {
                    // Bound first: a clear lands in the bound pass on vk.
                    // SAFETY: the live device and UI target, on the device's thread.
                    unsafe {
                        ffi::gfx_dll_bind_framebuffer(device, fb);
                        ffi::gfx_dll_clear_color(device, fb, 0, &gfx_color(LinearRgba::NONE));
                    }
                }
                if let Some(post) = post.as_deref_mut() {
                    let scene = match dest {
                        Dest::Scene => Some(&mut *target),
                        Dest::Image(id) => images.image_target(id).map(|t| &mut t.main),
                    };
                    if let Some(scene) = scene {
                        post.run(shaders, scene, viewport, ring, offsets, wave, into);
                    }
                }
                bound = ptr::null_mut();
            }
            Cmd::ImageView {
                image,
                rect,
                clear,
                msaa,
            } => {
                let Some(t) = images.image_target(image) else {
                    continue;
                };
                let Some(fb) = bound_framebuffer(&t.main, msaa) else {
                    continue;
                };
                let full = t.main.size;
                let depth = !t.main.depth.is_null();
                let (x, y, size) = rect;
                // SAFETY: the live device and image target, on the device's thread.
                unsafe {
                    ffi::gfx_dll_bind_framebuffer(device, fb);
                    // A wgpu clear op clears the whole attachment, whatever the viewport.
                    ffi::gfx_dll_set_viewport(device, 0, 0, full.x, full.y, 0.0, 1.0);
                    ffi::gfx_dll_set_scissor(device, 0, 0, full.x, full.y);
                    if let Some(c) = clear {
                        ffi::gfx_dll_clear_color(device, fb, 0, &gfx_color(c));
                    }
                    if depth {
                        ffi::gfx_dll_clear_depth_stencil(device, fb, 0.0, 0);
                    }
                    ffi::gfx_dll_set_viewport(device, x, y, size.x, size.y, 0.0, 1.0);
                    ffi::gfx_dll_set_scissor(device, x, y, size.x, size.y);
                }
                bound = ptr::null_mut();
            }
            Cmd::Blit {
                image,
                scissor,
                clear,
            } => {
                let Some(t) = images.image_target(image) else {
                    continue;
                };
                let clear = if t.written { None } else { clear };
                t.written = true;
                let (source, fb, size) = (t.main.color(), t.output_framebuffer, t.size);
                if let Some(post) = post.as_deref_mut() {
                    post.blit(shaders, source, fb, size.max(UVec2::ONE), scissor, clear);
                }
                bound = ptr::null_mut();
            }
            Cmd::UiBegin { viewport, clear } => {
                let Some(ui) = ui else {
                    continue;
                };
                let size = viewport.size();
                // SAFETY: the live device and UI target, on the device's thread.
                unsafe {
                    ffi::gfx_dll_bind_framebuffer(device, ui.framebuffer);
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
                        ffi::gfx_dll_clear_color(device, ui.framebuffer, 0, &gfx_color(c));
                    }
                }
                bound = ptr::null_mut();
            }
            Cmd::Overlay {
                draws,
                offset,
                clear,
                composite,
            } => {
                let Some(overlay) = overlay.as_deref_mut() else {
                    continue;
                };
                let color = overlay.draw(shaders, ring, offset, clear, draws.0..draws.1);
                if let (Some(color), Some(post)) = (color, post.as_deref_mut()) {
                    post.composite(shaders, target, color, ring, composite);
                }
                bound = ptr::null_mut();
            }
            Cmd::UiDecode { offset, viewport } => {
                if let (Some(ui), Some(post)) = (ui, post.as_deref_mut()) {
                    post.decode(shaders, target, ui.color, ring, offset, viewport);
                }
                bound = ptr::null_mut();
            }
            Cmd::Resolve { dest } => {
                let scene = match dest {
                    Dest::Scene => Some(&*target),
                    Dest::Image(id) => images.image_target(id).map(|t| &t.main),
                };
                if let Some(scene) = scene {
                    if !scene.resolve() {
                        error!("gfx: a multisampled view's resolve failed");
                    }
                }
                // vk resolves between render passes: the pass resumed after it binds nothing.
                bound = ptr::null_mut();
            }
            Cmd::View {
                viewport,
                clear,
                msaa,
            } => {
                let size = viewport.size();
                let Some(framebuffer) = bound_framebuffer(target, msaa) else {
                    continue;
                };
                // SAFETY: the live device and scene target, on the device's thread.
                unsafe {
                    ffi::gfx_dll_bind_framebuffer(device, framebuffer);
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
                mut textures,
                texture_count,
                view_offset,
                draw_offset,
                draw_size,
                count,
                first,
                indexed,
            } => {
                // SAFETY: every handle is live and made on `device`; the ring holds both blocks.
                unsafe {
                    if pipeline != bound {
                        ffi::gfx_dll_bind_pipeline(device, pipeline);
                        bound = pipeline;
                    }
                    ffi::gfx_dll_bind_attributes_state(device, attributes, layout);
                    ffi::gfx_dll_bind_constant(device, 0, ring, VIEW_BLOCK as u32, view_offset);
                    ffi::gfx_dll_bind_constant(device, 1, ring, draw_size, draw_offset);
                    ffi::gfx_dll_bind_samplers(device, 0, texture_count, textures.as_mut_ptr());
                    if indexed {
                        ffi::gfx_dll_draw_indexed(device, count, first);
                    } else {
                        ffi::gfx_dll_draw(device, count, first);
                    }
                }
            }
        }
    }
}

/// The framebuffer and class a view of `samples` draws into on `target`: its multisampled target
/// (made on first use) when `samples` > 1 and gfx made it, else its current colour; and whether it
/// is the multisampled one.
fn msaa_or_main(
    target: &mut SceneTarget,
    samples: u32,
) -> ((ffi::GfxFramebuffer, TargetClass), bool) {
    if samples > 1 {
        if let (Some(m), _) = target.ensure_msaa(samples) {
            let fb = m.framebuffer;
            return ((fb, target.msaa_class(samples)), true);
        }
    }
    ((target.framebuffers[0], target.class()), false)
}

/// The framebuffer a view starting on `target` binds: its multisampled one when `msaa`.
fn bound_framebuffer(target: &SceneTarget, msaa: bool) -> Option<ffi::GfxFramebuffer> {
    if msaa {
        target.msaa.as_ref().map(|m| m.framebuffer)
    } else {
        Some(target.framebuffer())
    }
}

/// The view block's `ambient` row: a camera's own `AmbientLight` over the global one, as
/// bevy_pbr's `prepare_lights` takes it, and the camera's exposure.
fn ambient_row((color, brightness): (Color, f32), exposure: Exposure) -> [f32; 4] {
    let c = LinearRgba::from(color);
    [
        c.red * brightness,
        c.green * brightness,
        c.blue * brightness,
        exposure.exposure(),
    ]
}

/// The view block (see [`VIEW_BLOCK`]).
fn view_block(view: &View) -> [f32; VIEW_BLOCK / 4] {
    let mut b = [0.0f32; VIEW_BLOCK / 4];
    b[..16].copy_from_slice(&view.clip_from_world.to_cols_array());
    b[16..32].copy_from_slice(&view.view_from_world.to_cols_array());
    b[32..48].copy_from_slice(&view.clip_from_view.to_cols_array());
    b[48..52].copy_from_slice(&view.view_from_world.inverse().w_axis.to_array());
    let (min, size) = (view.viewport.min.as_vec2(), view.viewport.size().as_vec2());
    b[52..56].copy_from_slice(&[min.x, min.y, size.x, size.y]);
    b[56] = view.mip_bias;
    b[57] = match (view.gl_remap, view.top_down) {
        (false, _) => 0.0,
        (true, false) => 1.0,
        (true, true) => 2.0,
    };
    b[58] = view.target_height as f32;
    b[59] = view.time;
    b[60..64].copy_from_slice(&view.ambient);
    b
}

/// One draw's block: the world matrix, the tag row and the program's parameter rows.
fn draw_block(item: &DrawItem, desc: &GfxMaterialDesc, present: u32) -> Vec<f32> {
    let params = desc.program.params.min(desc.params.len());
    let mut b = Vec::with_capacity(DRAW_HEAD / 4 + 4 * params);
    b.extend_from_slice(&item.world_from_local.to_cols_array());
    b.extend_from_slice(&[f32::from_bits(item.tag), f32::from_bits(present), 0.0, 0.0]);
    b.extend(desc.params[..params].iter().flatten());
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
        present.draw(target.color(), size);
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
    fn the_ambient_row_is_bevys_lights_ambient_color_and_exposure() {
        // bevy_pbr's defaults: white at 80, `Exposure::BLENDER` (ev100 9.7).
        let row = ambient_row((Color::WHITE, 80.0), Exposure::default());
        assert_eq!(&row[..3], &[80.0; 3]);
        assert!((row[3] - 2f32.powf(-9.7) / 1.2).abs() < 1e-9);
    }

    #[test]
    fn the_view_block_ends_with_the_ambient_row() {
        let view = View {
            order: 0,
            viewport: URect::new(0, 0, 4, 4),
            clear: None,
            clip_from_world: Mat4::IDENTITY,
            clip_from_view: Mat4::IDENTITY,
            view_from_world: Mat4::IDENTITY,
            mip_bias: 0.0,
            gl_remap: false,
            time: 0.0,
            target_height: 4,
            invert_culling: false,
            glow: None,
            ui_lane: None,
            claimed: false,
            ambient: [1.0, 2.0, 3.0, 4.0],
            entity: Entity::PLACEHOLDER,
            dest: Dest::Scene,
            top_down: false,
            target_size: UVec2::splat(4),
            is_3d: true,
            hdr: true,
            output_clear: None,
            has_viewport: false,
            samples: 1,
            overlay: false,
            output_blend: Blend::Replace,
        };
        assert_eq!(view_block(&view)[60..], [1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn a_rect_counts_from_the_bottom_unless_the_target_runs_top_down() {
        // A 10x20 cell 5 px from the top of a 100 px target.
        let cell = URect::new(3, 5, 13, 25);
        assert_eq!(gfx_rect(cell, 100, false), (3, 75, UVec2::new(10, 20)));
        assert_eq!(gfx_rect(cell, 100, true), (3, 5, UVec2::new(10, 20)));
    }

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
        assert_eq!(ring.push(&[2.0; 32]), 256);
        assert_eq!(ring.push(&[3.0; 32]), 512);
        assert_eq!(ring.data.len(), 512 + 128);
        assert_eq!(
            f32::from_le_bytes(ring.data[256..260].try_into().unwrap()),
            2.0
        );
    }
}
