//! The shared `wow_light` buffer on the gfx renderer. gfx has no storage buffers, so the buffer's
//! rows live in a data texture (`benilla_gfx::data`) keyed by the [`SharedLightBuffer`] every world
//! material binds, at the same row index as the wgpu buffer's byte offset / 16. [`pack`] writes it
//! from the same main-world tables the render-world uploads read, gated on the same generations:
//! the per-frame blob, the prop probes, the rig slot, tint and origin tables, the mat-anim rows,
//! the straddle clips and the palette's dirty bone ranges.
//!
//! The `u32` regions hold floats here, since a float texel is not a safe carrier for arbitrary
//! bits: the rig slot table its base bone indices (exact below 2^24), and the tint table
//! `word & 0xFFFFFF`, with `-1` for the identity word 0 (a black tint is `0xFF000000`, so 0).

use bevy::prelude::*;

use benilla_gfx::GfxDataTextures;

use crate::lighting::SharedLightBuffer;

/// A wgpu byte offset as a data-texture row.
fn row(offset: u64) -> usize {
    (offset / 16) as usize
}

/// Every region's first row, in `wow_model.wgsl`'s struct order.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Bases {
    pub points: usize,
    pub probes: usize,
    pub rig_table: usize,
    pub rig_tint: usize,
    pub rig_origin: usize,
    pub matanim: usize,
    pub water_clip: usize,
    pub palette: usize,
    pub end: usize,
}

pub(crate) fn bases() -> Bases {
    Bases {
        points: crate::lighting::LIGHT_HEADER_ROWS,
        probes: row(crate::lighting::prop_probe_region_offset()),
        rig_table: row(crate::rig_palette::rig_table_region_offset()),
        rig_tint: row(crate::instance_tint::region_offset()),
        rig_origin: row(crate::rig_palette::rig_origin_region_offset()),
        matanim: row(crate::mat_anim_table::region_offset()),
        water_clip: row(crate::straddle::region_offset()),
        palette: row(crate::rig_palette::palette_region_offset()),
        end: row(crate::lighting::light_blob_bytes()),
    }
}

/// What the last pack wrote, as each upload system's `Local` gate.
#[derive(Default)]
pub(crate) struct Seen {
    probes: Option<u64>,
    rig_table: Option<u64>,
    rig_origin: Option<u64>,
    palette_dirty: usize,
    palette_primed: bool,
    tints: Option<u64>,
    matanim: Option<u64>,
    clips: Option<u64>,
}

/// `GfxRenderSystems::Pack`: writes what changed into the shared buffer's data texture.
pub(crate) fn pack(world: &mut World, mut seen: Local<Seen>) {
    let Some(id) = world.get_resource::<SharedLightBuffer>().map(|b| b.0.id()) else {
        return;
    };
    world.resource_scope(|world, mut textures: Mut<GfxDataTextures>| {
        let b = bases();
        let tex = textures.get_or_insert(id, b.end);
        let seen = &mut *seen;

        if let Some(rows) = crate::lighting::gfx_light_rows(world) {
            tex.write(0, rows);
        }

        if let Some((rows, high, generation, dirty)) = crate::lighting::gfx_prop_probes(world) {
            if seen.probes != Some(generation) {
                let high = high.min(rows.len());
                let consecutive = seen.probes.is_some_and(|s| s.wrapping_add(1) == generation);
                let (lo, hi) = match dirty {
                    Some((lo, hi)) if consecutive => (lo.min(high), hi.min(high)),
                    _ => (0, high),
                };
                seen.probes = Some(generation);
                let flat: Vec<[f32; 4]> = rows[lo..hi].iter().flatten().copied().collect();
                tex.write(b.probes + 7 * lo, &flat);
            }
        }

        if let Some(p) = crate::rig_palette::gfx_palettes(world) {
            if seen.rig_table != Some(p.table_generation) && !p.table.is_empty() {
                seen.rig_table = Some(p.table_generation);
                tex.write(b.rig_table, &pack4(p.table.iter().map(|&base| base as f32)));
            }
            if seen.rig_origin != Some(p.origin_generation) && !p.origins.is_empty() {
                seen.rig_origin = Some(p.origin_generation);
                tex.write(b.rig_origin, p.origins);
            }
            if !seen.palette_primed && !p.rows.is_empty() {
                // The first sight writes every row; after it only the published dirty ranges.
                seen.palette_primed = true;
                seen.palette_dirty = p.dirty_id;
                tex.write(b.palette, p.rows);
            } else if seen.palette_dirty != p.dirty_id && !p.dirty.is_empty() {
                seen.palette_dirty = p.dirty_id;
                for &(base, len, _) in p.dirty {
                    let (lo, hi) = (3 * base as usize, 3 * (base + len) as usize);
                    if let Some(rows) = p.rows.get(lo..hi) {
                        tex.write(b.palette + lo, rows);
                    }
                }
            }
        }

        if let Some(tints) = world.get_resource::<crate::instance_tint::InstanceTints>() {
            let (slots, generation) = tints.gfx_slots();
            if seen.tints != Some(generation) {
                seen.tints = Some(generation);
                tex.write(b.rig_tint, &pack4(slots.iter().map(|&w| tint_value(w))));
            }
        }

        if let Some(table) = world.get_resource::<crate::mat_anim_table::MatAnimTable>() {
            let (rows, generation) = table.gfx_rows();
            if seen.matanim != Some(generation) {
                seen.matanim = Some(generation);
                tex.write(b.matanim, rows);
            }
        }

        if let Some(clips) = world.get_resource::<crate::straddle::WaterClips>() {
            let (slots, generation) = clips.gfx_slots();
            if seen.clips != Some(generation) {
                seen.clips = Some(generation);
                let rows: Vec<[f32; 4]> = slots
                    .chunks(2)
                    .map(|c| {
                        let hi = c.get(1).copied().unwrap_or([0.0; 2]);
                        [c[0][0], c[0][1], hi[0], hi[1]]
                    })
                    .collect();
                tex.write(b.water_clip, &rows);
            }
        }
    });
}

/// A `u32` tint word as the shader reads it: `-1` for the identity, else its RGB as a float.
fn tint_value(word: u32) -> f32 {
    if word == crate::instance_tint::IDENTITY {
        -1.0
    } else {
        (word & 0x00ff_ffff) as f32
    }
}

/// Four scalars a row.
fn pack4(values: impl Iterator<Item = f32>) -> Vec<[f32; 4]> {
    let values: Vec<f32> = values.collect();
    values
        .chunks(4)
        .map(|c| {
            let mut r = [0.0; 4];
            r[..c.len()].copy_from_slice(c);
            r
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `#define NAME value` from a `.gfxs` source.
    fn define(src: &str, name: &str) -> usize {
        src.lines()
            .find_map(|l| l.strip_prefix(&format!("#define {name} ")))
            .unwrap_or_else(|| panic!("{name} is not defined"))
            .trim()
            .parse()
            .unwrap()
    }

    #[test]
    fn the_model_shader_reads_every_region_where_the_packer_writes_it() {
        let vs = include_str!("../../../benilla-gfx/shaders/src/wow_model.vs.gfxs");
        let fs = include_str!("../../../benilla-gfx/shaders/src/wow_model.fs.gfxs");
        let b = bases();
        assert_eq!(define(vs, "WOW_POINTS_BASE"), b.points);
        assert_eq!(define(vs, "WOW_RIG_TABLE_BASE"), b.rig_table);
        assert_eq!(define(vs, "WOW_RIG_ORIGIN_BASE"), b.rig_origin);
        assert_eq!(define(vs, "WOW_PALETTE_BASE"), b.palette);
        assert_eq!(define(fs, "WOW_PROBES_BASE"), b.probes);
        assert_eq!(define(fs, "WOW_RIG_TINT_BASE"), b.rig_tint);
        assert_eq!(define(fs, "WOW_MATANIM_BASE"), b.matanim);
        assert_eq!(define(fs, "WOW_WATER_CLIP_BASE"), b.water_clip);
        // The prefix: 21 header rows, then the point table.
        assert_eq!(b.probes, 21 + 2 * 256);
    }

    #[test]
    fn a_tint_word_keeps_black_apart_from_the_identity() {
        assert_eq!(tint_value(0), -1.0);
        assert_eq!(tint_value(crate::instance_tint::pack([0, 0, 0])), 0.0);
        assert_eq!(
            tint_value(crate::instance_tint::pack([1, 2, 3])),
            ((1 << 16) | (2 << 8) | 3) as f32
        );
    }

    #[test]
    fn scalars_pack_four_a_row() {
        let rows = pack4([1.0, 2.0, 3.0, 4.0, 5.0].into_iter());
        assert_eq!(rows, vec![[1.0, 2.0, 3.0, 4.0], [5.0, 0.0, 0.0, 0.0]]);
    }
}
