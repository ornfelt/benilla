//! The shared `wow_light` buffer on the gfx renderer. gfx has no storage buffers, so the buffer's
//! rows live in a data texture (`benilla_gfx::data`) keyed by the [`SharedLightBuffer`] every world
//! material binds, at the same row index as the wgpu buffer's byte offset / 16. [`pack`] writes it
//! from the same main-world tables the render-world uploads read, gated on the same generations:
//! the per-frame blob, the prop probes, the rig slot, tint and origin tables, the mat-anim rows,
//! the straddle clips and the palette's dirty bone ranges.
//!
//! The booths' own buffers (the studio and pane lights, the glue rig, the UI model tiles') are
//! data textures too: each [`crate::lighting::LightBlob`] write lands in its buffer's texture at
//! the next pack, and the regions the render world mirrors into them (the rig palette and tables,
//! the tints, the mat-anim rows) are copied from the shared texture.
//!
//! The `u32` regions hold floats here, since a float texel is not a safe carrier for arbitrary
//! bits: the rig slot table its base bone indices (exact below 2^24), and the tint table
//! `word & 0xFFFFFF`, with `-1` for the identity word 0 (a black tint is `0xFF000000`, so 0).

use std::collections::HashMap;
use std::ops::Range;
use std::sync::Mutex;

use bevy::prelude::*;
use bevy::render::render_resource::BufferId;

use benilla_gfx::{DataTexture, GfxDataTextures};

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
    /// Each mirror buffer and the regions it has been filled with ([`Mirror`]).
    mirrors: HashMap<BufferId, u8>,
}

/// The regions a mirror buffer carries, by the list it is on: the rig palette's
/// ([`crate::rig_palette::RigPaletteMirrors`]: the slot table, origins and palette), the tints'
/// ([`crate::instance_tint::InstanceTintMirrors`]) and the mat-anim table's
/// ([`crate::mat_anim_table::MatAnimMirrors`]).
struct Mirror;

impl Mirror {
    const RIG: u8 = 1;
    const TINT: u8 = 2;
    const MATANIM: u8 = 4;
}

/// The shared regions a pack rewrote: rows `ranges` of the shared texture.
#[derive(Default)]
struct Changed {
    table: bool,
    origin: bool,
    tint: bool,
    matanim: bool,
    palette: Vec<Range<usize>>,
}

/// For gfx: every [`crate::lighting::LightBlob::write`] since the last pack, as (buffer, first
/// row, rows); the booths write their studio blobs where no pack runs.
static BLOB_WRITES: Mutex<Vec<BlobWrite>> = Mutex::new(Vec::new());

/// One queued blob write: the buffer, the first row, the rows.
type BlobWrite = (BufferId, usize, Vec<[f32; 4]>);

/// Queues `rows` for buffer `id`'s data texture from row `at`, for the next [`pack`].
pub(crate) fn record_blob_write(id: BufferId, at: usize, rows: Vec<[f32; 4]>) {
    if let Ok(mut writes) = BLOB_WRITES.lock() {
        writes.push((id, at, rows));
    }
}

/// Every mirror buffer and the lists it is on.
fn mirrors(world: &World) -> HashMap<BufferId, u8> {
    let mut out: HashMap<BufferId, u8> = HashMap::new();
    let mut add = |buffers: Option<Vec<BufferId>>, kind: u8| {
        for id in buffers.unwrap_or_default() {
            *out.entry(id).or_default() |= kind;
        }
    };
    add(
        world
            .get_resource::<crate::rig_palette::RigPaletteMirrors>()
            .map(|m| m.0.values().copied().collect()),
        Mirror::RIG,
    );
    add(
        world
            .get_resource::<crate::instance_tint::InstanceTintMirrors>()
            .map(|m| m.0.values().copied().collect()),
        Mirror::TINT,
    );
    add(
        world
            .get_resource::<crate::mat_anim_table::MatAnimMirrors>()
            .map(|m| m.0.values().copied().collect()),
        Mirror::MATANIM,
    );
    out
}

/// `GfxRenderSystems::Pack`: writes what changed into the shared buffer's data texture, the
/// queued off-world blobs into theirs, and the mirrored regions into every mirror's, as the
/// render-world uploads write the shared buffer and each mirror.
pub(crate) fn pack(world: &mut World, mut seen: Local<Seen>) {
    let Some(id) = world.get_resource::<SharedLightBuffer>().map(|b| b.0) else {
        return;
    };
    let blobs = BLOB_WRITES
        .lock()
        .map(|mut w| std::mem::take(&mut *w))
        .unwrap_or_default();
    let mirrors = mirrors(world);
    world.resource_scope(|world, mut textures: Mut<GfxDataTextures>| {
        let b = bases();
        let seen = &mut *seen;
        let changed = pack_shared(world, light_texture(&mut textures, id, &b), &b, seen);

        for (buffer, at, rows) in blobs {
            light_texture(&mut textures, buffer, &b).write(at, &rows);
        }

        // A mirror takes its regions whole on first sight, then what the shared pack rewrote;
        // a superset of the render world's mirrored ranges, which only the booth rigs read.
        for (mirror, kinds) in mirrors {
            let had = seen.mirrors.get(&mirror).copied().unwrap_or(0);
            seen.mirrors.insert(mirror, kinds);
            let fresh = kinds & !had;
            let mut ranges: Vec<Range<usize>> = Vec::new();
            let mut region = |kind: u8, dirty: bool, range: Range<usize>| {
                if kinds & kind != 0 && (fresh & kind != 0 || dirty) {
                    ranges.push(range);
                }
            };
            region(Mirror::RIG, changed.table, b.rig_table..b.rig_tint);
            region(Mirror::RIG, changed.origin, b.rig_origin..b.matanim);
            region(Mirror::TINT, changed.tint, b.rig_tint..b.rig_origin);
            region(Mirror::MATANIM, changed.matanim, b.matanim..b.water_clip);
            if fresh & Mirror::RIG != 0 {
                ranges.push(b.palette..b.end);
            } else if kinds & Mirror::RIG != 0 {
                ranges.extend(changed.palette.iter().cloned());
            }
            for range in ranges {
                let Some(rows) = textures.get(id).and_then(|t| t.rows().get(range.clone())) else {
                    continue;
                };
                let rows = rows.to_vec();
                light_texture(&mut textures, mirror, &b).write(range.start, &rows);
            }
        }
    });
}

/// Buffer `id`'s texture, made when it has none as the wgpu buffer is made, zeroed: its tint
/// region reads the identity, -1 here ([`tint_value`]), since a buffer no tint mirror fills (the
/// portrait booths') keeps it so, where a zero row would tint every rig on it black.
fn light_texture<'a>(
    textures: &'a mut GfxDataTextures,
    id: BufferId,
    b: &Bases,
) -> &'a mut DataTexture {
    if textures.get(id).is_none() {
        let identity = tint_value(crate::instance_tint::IDENTITY);
        textures
            .get_or_insert(id, b.end)
            .write(b.rig_tint, &vec![[identity; 4]; b.rig_origin - b.rig_tint]);
    }
    textures.get_or_insert(id, b.end)
}

/// The shared buffer's writes, each gated as its upload system is.
fn pack_shared(world: &World, tex: &mut DataTexture, b: &Bases, seen: &mut Seen) -> Changed {
    let mut changed = Changed::default();
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
            changed.table = true;
        }
        if seen.rig_origin != Some(p.origin_generation) && !p.origins.is_empty() {
            seen.rig_origin = Some(p.origin_generation);
            tex.write(b.rig_origin, p.origins);
            changed.origin = true;
        }
        if !seen.palette_primed && !p.rows.is_empty() {
            // The first sight writes every row; after it only the published dirty ranges.
            seen.palette_primed = true;
            seen.palette_dirty = p.dirty_id;
            tex.write(b.palette, p.rows);
            changed.palette.push(b.palette..b.palette + p.rows.len());
        } else if seen.palette_dirty != p.dirty_id && !p.dirty.is_empty() {
            seen.palette_dirty = p.dirty_id;
            for &(base, len, _) in p.dirty {
                let (lo, hi) = (3 * base as usize, 3 * (base + len) as usize);
                if let Some(rows) = p.rows.get(lo..hi) {
                    tex.write(b.palette + lo, rows);
                    changed.palette.push(b.palette + lo..b.palette + hi);
                }
            }
        }
    }

    if let Some(tints) = world.get_resource::<crate::instance_tint::InstanceTints>() {
        let (slots, generation) = tints.gfx_slots();
        if seen.tints != Some(generation) {
            seen.tints = Some(generation);
            tex.write(b.rig_tint, &pack4(slots.iter().map(|&w| tint_value(w))));
            changed.tint = true;
        }
    }

    if let Some(table) = world.get_resource::<crate::mat_anim_table::MatAnimTable>() {
        let (rows, generation) = table.gfx_rows();
        if seen.matanim != Some(generation) {
            seen.matanim = Some(generation);
            tex.write(b.matanim, rows);
            changed.matanim = true;
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
    changed
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
    fn the_static_and_terrain_shaders_read_the_same_regions() {
        let b = bases();
        for vs in [
            include_str!("../../../benilla-gfx/shaders/src/static_gx.vs.gfxs"),
            include_str!("../../../benilla-gfx/shaders/src/terrain.vs.gfxs"),
        ] {
            assert_eq!(define(vs, "WOW_POINTS_BASE"), b.points);
        }
        let fs = include_str!("../../../benilla-gfx/shaders/src/static_gx.fs.gfxs");
        assert_eq!(define(fs, "WOW_PROBES_BASE"), b.probes);
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
    fn a_new_light_texture_reads_the_identity_tint_and_zero_elsewhere() {
        let b = bases();
        let mut textures = GfxDataTextures::default();
        let id = BufferId::new();
        let tex = light_texture(&mut textures, id, &b);
        assert_eq!(tex.row(b.rig_tint), [-1.0; 4]);
        assert_eq!(tex.row(b.rig_origin - 1), [-1.0; 4]);
        assert_eq!(tex.row(b.rig_tint - 1), [0.0; 4]);
        assert_eq!(tex.row(b.rig_origin), [0.0; 4]);
        // A later write keeps its rows: the prefill runs once, when the texture is made.
        tex.write(b.rig_tint, &[[5.0; 4]]);
        assert_eq!(
            light_texture(&mut textures, id, &b).row(b.rig_tint),
            [5.0; 4]
        );
    }

    #[test]
    fn scalars_pack_four_a_row() {
        let rows = pack4([1.0, 2.0, 3.0, 4.0, 5.0].into_iter());
        assert_eq!(rows, vec![[1.0, 2.0, 3.0, 4.0], [5.0, 0.0, 0.0, 0.0]]);
    }
}
