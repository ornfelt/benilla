use super::CachedTexture;
use crate::render_resource::{TextureFormat, TextureView};
use crate::wgpu::{LoadOp, Operations, RenderPassColorAttachment, StoreOp};
use alloc::sync::Arc;
use bevy_color::LinearRgba;
use core::sync::atomic::{AtomicBool, Ordering};

/// A wrapper for a [`CachedTexture`] that is used as a [`RenderPassColorAttachment`].
#[derive(Clone)]
pub struct ColorAttachment {
    pub texture: CachedTexture,
    is_first_call: Arc<AtomicBool>,
}

impl ColorAttachment {
    pub(crate) fn mark_as_cleared(&self) {
        self.is_first_call.store(false, Ordering::SeqCst);
    }
}

/// A wrapper for a [`TextureView`] that is used as a [`RenderPassColorAttachment`] for a view
/// target's final output texture.
#[derive(Clone)]
pub struct OutputColorAttachment {
    pub view: TextureView,
    pub view_format: TextureFormat,
    is_first_call: Arc<AtomicBool>,
}

impl OutputColorAttachment {
    /// Get this texture view as an attachment. The attachment will be cleared with a value of
    /// the provided `clear_color` if this is the first time calling this function, otherwise it
    /// will be loaded.
    pub fn get_attachment(&self, clear_color: Option<LinearRgba>) -> RenderPassColorAttachment<'_> {
        let first_call = self.is_first_call.fetch_and(false, Ordering::SeqCst);

        RenderPassColorAttachment {
            view: &self.view,
            depth_slice: None,
            resolve_target: None,
            ops: Operations {
                load: match (clear_color, first_call) {
                    (Some(clear_color), true) => LoadOp::Clear(clear_color.into()),
                    (None, _) | (Some(_), false) => LoadOp::Load,
                },
                store: StoreOp::Store,
            },
        }
    }
}
