//! `Assets<Image>` on the device: a gfx texture per 2D image (and 2D array), every mip level
//! uploaded, made on first draw and dropped when the asset changes or goes. BC1-3 blocks go up
//! as they are, as the wgpu path uploads them.
//!
//! An `*Srgb` image goes up as `gfx_benilla`'s sRGB format of the same layout, so sampling
//! decodes each texel before filtering, as wgpu's sRGB views do. (Decoding in the shader after the
//! filter was measured off by up to 184 levels on a magnified 2x2 texture, `examples/parity.rs`.)
//!
//! A gfx texture carries its own sampling state; it is the image's `ImageSampler`, or the
//! `ImagePlugin` default for `ImageSampler::Default`. A draw that samples an image through a
//! sampler of its own, as wgpu binds one beside a texture, takes a [`GfxSampler`] variant: the
//! same texels uploaded again with that state, unless it is the image's own.

use std::collections::HashMap;
use std::ptr;

use bevy::asset::AssetId;
use bevy::image::{Image, ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::render::render_resource::{
    TextureDataOrder, TextureDimension, TextureFormat, TextureViewDimension,
};

use crate::ffi::{
    self, texture_usage, GfxDevice, GfxFiltering, GfxFormat, GfxTexture, GfxTextureAddressing,
    GfxTextureType,
};

/// An image on the device.
pub struct GpuImage {
    pub texture: GfxTexture,
    pub width: u32,
    pub height: u32,
}

/// How an image format goes to gfx: the gfx format, and whether the texels are swizzled BGRA to
/// RGBA on the way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Upload {
    pub format: GfxFormat,
    pub swizzle: bool,
    /// `(block width and height, bytes per block)`; `(1, bytes per texel)` when uncompressed.
    pub block: (u32, u32),
}

pub fn upload_format(format: TextureFormat) -> Option<Upload> {
    use TextureFormat as T;
    let plain = |format, bytes| Upload {
        format,
        swizzle: false,
        block: (1, bytes),
    };
    let bc = |format, bytes| Upload {
        format,
        swizzle: false,
        block: (4, bytes),
    };
    Some(match format {
        T::Rgba8Unorm => plain(GfxFormat::R8G8B8A8Unorm, 4),
        T::Rgba8UnormSrgb => plain(GfxFormat::R8G8B8A8Srgb, 4),
        T::Bgra8Unorm => Upload {
            swizzle: true,
            ..plain(GfxFormat::R8G8B8A8Unorm, 4)
        },
        T::Bgra8UnormSrgb => Upload {
            swizzle: true,
            ..plain(GfxFormat::R8G8B8A8Srgb, 4)
        },
        T::Rgba8Snorm => plain(GfxFormat::R8G8B8A8Snorm, 4),
        T::Rgba8Uint => plain(GfxFormat::R8G8B8A8Uint, 4),
        T::R8Unorm => plain(GfxFormat::R8Unorm, 1),
        T::R8Uint => plain(GfxFormat::R8Uint, 1),
        T::Rg8Unorm => plain(GfxFormat::R8G8Unorm, 2),
        T::R16Float => plain(GfxFormat::R16Sfloat, 2),
        T::R16Unorm => plain(GfxFormat::R16Unorm, 2),
        T::Rg16Float => plain(GfxFormat::R16G16Sfloat, 4),
        T::Rgba16Float => plain(GfxFormat::R16G16B16A16Sfloat, 8),
        T::Rgba16Unorm => plain(GfxFormat::R16G16B16A16Unorm, 8),
        T::R32Float => plain(GfxFormat::R32Sfloat, 4),
        T::R32Uint => plain(GfxFormat::R32Uint, 4),
        T::Rg32Float => plain(GfxFormat::R32G32Sfloat, 8),
        T::Rgba32Float => plain(GfxFormat::R32G32B32A32Sfloat, 16),
        T::Bc1RgbaUnorm => bc(GfxFormat::Bc1RgbaUnormBlock, 8),
        T::Bc1RgbaUnormSrgb => bc(GfxFormat::Bc1RgbaSrgbBlock, 8),
        T::Bc2RgbaUnorm => bc(GfxFormat::Bc2UnormBlock, 16),
        T::Bc2RgbaUnormSrgb => bc(GfxFormat::Bc2SrgbBlock, 16),
        T::Bc3RgbaUnorm => bc(GfxFormat::Bc3UnormBlock, 16),
        T::Bc3RgbaUnormSrgb => bc(GfxFormat::Bc3SrgbBlock, 16),
        T::Bc4RUnorm => bc(GfxFormat::Bc4UnormBlock, 8),
        T::Bc4RSnorm => bc(GfxFormat::Bc4SnormBlock, 8),
        T::Bc5RgUnorm => bc(GfxFormat::Bc5UnormBlock, 16),
        T::Bc5RgSnorm => bc(GfxFormat::Bc5SnormBlock, 16),
        _ => return None,
    })
}

impl Upload {
    /// Bytes of one layer of mip `level` of a `width` x `height` image.
    pub fn level_bytes(&self, width: u32, height: u32, level: u32) -> usize {
        let (w, h) = ((width >> level).max(1), (height >> level).max(1));
        let (block, bytes) = self.block;
        (w.div_ceil(block) * h.div_ceil(block) * bytes) as usize
    }
}

/// A sampler a draw binds beside an image in place of the image's own: linear min and mag, the
/// address mode per axis (repeat or clamp to edge), the mip filter (linear, else nearest) and the
/// anisotropy clamp.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GfxSampler {
    pub repeat: [bool; 2],
    pub mipmap_linear: bool,
    pub anisotropy: u16,
}

impl GfxSampler {
    /// `d` as a `GfxSampler`, when it is one.
    pub fn of(d: &ImageSamplerDescriptor) -> Option<Self> {
        let repeat = |mode| match mode {
            ImageAddressMode::Repeat => Some(true),
            ImageAddressMode::ClampToEdge => Some(false),
            _ => None,
        };
        let sampler = Self {
            repeat: [repeat(d.address_mode_u)?, repeat(d.address_mode_v)?],
            mipmap_linear: d.mipmap_filter == ImageFilterMode::Linear,
            anisotropy: d.anisotropy_clamp,
        };
        (sampler.descriptor() == *d).then_some(sampler)
    }

    pub fn descriptor(self) -> ImageSamplerDescriptor {
        let mode = |repeat| {
            if repeat {
                ImageAddressMode::Repeat
            } else {
                ImageAddressMode::ClampToEdge
            }
        };
        ImageSamplerDescriptor {
            address_mode_u: mode(self.repeat[0]),
            address_mode_v: mode(self.repeat[1]),
            mag_filter: ImageFilterMode::Linear,
            min_filter: ImageFilterMode::Linear,
            mipmap_filter: if self.mipmap_linear {
                ImageFilterMode::Linear
            } else {
                ImageFilterMode::Nearest
            },
            anisotropy_clamp: self.anisotropy,
            ..Default::default()
        }
    }
}

/// Every uploaded image by asset id, and the stand-ins drawn in place of a missing texture.
pub struct GpuImages {
    device: GfxDevice,
    images: HashMap<AssetId<Image>, GpuImage>,
    /// Images uploaded again under a sampler other than their own.
    variants: HashMap<(AssetId<Image>, GfxSampler), GpuImage>,
    stale: HashMap<AssetId<Image>, ()>,
    /// Opaque white, 1x1: the texture a material without one samples.
    pub white: GfxTexture,
    default_sampler: ImageSamplerDescriptor,
    /// Formats and shapes refused so far, logged once each.
    refused: Vec<String>,
}

impl GpuImages {
    pub fn new(device: GfxDevice, default_sampler: ImageSamplerDescriptor) -> Self {
        let white = solid(device, [255, 255, 255, 255]);
        Self {
            device,
            images: HashMap::new(),
            variants: HashMap::new(),
            stale: HashMap::new(),
            white,
            default_sampler,
            refused: Vec::new(),
        }
    }

    pub fn modified(&mut self, id: AssetId<Image>) {
        if self.images.contains_key(&id) || self.variants.keys().any(|(v, _)| *v == id) {
            self.stale.insert(id, ());
        }
    }

    pub fn removed(&mut self, id: AssetId<Image>) {
        self.stale.remove(&id);
        let device = self.device;
        if let Some(i) = self.images.remove(&id) {
            // SAFETY: made on this device, owned by this entry alone.
            unsafe { ffi::gfx_dll_delete_texture(device, i.texture) };
        }
        self.variants.retain(|(v, _), i| {
            let keep = *v != id;
            if !keep {
                // SAFETY: made on this device, owned by this entry alone.
                unsafe { ffi::gfx_dll_delete_texture(device, i.texture) };
            }
            keep
        });
    }

    pub fn len(&self) -> usize {
        self.images.len()
    }

    pub fn is_empty(&self) -> bool {
        self.images.is_empty()
    }

    /// The image on the device, uploading it from `image` first when needed; `None` for an image
    /// with no data or a format or shape gfx cannot take.
    pub fn get(&mut self, id: AssetId<Image>, image: &Image) -> Option<&GpuImage> {
        if self.stale.remove(&id).is_some() {
            self.removed(id);
        }
        if !self.images.contains_key(&id) {
            match upload(self.device, image, &self.default_sampler, false) {
                Ok(gpu) => {
                    self.images.insert(id, gpu);
                }
                Err(why) => {
                    if !self.refused.contains(&why) {
                        bevy::log::warn!("gfx: an image is not uploaded: {why}");
                        self.refused.push(why);
                    }
                    return None;
                }
            }
        }
        self.images.get(&id)
    }

    /// The image on the device as `sampler` samples it: the image itself when that is its own
    /// sampler, else a variant uploaded with it.
    pub fn get_sampled(
        &mut self,
        id: AssetId<Image>,
        image: &Image,
        sampler: GfxSampler,
    ) -> Option<&GpuImage> {
        let descriptor = sampler.descriptor();
        let own = match &image.sampler {
            ImageSampler::Default => &self.default_sampler,
            ImageSampler::Descriptor(d) => d,
        };
        if *own == descriptor {
            return self.get(id, image);
        }
        if self.stale.remove(&id).is_some() {
            self.removed(id);
        }
        if !self.variants.contains_key(&(id, sampler)) {
            match upload(self.device, image, &descriptor, true) {
                Ok(gpu) => {
                    self.variants.insert((id, sampler), gpu);
                }
                Err(why) => {
                    if !self.refused.contains(&why) {
                        bevy::log::warn!("gfx: an image is not uploaded: {why}");
                        self.refused.push(why);
                    }
                    return None;
                }
            }
        }
        self.variants.get(&(id, sampler))
    }

    /// The image on the device as `like`'s sampler samples it, as a Bevy `#[sampler]` binding
    /// samples every texture of its group with one image's sampler.
    pub fn get_sampled_like(
        &mut self,
        id: AssetId<Image>,
        image: &Image,
        like: &Image,
    ) -> Option<&GpuImage> {
        let own = |i: &Image| match &i.sampler {
            ImageSampler::Default => self.default_sampler.clone(),
            ImageSampler::Descriptor(d) => d.clone(),
        };
        let (mine, theirs) = (own(image), own(like));
        if mine == theirs {
            return self.get(id, image);
        }
        match GfxSampler::of(&theirs) {
            Some(sampler) => self.get_sampled(id, image, sampler),
            None => {
                let why = format!("a shared sampler gfx cannot key ({theirs:?})");
                if !self.refused.contains(&why) {
                    bevy::log::warn!("gfx: {why}; the image samples with its own");
                    self.refused.push(why);
                }
                self.get(id, image)
            }
        }
    }

    /// Deletes every texture; the device must still be alive.
    pub fn clear(&mut self) {
        // SAFETY: every texture was made on this device and is dropped from the store here.
        unsafe {
            for (_, i) in self.images.drain() {
                ffi::gfx_dll_delete_texture(self.device, i.texture);
            }
            for (_, i) in self.variants.drain() {
                ffi::gfx_dll_delete_texture(self.device, i.texture);
            }
            if !self.white.is_null() {
                ffi::gfx_dll_delete_texture(self.device, self.white);
                self.white = ptr::null_mut();
            }
        }
        self.stale.clear();
    }
}

impl Drop for GpuImages {
    fn drop(&mut self) {
        self.clear();
    }
}

/// Uploads `image`, sampled by its own sampler (`default_sampler` standing in for
/// `ImageSampler::Default`) or, with `forced`, by `default_sampler` whatever its own.
fn upload(
    device: GfxDevice,
    image: &Image,
    default_sampler: &ImageSamplerDescriptor,
    forced: bool,
) -> Result<GpuImage, String> {
    let desc = &image.texture_descriptor;
    let format = desc.format;
    let up = upload_format(format).ok_or_else(|| format!("format {format:?}"))?;
    if desc.dimension != TextureDimension::D2 {
        return Err(format!("dimension {:?}", desc.dimension));
    }
    let view = image
        .texture_view_descriptor
        .as_ref()
        .and_then(|v| v.dimension);
    let layers = desc.size.depth_or_array_layers.max(1);
    let array = matches!(view, Some(TextureViewDimension::D2Array)) || layers > 1;
    if matches!(
        view,
        Some(TextureViewDimension::Cube | TextureViewDimension::CubeArray)
    ) {
        return Err("cube view".into());
    }
    let data = image.data.as_deref().ok_or("no data")?;
    let (width, height) = (desc.size.width, desc.size.height);
    let levels = desc.mip_level_count.max(1);
    let sampler = match &image.sampler {
        _ if forced => default_sampler,
        ImageSampler::Default => default_sampler,
        ImageSampler::Descriptor(d) => d,
    };
    let info = ffi::GfxTextureCreateInfo {
        texture_type: if array {
            GfxTextureType::Texture2DArray
        } else {
            GfxTextureType::Texture2D
        },
        usage: texture_usage::SAMPLED,
        format: up.format,
        levels: levels.min(u8::MAX as u32) as u8,
        width,
        height,
        depth: if array { layers } else { 1 },
        addressing_s: addressing(sampler.address_mode_u),
        addressing_t: addressing(sampler.address_mode_v),
        addressing_r: addressing(sampler.address_mode_w),
        min_filtering: filtering(sampler.min_filter),
        mag_filtering: filtering(sampler.mag_filter),
        mip_filtering: if levels > 1 {
            filtering(sampler.mipmap_filter)
        } else {
            GfxFiltering::None
        },
        anisotropy: if sampler.anisotropy_clamp > 1 {
            sampler.anisotropy_clamp as u32
        } else {
            0
        },
        border_color: [0.0; 4],
    };
    let mut texture: GfxTexture = ptr::null_mut();
    // SAFETY: `info` is live for the call.
    if !unsafe { ffi::gfx_dll_create_texture(device, &info, &mut texture) } {
        return Err(format!(
            "gfx_dll_create_texture failed ({format:?} {width}x{height})"
        ));
    }
    let layer_count = if array { layers } else { 1 };
    // Every device writes a 2D array one layer a call, `offset` naming it (`data.rs` alike).
    let result = (0..levels).try_for_each(|level| {
        let bytes = level_data(&up, image, data, level, layer_count)?;
        let (w, h) = ((width >> level).max(1), (height >> level).max(1));
        let layer_bytes = bytes.len() / layer_count as usize;
        bytes
            .chunks_exact(layer_bytes)
            .enumerate()
            .try_for_each(|(layer, texels)| {
                // SAFETY: `texture` is live; `texels` holds one whole layer of the level and
                // outlives the call.
                let ok = unsafe {
                    ffi::gfx_dll_set_texture_data(
                        device,
                        texture,
                        level as u8,
                        if array { layer as u32 } else { 0 },
                        w,
                        h,
                        1,
                        texels.len() as u32,
                        texels.as_ptr().cast(),
                    )
                };
                ok.then_some(()).ok_or_else(|| {
                    format!("gfx_dll_set_texture_data failed at level {level}, layer {layer}")
                })
            })
    });
    if let Err(e) = result {
        // SAFETY: made above, referenced nowhere else.
        unsafe { ffi::gfx_dll_delete_texture(device, texture) };
        return Err(e);
    }
    Ok(GpuImage {
        texture,
        width,
        height,
    })
}

/// Every layer of mip `level`, layer after layer, from Bevy's layout of `data` (layer-major by
/// default: each layer's whole chain in turn; mip-major keeps each level's layers together).
fn level_data(
    up: &Upload,
    image: &Image,
    data: &[u8],
    level: u32,
    layers: u32,
) -> Result<Vec<u8>, String> {
    let desc = &image.texture_descriptor;
    let (width, height) = (desc.size.width, desc.size.height);
    let levels = desc.mip_level_count.max(1);
    let size = |l| up.level_bytes(width, height, l);
    let chain: usize = (0..levels).map(size).sum();
    let mut out = Vec::with_capacity(size(level) * layers as usize);
    for layer in 0..layers as usize {
        let start = match image.data_order {
            TextureDataOrder::LayerMajor => layer * chain + (0..level).map(size).sum::<usize>(),
            TextureDataOrder::MipMajor => {
                (0..level).map(size).sum::<usize>() * layers as usize + layer * size(level)
            }
        };
        let bytes = data
            .get(start..start + size(level))
            .ok_or_else(|| format!("{} bytes short of level {level}", start + size(level)))?;
        out.extend_from_slice(bytes);
    }
    if up.swizzle {
        for texel in out.as_chunks_mut::<4>().0 {
            texel.swap(0, 2);
        }
    }
    Ok(out)
}

fn addressing(mode: ImageAddressMode) -> GfxTextureAddressing {
    match mode {
        ImageAddressMode::ClampToEdge => GfxTextureAddressing::Clamp,
        ImageAddressMode::Repeat => GfxTextureAddressing::Repeat,
        ImageAddressMode::MirrorRepeat => GfxTextureAddressing::Mirror,
        ImageAddressMode::ClampToBorder => GfxTextureAddressing::Border,
    }
}

fn filtering(mode: ImageFilterMode) -> GfxFiltering {
    match mode {
        ImageFilterMode::Nearest => GfxFiltering::Nearest,
        ImageFilterMode::Linear => GfxFiltering::Linear,
    }
}

/// A 1x1 RGBA8 texture of `rgba`.
fn solid(device: GfxDevice, rgba: [u8; 4]) -> GfxTexture {
    let info = ffi::GfxTextureCreateInfo {
        texture_type: GfxTextureType::Texture2D,
        usage: texture_usage::SAMPLED,
        format: GfxFormat::R8G8B8A8Unorm,
        levels: 1,
        width: 1,
        height: 1,
        depth: 1,
        addressing_s: GfxTextureAddressing::Repeat,
        addressing_t: GfxTextureAddressing::Repeat,
        addressing_r: GfxTextureAddressing::Repeat,
        min_filtering: GfxFiltering::Nearest,
        mag_filtering: GfxFiltering::Nearest,
        mip_filtering: GfxFiltering::None,
        anisotropy: 0,
        border_color: [0.0; 4],
    };
    let mut texture: GfxTexture = ptr::null_mut();
    // SAFETY: `info` and `rgba` are live for the calls.
    unsafe {
        if ffi::gfx_dll_create_texture(device, &info, &mut texture) {
            ffi::gfx_dll_set_texture_data(device, texture, 0, 0, 1, 1, 1, 4, rgba.as_ptr().cast());
        }
    }
    texture
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_blp_sampler_keys_and_an_unkeyable_one_does_not() {
        // The BLP loader's model sampler: linear, clamped U, repeated V, trilinear, aniso 8.
        let blp = ImageSamplerDescriptor {
            address_mode_u: ImageAddressMode::ClampToEdge,
            address_mode_v: ImageAddressMode::Repeat,
            mag_filter: ImageFilterMode::Linear,
            min_filter: ImageFilterMode::Linear,
            mipmap_filter: ImageFilterMode::Linear,
            anisotropy_clamp: 8,
            ..Default::default()
        };
        let s = GfxSampler::of(&blp).expect("keyable");
        assert_eq!(s.repeat, [false, true]);
        assert_eq!(s.descriptor(), blp);
        // Nearest magnification is not a `GfxSampler`.
        let nearest = ImageSamplerDescriptor {
            mag_filter: ImageFilterMode::Nearest,
            ..blp
        };
        assert_eq!(GfxSampler::of(&nearest), None);
    }

    #[test]
    fn bc_levels_round_up_to_whole_blocks() {
        let bc1 = upload_format(TextureFormat::Bc1RgbaUnormSrgb).unwrap();
        assert_eq!(bc1.format, GfxFormat::Bc1RgbaSrgbBlock);
        assert_eq!(bc1.level_bytes(64, 32, 0), 16 * 8 * 8);
        // 2x2 and 1x1 levels are one block each.
        assert_eq!(bc1.level_bytes(64, 32, 5), 8);
        assert_eq!(bc1.level_bytes(64, 32, 6), 8);
        let bc3 = upload_format(TextureFormat::Bc3RgbaUnorm).unwrap();
        assert_eq!(bc3.level_bytes(8, 8, 0), 4 * 16);
    }

    #[test]
    fn bgra_is_swizzled_into_the_srgb_format() {
        let up = upload_format(TextureFormat::Bgra8UnormSrgb).unwrap();
        assert_eq!(up.format, GfxFormat::R8G8B8A8Srgb);
        assert!(up.swizzle);
        assert_eq!(up.level_bytes(3, 5, 1), 4 * 2);
    }
}
