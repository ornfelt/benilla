#![expect(missing_docs, reason = "Not all docs are written yet, see #3492.")]

extern crate alloc;

pub mod prelude {
    pub use crate::{
        dynamic_texture_atlas_builder::DynamicTextureAtlasBuilder,
        texture_atlas::{TextureAtlas, TextureAtlasLayout},
        BevyDefault as _, Image, ImageFormat, ImagePlugin, TextureError,
    };
}

#[cfg(all(feature = "zstd", not(feature = "zstd_rust")))]
compile_error!(
    "Choosing a zstd backend is required for zstd support. Please enable the \"zstd_rust\" feature."
);

mod image;
pub use self::image::*;
mod dynamic_texture_atlas_builder;
mod image_loader;
#[cfg(feature = "ktx2")]
mod ktx2;
mod texture_atlas;

pub use dynamic_texture_atlas_builder::*;
pub use image_loader::*;
#[cfg(feature = "ktx2")]
pub use ktx2::*;
pub use texture_atlas::*;

pub(crate) mod image_texture_conversion;
pub use image_texture_conversion::IntoDynamicImageError;
