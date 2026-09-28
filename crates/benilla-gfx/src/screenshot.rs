//! Bevy's `Screenshot` under gfx. With no render world, `ScreenshotPlugin` keeps only its main-world
//! half: [`CapturedScreenshots`], whose images `trigger_screenshots` turns into
//! `ScreenshotCaptured`. Here a screenshot of the primary window taken during a frame makes the
//! present draw that frame again into a capture texture, the window's pixels as the wgpu path
//! reads its swapchain; the next frame, once the device has run the frame, reads it back and sends
//! it down bevy's channel, so `save_to_disk` and every other observer run as on wgpu.

use std::ptr;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

use bevy::asset::RenderAssetUsages;
use bevy::camera::{NormalizedRenderTarget, RenderTarget};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::render::view::window::screenshot::{CapturedScreenshots, Capturing, Screenshot};
use bevy::window::PrimaryWindow;

use crate::context::GfxContext;
use crate::draw::GfxRenderer;
use crate::ffi::{self, GfxDevice, GfxDeviceBackend, GfxFramebuffer, GfxTexture};
use crate::target::{self, CAPTURE_FORMAT};

/// The sending half of the [`CapturedScreenshots`] channel this module installs.
#[derive(Resource)]
pub(crate) struct ScreenshotSender(Sender<(Entity, Image)>);

/// Replaces `ScreenshotPlugin`'s channel, whose sender went to the render world that gfx has not.
pub(crate) fn finish(app: &mut App) {
    let (tx, rx) = std::sync::mpsc::channel();
    app.insert_resource(CapturedScreenshots(Arc::new(Mutex::new(rx))))
        .insert_resource(ScreenshotSender(tx));
}

/// The capture texture and the screenshots drawn into it this frame.
pub struct CaptureTarget {
    device: GfxDevice,
    texture: GfxTexture,
    framebuffer: GfxFramebuffer,
    size: UVec2,
    pending: Vec<Entity>,
}

impl CaptureTarget {
    fn new(device: GfxDevice, size: UVec2) -> Option<Self> {
        let texture = target::render_texture(device, CAPTURE_FORMAT, size)?;
        let Some(framebuffer) = target::framebuffer(device, texture, ptr::null_mut(), size) else {
            // SAFETY: made above on `device`, referenced nowhere else.
            unsafe { ffi::gfx_dll_delete_texture(device, texture) };
            return None;
        };
        Some(Self {
            device,
            texture,
            framebuffer,
            size,
            pending: Vec::new(),
        })
    }
}

impl Drop for CaptureTarget {
    fn drop(&mut self) {
        // SAFETY: each handle was made on `self.device` and belongs to this target alone.
        unsafe {
            ffi::gfx_dll_delete_framebuffer(self.device, self.framebuffer);
            ffi::gfx_dll_delete_texture(self.device, self.texture);
        }
    }
}

/// `GfxRenderSystems::Present`, after the present: the frame's new screenshots of the primary
/// window draw it into the capture texture.
pub(crate) fn capture(
    mut commands: Commands,
    ctx: NonSend<GfxContext>,
    mut renderer: NonSendMut<GfxRenderer>,
    shots: Query<(Entity, &Screenshot), Without<Capturing>>,
    primary: Query<Entity, With<PrimaryWindow>>,
) {
    let primary = primary.single().ok();
    let mut taken = Vec::new();
    for (entity, shot) in &shots {
        commands.entity(entity).insert(Capturing);
        let on_window = matches!(
            RenderTarget::normalize(&shot.0, primary),
            Some(NormalizedRenderTarget::Window(w)) if Some(w.entity()) == primary
        );
        if on_window {
            taken.push(entity);
        } else {
            warn_once!("gfx: a screenshot of anything but the primary window is not taken");
        }
    }
    if taken.is_empty() {
        return;
    }
    let size = ctx.size().max(UVec2::ONE);
    let renderer = &mut *renderer;
    if renderer.capture.as_ref().is_none_or(|c| c.size != size) {
        // A capture still waiting on the old size is lost with it, as a resize loses wgpu's.
        renderer.capture = CaptureTarget::new(renderer.device(), size);
    }
    let scene = renderer.scene_color();
    let (Some(capture), Some(present), Some(scene)) =
        (renderer.capture.as_mut(), renderer.present.as_mut(), scene)
    else {
        error!("gfx: the screenshot capture target could not be made");
        return;
    };
    present.draw_into(capture.framebuffer, scene, size);
    capture.pending.extend(taken);
}

/// `GfxRenderSystems::Prepare`: last frame's capture, read back and sent as one image per
/// screenshot.
pub(crate) fn collect(mut renderer: NonSendMut<GfxRenderer>, sender: Res<ScreenshotSender>) {
    let backend = renderer.backend();
    let Some(capture) = renderer.capture.as_mut() else {
        return;
    };
    if capture.pending.is_empty() {
        return;
    }
    let (w, h) = (capture.size.x, capture.size.y);
    let mut data = vec![0u8; (w * h * 4) as usize];
    // SAFETY: the live device and capture texture, on the device's thread; `data` holds the rect.
    let ok = unsafe {
        ffi::gfx_dll_read_texture(
            capture.device,
            capture.texture,
            0,
            0,
            w,
            h,
            data.len() as u32,
            data.as_mut_ptr().cast(),
        )
    };
    let pending = std::mem::take(&mut capture.pending);
    if !ok {
        error!("gfx: a screenshot could not be read back");
        return;
    }
    if matches!(
        backend,
        GfxDeviceBackend::Gl3 | GfxDeviceBackend::Gl4 | GfxDeviceBackend::Gles3
    ) {
        flip_rows(&mut data, (w * 4) as usize);
    }
    // The bytes the present stored: sRGB-encoded, as the wgpu path's sRGB swapchain holds them.
    let image = Image::new(
        Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    for entity in pending {
        if sender.0.send((entity, image.clone())).is_err() {
            error!("gfx: a screenshot could not be sent");
        }
    }
}

/// Turns `rows` of `pitch` bytes upside down: GL's first row is the image's bottom.
fn flip_rows(data: &mut [u8], pitch: usize) {
    let rows = data.len() / pitch;
    for r in 0..rows / 2 {
        let (top, bottom) = data.split_at_mut((rows - 1 - r) * pitch);
        top[r * pitch..(r + 1) * pitch].swap_with_slice(&mut bottom[..pitch]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_flip_in_place() {
        let mut data = vec![1, 1, 2, 2, 3, 3];
        flip_rows(&mut data, 2);
        assert_eq!(data, vec![3, 3, 2, 2, 1, 1]);
        let mut even = vec![1, 2, 3, 4];
        flip_rows(&mut even, 1);
        assert_eq!(even, vec![4, 3, 2, 1]);
    }
}
