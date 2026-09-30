use crate::render_resource::{Texture, TextureView};

/// A cached GPU [`Texture`] with corresponding [`TextureView`].
///
/// This is useful for textures that are created repeatedly (each frame) in the rendering process
/// to reduce the amount of GPU memory allocations.
#[derive(Clone)]
pub struct CachedTexture {
    pub texture: Texture,
    pub default_view: TextureView,
}
