//! The render-graph instruments' gfx half. On wgpu, benilla's `WOW_DEPTH` probe copies a view's
//! depth texture in a graph node between `MainOpaquePass` and `MainTransmissivePass` (or after
//! `MainTransparentPass`) and maps it after submit, and `WOW_PHASE` reads the render world's
//! phase lists after `PhaseSort`. Here [`GfxDepthProbe`] asks the draw of one camera for the same
//! copy at the same point of its phases, into a depth texture of its own, read back the next
//! frame once the device has run it; [`GfxPhaseRecord`] keeps each 3D view's phase lists as the
//! draw built them.
//!
//! gles3 reads colour bytes only, so there the copy is read through [`crate::post::FfxPost::
//! pack_depth`]: each depth float's bits as four bytes of an RGBA8 target.

use std::ptr;

use bevy::prelude::*;

use crate::context::GfxContext;
use crate::draw::GfxRenderer;
use crate::ffi::{self, GfxDevice, GfxDeviceBackend, GfxFormat, GfxFramebuffer, GfxTexture};
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
    /// gles3's RGBA8 target the copy's bits are packed into for the read, made on first use.
    packed: Option<(GfxTexture, GfxFramebuffer)>,
}

impl DepthCopy {
    pub(crate) fn new(device: GfxDevice, size: UVec2) -> Option<Self> {
        let texture = target::render_texture(device, DEPTH_FORMAT, size)?;
        Some(Self {
            device,
            texture,
            size,
            pending: None,
            packed: None,
        })
    }

    /// The packed target, made on first use.
    fn packed(&mut self) -> Option<(GfxTexture, GfxFramebuffer)> {
        if self.packed.is_none() {
            let color = target::render_texture(self.device, GfxFormat::R8G8B8A8Unorm, self.size)?;
            let Some(fb) = target::framebuffer(self.device, color, ptr::null_mut(), self.size)
            else {
                // SAFETY: made on `self.device` just above, owned by nothing else.
                unsafe { ffi::gfx_dll_delete_texture(self.device, color) };
                return None;
            };
            self.packed = Some((color, fb));
        }
        self.packed
    }
}

impl Drop for DepthCopy {
    fn drop(&mut self) {
        // SAFETY: made on `self.device` and belonging to this copy alone.
        unsafe {
            if let Some((color, fb)) = self.packed.take() {
                ffi::gfx_dll_delete_framebuffer(self.device, fb);
                ffi::gfx_dll_delete_texture(self.device, color);
            }
            ffi::gfx_dll_delete_texture(self.device, self.texture);
        }
    }
}

/// `GfxRenderSystems::Prepare`: last frame's depth copy, read back into [`GfxDepthProbe`].
pub(crate) fn collect_depth(
    mut ctx: NonSendMut<GfxContext>,
    mut renderer: NonSendMut<GfxRenderer>,
    mut probe: ResMut<GfxDepthProbe>,
) {
    let backend = renderer.backend();
    let bottom_up = renderer.rows_bottom_up();
    let GfxRenderer {
        depth_copy, post, ..
    } = &mut *renderer;
    let Some(copy) = depth_copy.as_mut() else {
        return;
    };
    let Some(tag) = copy.pending.take() else {
        return;
    };
    let (w, h) = (copy.size.x, copy.size.y);
    let mut depth = vec![0f32; (w * h) as usize];
    let ok = if backend == GfxDeviceBackend::Gles3 {
        read_packed(&mut ctx, post.as_mut(), copy, &mut depth)
    } else {
        // SAFETY: the live device and copy texture, on the device's thread; `depth` holds the
        // rect.
        unsafe {
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
        }
    };
    if !ok {
        error!("gfx: the depth copy could not be read back ({backend:?})");
        return;
    }
    // A GL without the upper-left clip origin draws the scene bottom-up: its first row is the
    // image's bottom.
    if bottom_up {
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

/// gles3's read of `copy` into `depth`: its bits packed into RGBA8 ([`DepthCopy::packed`]), read
/// as bytes and put back together.
fn read_packed(
    ctx: &mut GfxContext,
    post: Option<&mut crate::post::FfxPost>,
    copy: &mut DepthCopy,
    depth: &mut [f32],
) -> bool {
    let (Some(post), Some((color, fb))) = (post, copy.packed()) else {
        return false;
    };
    if !post.pack_depth(&mut ctx.shaders, copy.texture, fb, copy.size) {
        return false;
    }
    let mut bytes = vec![0u8; depth.len() * 4];
    // SAFETY: the live device and packed texture, on the device's thread; `bytes` holds the rect.
    let ok = unsafe {
        ffi::gfx_dll_read_texture(
            copy.device,
            color,
            0,
            0,
            copy.size.x,
            copy.size.y,
            bytes.len() as u32,
            bytes.as_mut_ptr().cast(),
        )
    };
    for (d, b) in depth.iter_mut().zip(bytes.as_chunks::<4>().0) {
        *d = unpack_depth(*b);
    }
    ok
}

/// A texel of [`crate::post::FfxPost::pack_depth`]'s target: the float's bits, least significant
/// byte in red.
fn unpack_depth(rgba: [u8; 4]) -> f32 {
    f32::from_bits(u32::from_le_bytes(rgba))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_packed_texel_is_the_floats_bits_red_first() {
        for d in [0.0f32, 1.0, 0.123_456_78, 3.4e-7] {
            let b = d.to_bits();
            let rgba = [b as u8, (b >> 8) as u8, (b >> 16) as u8, (b >> 24) as u8];
            assert_eq!(unpack_depth(rgba).to_bits(), b);
        }
    }
}
