//! Storage buffers as gfx data textures. gfx has no storage buffers, so a buffer a shader reads
//! by index (benilla's shared `wow_light` blob) is a float RGBA 2D-array texture of the same
//! `vec4` rows, fetched texel by texel (`texelFetch` / `Load`). The main world keeps each one's
//! rows in a [`DataTexture`] keyed by the Bevy buffer a material binds; the renderer uploads the
//! layers a write touched before the frame's draws.
//!
//! A layer is [`DATA_WIDTH`] x [`DATA_HEIGHT`] rows: every device updates a 2D array one whole
//! layer at a time (`gfx_dll_set_texture_data`'s `offset` is the layer), so a small write costs
//! one 64 KB layer. Row `i` sits at `(i % 256, (i / 256) % 16, i / 4096)`; the shaders' fetch
//! helpers mirror it.

use std::collections::HashMap;
use std::ptr;

use bevy::prelude::*;
use bevy::render::render_resource::BufferId;

use crate::ffi::{
    self, texture_usage, GfxDevice, GfxFiltering, GfxFormat, GfxTexture, GfxTextureAddressing,
    GfxTextureType,
};

pub const DATA_WIDTH: u32 = 256;
pub const DATA_HEIGHT: u32 = 16;
/// Rows a layer.
pub const DATA_LAYER_ROWS: usize = (DATA_WIDTH * DATA_HEIGHT) as usize;

/// One buffer's rows and the layers written since the last upload.
pub struct DataTexture {
    rows: Vec<[f32; 4]>,
    dirty: Vec<bool>,
}

impl DataTexture {
    /// `rows` zeroed rows, rounded up to whole layers.
    pub fn new(rows: usize) -> Self {
        let layers = rows.div_ceil(DATA_LAYER_ROWS).max(1);
        Self {
            rows: vec![[0.0; 4]; layers * DATA_LAYER_ROWS],
            dirty: vec![true; layers],
        }
    }

    pub fn layers(&self) -> u32 {
        self.dirty.len() as u32
    }

    pub fn rows(&self) -> &[[f32; 4]] {
        &self.rows
    }

    /// Writes `rows` from row `at`, marking the layers they touch; rows past the end are dropped.
    pub fn write(&mut self, at: usize, rows: &[[f32; 4]]) {
        let end = (at + rows.len()).min(self.rows.len());
        if at >= end {
            return;
        }
        let dst = &mut self.rows[at..end];
        let src = &rows[..end - at];
        if dst == src {
            return;
        }
        dst.copy_from_slice(src);
        for layer in at / DATA_LAYER_ROWS..=(end - 1) / DATA_LAYER_ROWS {
            self.dirty[layer] = true;
        }
    }

    /// The row at `i`, zero past the end.
    pub fn row(&self, i: usize) -> [f32; 4] {
        self.rows.get(i).copied().unwrap_or([0.0; 4])
    }
}

/// Every data texture by the Bevy buffer it stands in for.
#[derive(Resource, Default)]
pub struct GfxDataTextures {
    textures: HashMap<BufferId, DataTexture>,
}

impl GfxDataTextures {
    /// `id`'s texture, made with `rows` zeroed rows when it has none yet.
    pub fn get_or_insert(&mut self, id: BufferId, rows: usize) -> &mut DataTexture {
        self.textures
            .entry(id)
            .or_insert_with(|| DataTexture::new(rows))
    }

    pub fn get(&self, id: BufferId) -> Option<&DataTexture> {
        self.textures.get(&id)
    }

    pub fn remove(&mut self, id: BufferId) {
        self.textures.remove(&id);
    }
}

/// The data textures on the device.
pub struct GpuDataTextures {
    device: GfxDevice,
    textures: HashMap<BufferId, (GfxTexture, u32)>,
}

impl GpuDataTextures {
    pub fn new(device: GfxDevice) -> Self {
        Self {
            device,
            textures: HashMap::new(),
        }
    }

    pub fn get(&self, id: BufferId) -> Option<GfxTexture> {
        self.textures.get(&id).map(|t| t.0)
    }

    /// Uploads every written layer, making a texture for a new buffer and dropping the textures
    /// of buffers no longer kept.
    pub(crate) fn sync(&mut self, data: &mut GfxDataTextures) {
        let device = self.device;
        self.textures.retain(|id, (t, _)| {
            let keep = data.textures.contains_key(id);
            if !keep {
                // SAFETY: made on this device, owned by this entry alone.
                unsafe { ffi::gfx_dll_delete_texture(device, *t) };
            }
            keep
        });
        for (id, texture) in &mut data.textures {
            let layers = texture.layers();
            let entry = match self.textures.get(id) {
                Some(&(t, n)) if n == layers => Some(t),
                other => {
                    if let Some(&(t, _)) = other {
                        // SAFETY: made on this device; re-made below at the new size.
                        unsafe { ffi::gfx_dll_delete_texture(device, t) };
                    }
                    texture.dirty.fill(true);
                    create(device, layers).inspect(|t| {
                        self.textures.insert(*id, (*t, layers));
                    })
                }
            };
            let Some(gpu) = entry else {
                self.textures.remove(id);
                continue;
            };
            for layer in 0..layers as usize {
                if !std::mem::take(&mut texture.dirty[layer]) {
                    continue;
                }
                let rows = &texture.rows[layer * DATA_LAYER_ROWS..(layer + 1) * DATA_LAYER_ROWS];
                // SAFETY: `rows` is one whole layer of the texture's RGBA32F format.
                unsafe {
                    ffi::gfx_dll_set_texture_data(
                        device,
                        gpu,
                        0,
                        layer as u32,
                        DATA_WIDTH,
                        DATA_HEIGHT,
                        1,
                        std::mem::size_of_val(rows) as u32,
                        rows.as_ptr().cast(),
                    );
                }
            }
        }
    }

    pub fn clear(&mut self) {
        for (_, (t, _)) in self.textures.drain() {
            // SAFETY: made on this device, dropped from the store here.
            unsafe { ffi::gfx_dll_delete_texture(self.device, t) };
        }
    }
}

impl Drop for GpuDataTextures {
    fn drop(&mut self) {
        self.clear();
    }
}

fn create(device: GfxDevice, layers: u32) -> Option<GfxTexture> {
    let info = ffi::GfxTextureCreateInfo {
        texture_type: GfxTextureType::Texture2DArray,
        usage: texture_usage::SAMPLED,
        format: GfxFormat::R32G32B32A32Sfloat,
        levels: 1,
        width: DATA_WIDTH,
        height: DATA_HEIGHT,
        depth: layers,
        addressing_s: GfxTextureAddressing::Clamp,
        addressing_t: GfxTextureAddressing::Clamp,
        addressing_r: GfxTextureAddressing::Clamp,
        min_filtering: GfxFiltering::Nearest,
        mag_filtering: GfxFiltering::Nearest,
        mip_filtering: GfxFiltering::None,
        anisotropy: 0,
        border_color: [0.0; 4],
    };
    let mut texture: GfxTexture = ptr::null_mut();
    // SAFETY: `info` is live for the call.
    unsafe { ffi::gfx_dll_create_texture(device, &info, &mut texture) }.then_some(texture)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_write_marks_only_the_layers_it_touches() {
        let mut t = DataTexture::new(3 * DATA_LAYER_ROWS);
        t.dirty.fill(false);
        t.write(DATA_LAYER_ROWS - 1, &[[1.0; 4], [2.0; 4]]);
        assert_eq!(t.dirty, vec![true, true, false]);
        t.dirty.fill(false);
        // The same rows again change nothing.
        t.write(DATA_LAYER_ROWS - 1, &[[1.0; 4], [2.0; 4]]);
        assert_eq!(t.dirty, vec![false; 3]);
        assert_eq!(t.row(DATA_LAYER_ROWS), [2.0; 4]);
    }

    #[test]
    fn rows_round_up_to_whole_layers_and_overflow_is_dropped() {
        let mut t = DataTexture::new(DATA_LAYER_ROWS + 1);
        assert_eq!(t.layers(), 2);
        t.write(2 * DATA_LAYER_ROWS - 1, &[[1.0; 4], [5.0; 4]]);
        assert_eq!(t.row(2 * DATA_LAYER_ROWS - 1), [1.0; 4]);
        assert_eq!(t.row(2 * DATA_LAYER_ROWS), [0.0; 4]);
    }
}
