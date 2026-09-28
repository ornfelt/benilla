//! The render-graph instruments' gfx half. On wgpu, benilla's `WOW_DEPTH` probe copies a view's
//! depth texture in a graph node between `MainOpaquePass` and `MainTransmissivePass` (or after
//! `MainTransparentPass`) and maps it after submit, and `WOW_PHASE` reads the render world's
//! phase lists after `PhaseSort`. Here [`GfxDepthProbe`] asks the draw of one camera for the same
//! copy at the same point of its phases, into a depth texture of its own, read back the next
//! frame once the device has run it; [`GfxPhaseRecord`] keeps each 3D view's phase lists as the
//! draw built them.

use bevy::prelude::*;

use crate::draw::GfxRenderer;
use crate::ffi::{self, GfxDevice, GfxDeviceBackend, GfxTexture};
use crate::target::{self, DEPTH_FORMAT};

/// A depth copy asked of the draw ([`GfxDepthProbe::request`]).
#[derive(Debug, Clone, Copy)]
pub struct GfxDepthRequest {
    /// The 3D camera whose depth is copied.
    pub camera: Entity,
    /// After its transparent phase (`WOW_DEPTH_AFTER`), else after its opaque and mask phases.
    pub after_transparent: bool,
    /// The caller's tag, handed back with the copy.
    pub tag: u32,
}

/// A depth copy read back: the camera's whole depth target, rows top-down as bevy's pixels run,
/// the reverse-Z values as stored.
pub struct GfxDepthReadback {
    pub tag: u32,
    pub size: UVec2,
    pub depth: Vec<f32>,
}

/// One camera's depth, copied at a point of its phases and read back the next frame.
#[derive(Resource, Default)]
pub struct GfxDepthProbe {
    /// This frame's copy, taken by the draw; nothing is copied for a camera that does not draw,
    /// or draws multisampled.
    pub request: Option<GfxDepthRequest>,
    /// The last copy read back, for the probe to take.
    pub result: Option<GfxDepthReadback>,
}

/// The copy target: a depth texture the size of the probed camera's.
pub(crate) struct DepthCopy {
    device: GfxDevice,
    pub(crate) texture: GfxTexture,
    pub(crate) size: UVec2,
    /// The tag of the copy the device has recorded and not yet been read back.
    pub(crate) pending: Option<u32>,
}

impl DepthCopy {
    pub(crate) fn new(device: GfxDevice, size: UVec2) -> Option<Self> {
        let texture = target::render_texture(device, DEPTH_FORMAT, size)?;
        Some(Self {
            device,
            texture,
            size,
            pending: None,
        })
    }
}

impl Drop for DepthCopy {
    fn drop(&mut self) {
        // SAFETY: made on `self.device` and belonging to this copy alone.
        unsafe { ffi::gfx_dll_delete_texture(self.device, self.texture) };
    }
}

/// `GfxRenderSystems::Prepare`: last frame's depth copy, read back into [`GfxDepthProbe`].
pub(crate) fn collect_depth(
    mut renderer: NonSendMut<GfxRenderer>,
    mut probe: ResMut<GfxDepthProbe>,
) {
    let backend = renderer.backend();
    let Some(copy) = renderer.depth_copy.as_mut() else {
        return;
    };
    let Some(tag) = copy.pending.take() else {
        return;
    };
    let (w, h) = (copy.size.x, copy.size.y);
    let mut depth = vec![0f32; (w * h) as usize];
    // SAFETY: the live device and copy texture, on the device's thread; `depth` holds the rect.
    let ok = unsafe {
        ffi::gfx_dll_read_texture(
            copy.device,
            copy.texture,
            0,
            0,
            w,
            h,
            (depth.len() * 4) as u32,
            depth.as_mut_ptr().cast(),
        )
    };
    if !ok {
        error!("gfx: the depth copy could not be read back ({backend:?} reads no depth on gles3)");
        return;
    }
    // GL draws the scene bottom-up: its first row is the image's bottom.
    if matches!(
        backend,
        GfxDeviceBackend::Gl3 | GfxDeviceBackend::Gl4 | GfxDeviceBackend::Gles3
    ) {
        let rows: Vec<f32> = depth
            .chunks_exact(w as usize)
            .rev()
            .flatten()
            .copied()
            .collect();
        depth = rows;
    }
    probe.result = Some(GfxDepthReadback {
        tag,
        size: copy.size,
        depth,
    });
}

/// The depth copy target at `size`, re-made when the size changed.
pub(crate) fn depth_copy(renderer: &mut GfxRenderer, size: UVec2) -> Option<GfxTexture> {
    let size = size.max(UVec2::ONE);
    if renderer.depth_copy.as_ref().is_none_or(|c| c.size != size) {
        renderer.depth_copy = None;
        renderer.depth_copy = DepthCopy::new(renderer.device(), size);
        if renderer.depth_copy.is_none() {
            error!("gfx: the depth copy target could not be made");
        }
    }
    renderer.depth_copy.as_ref().map(|c| c.texture)
}

/// One 3D view's phases as the draw built them: bevy_core_pipeline's `Opaque3d`, `AlphaMask3d`
/// and `Transparent3d` lists, each in draw order.
#[derive(Debug, Clone)]
pub struct GfxViewPhases {
    pub camera: Entity,
    /// The early draws (the static pass before the opaque phase), which no entity owns.
    pub early: usize,
    pub opaque: Vec<Entity>,
    pub mask: Vec<Entity>,
    /// Back to front, each with its sort distance (view z plus bias); a system's draw
    /// ([`crate::draw::SortedDraw`]) under the entity it stands for.
    pub transparent: Vec<(Entity, f32)>,
}

/// The frame's 3D view phases, kept while `enabled`: rebuilt by each draw.
#[derive(Resource, Default)]
pub struct GfxPhaseRecord {
    pub enabled: bool,
    pub views: Vec<GfxViewPhases>,
}
