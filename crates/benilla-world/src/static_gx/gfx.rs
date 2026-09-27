//! The retained static-world pass on the gfx renderer (`benilla-gfx`, the `gfx` feature): the
//! program `static_gx.{vs,fs}.gfxs`, a port of `static_gx.wgsl`, drawn for the world camera
//! before its opaque phase in the render node's order (WMO regions, then the doodad phase
//! near-first), each region's admitted runs in bake order.
//!
//! The wgpu side's texture-array pool is a wgpu economy (one bind group per pool class): here a
//! run binds its items' own image through the sampler the pool's pair would pick (repeat when
//! either axis wraps, else clamp), so runs split by texture as well. A region's record table is a
//! data texture of the same rows, written in `GfxRenderSystems::Pack` so a kill or fog change
//! reaches the device the frame it is published, as `prepare_static_gx`'s buffer writes do.

use bevy::mesh::VertexAttributeValues;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::render::render_resource::{BufferId, Face};

use benilla_gfx::draw::{DrawList, EarlyDraw};
use benilla_gfx::ffi::GfxFormat;
use benilla_gfx::images::GfxSampler;
use benilla_gfx::material::MAX_PARAMS;
use benilla_gfx::meshes::VertexInput;
use benilla_gfx::{
    GfxAlpha, GfxDataTextures, GfxDrawState, GfxMaterialDesc, GfxProgram, GfxRender,
    GfxRenderSystems, GfxTextureSlot,
};

use super::render::{
    build_runs, kill_bit, GxCellDraw, GxDoodadVis, GxItemDraw, GxRun, GxSel, GxWorld, StaticGxView,
    RECORD_FOG_BIT,
};
use super::{ATTRIBUTE_GX_ANCHOR, ATTRIBUTE_GX_WORD, WORD_WRAP_X, WORD_WRAP_Y};
use crate::lighting::SharedLightBuffer;

/// The program's inputs, in `static_gx.vs.gfxs` order; the word is read as the `int` both shader
/// languages have (its bits stop at 28).
pub const STATIC_GX: GfxProgram = GfxProgram {
    name: "static_gx",
    inputs: &[
        VertexInput::new(Mesh::ATTRIBUTE_POSITION, [0.0; 4]),
        VertexInput::new(Mesh::ATTRIBUTE_NORMAL, [0.0; 4]),
        VertexInput::new(Mesh::ATTRIBUTE_UV_0, [0.0; 4]),
        VertexInput::new(Mesh::ATTRIBUTE_COLOR, [1.0; 4]),
        VertexInput::new(ATTRIBUTE_GX_WORD, [0.0; 4]).read_as(GfxFormat::R32Sint),
        VertexInput::new(ATTRIBUTE_GX_ANCHOR, [0.0; 4]),
    ],
    params: 1,
    samplers: 3,
};

/// A region's gfx state, kept until its bake (mesh) changes.
struct GxRegionGfx {
    mesh: AssetId<Mesh>,
    /// The record table's data texture.
    records_id: BufferId,
    /// The rows as floats: x unused (the wgpu pool layer), y the batch order, z the SIDN word,
    /// w the flags; every value is exact in an `f32`.
    records: Vec<[f32; 4]>,
    /// One description per distinct (texture, sampler, cutout, two-sided), indexed by the runs'
    /// slot.
    descs: Vec<GfxMaterialDesc>,
    item_slot: Vec<u16>,
    runs: Vec<GxRun>,
    killed_applied: u32,
    fog_applied: Vec<bool>,
}

#[derive(Resource, Default)]
struct GxGfxCache {
    cells: HashMap<(i32, i32), GxRegionGfx>,
    wmos: HashMap<Entity, GxRegionGfx>,
    props: HashMap<Entity, GxRegionGfx>,
}

pub(super) fn build(app: &mut App) {
    app.init_resource::<GxGfxCache>().add_systems(
        GfxRender,
        (
            pack.in_set(GfxRenderSystems::Pack),
            collect.in_set(GfxRenderSystems::Collect),
        ),
    );
}

/// `prepare_static_gx`'s half: assembles the visible regions not yet assembled and syncs the kill
/// and fog columns of their record tables.
fn pack(
    gx: Res<GxWorld>,
    mut cache: ResMut<GxGfxCache>,
    mut textures: ResMut<GfxDataTextures>,
    meshes: Res<Assets<Mesh>>,
    images: Res<Assets<Image>>,
    light: Option<Res<SharedLightBuffer>>,
) {
    let cache = &mut *cache;
    // Every published map empty is a map change (`StaticGx::clear`).
    if gx.cells.is_empty() && gx.wmos.is_empty() && gx.props.is_empty() {
        for r in cache
            .cells
            .drain()
            .map(|(_, r)| r)
            .chain(cache.wmos.drain().map(|(_, r)| r))
            .chain(cache.props.drain().map(|(_, r)| r))
        {
            textures.remove(r.records_id);
        }
        return;
    }
    let Some(light) = light else {
        return;
    };
    let light = light.0.id();

    // Drop the regions that vanished or re-baked, with their record tables.
    let mut drop_gone = |gone: bool, r: &GxRegionGfx| {
        if gone {
            textures.remove(r.records_id);
        }
        !gone
    };
    cache
        .cells
        .retain(|c, r| drop_gone(!gx.cells.get(c).is_some_and(|d| d.mesh.id() == r.mesh), r));
    cache
        .wmos
        .retain(|e, r| drop_gone(!gx.wmos.get(e).is_some_and(|d| d.mesh.id() == r.mesh), r));
    cache
        .props
        .retain(|e, r| drop_gone(!gx.props.get(e).is_some_and(|d| d.mesh.id() == r.mesh), r));

    let mut assemble = |draw: &GxCellDraw| {
        let region = assemble_region(draw, &meshes, &images, light)?;
        textures
            .get_or_insert(region.records_id, region.records.len())
            .write(0, &region.records);
        Some(region)
    };
    for vis in &gx.visible {
        match vis {
            GxDoodadVis::Cell(cell) => {
                if !cache.cells.contains_key(cell) {
                    if let Some(r) = gx.cells.get(cell).and_then(|d| assemble(d)) {
                        cache.cells.insert(*cell, r);
                    }
                }
            }
            GxDoodadVis::Prop(entity, _) => {
                if !cache.props.contains_key(entity) {
                    if let Some(r) = gx.props.get(entity).and_then(|d| assemble(d)) {
                        cache.props.insert(*entity, r);
                    }
                }
            }
        }
    }
    for (entity, _) in &gx.visible_wmos {
        if !cache.wmos.contains_key(entity) {
            if let Some(r) = gx.wmos.get(entity).and_then(|d| assemble(d)) {
                cache.wmos.insert(*entity, r);
            }
        }
    }

    // The kill-bit sync, on a new bitmap revision; a cell that changed out of view syncs on
    // re-entry.
    for vis in &gx.visible {
        let GxDoodadVis::Cell(cell) = vis else {
            continue;
        };
        let (Some(r), Some(draw)) = (cache.cells.get_mut(cell), gx.cells.get(cell)) else {
            continue;
        };
        if r.killed_applied == draw.killed_rev {
            continue;
        }
        for (i, rec) in r.records.iter_mut().enumerate() {
            let flags = rec[3] as u32;
            rec[3] = ((flags & !1) | kill_bit(&draw.killed, i)) as f32;
        }
        textures
            .get_or_insert(r.records_id, r.records.len())
            .write(0, &r.records);
        r.runs = build_runs(&draw.draws, &r.item_slot, &draw.killed);
        r.killed_applied = draw.killed_rev;
    }

    // The interior-fog sync: the client's per-group `[0xca7f00]`, rewritten when it moves.
    let mut sync_fog = |r: &mut GxRegionGfx, draw: &GxCellDraw, sel: &GxSel| {
        if r.fog_applied == sel.fog {
            return;
        }
        for (rec, item) in r.records.iter_mut().zip(&draw.draws) {
            let on = item
                .group
                .is_some_and(|g| sel.fog.get(usize::from(g)).copied().unwrap_or(false));
            let flags = rec[3] as u32;
            rec[3] = ((flags & !RECORD_FOG_BIT) | (u32::from(on) * RECORD_FOG_BIT)) as f32;
        }
        textures
            .get_or_insert(r.records_id, r.records.len())
            .write(0, &r.records);
        r.fog_applied.clone_from(&sel.fog);
    };
    for (entity, sel) in &gx.visible_wmos {
        if let (Some(r), Some(draw)) = (cache.wmos.get_mut(entity), gx.wmos.get(entity)) {
            sync_fog(r, draw, sel);
        }
    }
    for vis in &gx.visible {
        let GxDoodadVis::Prop(entity, sel) = vis else {
            continue;
        };
        if let (Some(r), Some(draw)) = (cache.props.get_mut(entity), gx.props.get(entity)) {
            sync_fog(r, draw, sel);
        }
    }
}

/// One region's gfx state; `None`, and undrawn this frame, while its mesh or any member texture is
/// not loaded.
fn assemble_region(
    draw: &GxCellDraw,
    meshes: &Assets<Mesh>,
    images: &Assets<Image>,
    light: BufferId,
) -> Option<GxRegionGfx> {
    let mesh = meshes.get(draw.mesh.id())?;
    let Some(VertexAttributeValues::Uint32(words)) = mesh.attribute(ATTRIBUTE_GX_WORD) else {
        return None;
    };
    // The pool samplers' filter: `benilla_assets::tex_filter`, as `init_pipelines` makes them.
    let filter = benilla_assets::tex_filter();
    let mipmap_linear = filter.mipmap_filter() == bevy::image::ImageFilterMode::Linear;
    let anisotropy = filter.anisotropy_clamp();
    let records_id = BufferId::new();

    let mut descs: Vec<GfxMaterialDesc> = Vec::new();
    let mut item_slot: Vec<u16> = Vec::with_capacity(draw.draws.len());
    for item in &draw.draws {
        let word = *words.get(item.vertex_range.start as usize)?;
        let wraps = word & (WORD_WRAP_X | WORD_WRAP_Y) != 0;
        let (texture, size) = match item.texture {
            Some(id) => {
                let image = images.get(id)?;
                let sampler = GfxSampler {
                    repeat: [wraps; 2],
                    mipmap_linear,
                    anisotropy,
                };
                (
                    GfxTextureSlot::ImageSampled(id, sampler),
                    image.size().as_vec2(),
                )
            }
            None => (GfxTextureSlot::White, Vec2::ONE),
        };
        let mut params = [[0.0; 4]; MAX_PARAMS];
        params[0] = [size.x, size.y, if item.cutout { 1.0 } else { 0.0 }, 0.0];
        let desc = GfxMaterialDesc {
            program: STATIC_GX,
            textures: [
                texture,
                GfxTextureSlot::Data(light),
                GfxTextureSlot::Data(records_id),
                GfxTextureSlot::White,
            ],
            params,
            alpha: GfxAlpha::Opaque,
            cull: (!item.two_sided).then_some(Face::Back),
            state: GfxDrawState::default(),
        };
        let slot = match descs.iter().position(|d| *d == desc) {
            Some(s) => s,
            None => {
                descs.push(desc);
                descs.len() - 1
            }
        };
        item_slot.push(u16::try_from(slot).expect("gx region under u16 slots"));
    }
    let records = draw
        .draws
        .iter()
        .enumerate()
        .map(|(i, item)| record_row(item, kill_bit(&draw.killed, i)))
        .collect();
    let runs = build_runs(&draw.draws, &item_slot, &draw.killed);
    Some(GxRegionGfx {
        mesh: draw.mesh.id(),
        records_id,
        records,
        descs,
        item_slot,
        runs,
        killed_applied: draw.killed_rev,
        fog_applied: Vec::new(),
    })
}

/// `assemble_region`'s record row as floats: [layer (unused), batch order, SIDN, flags]. Every
/// value stays below 2^24, so each is exact.
fn record_row(item: &GxItemDraw, kill: u32) -> [f32; 4] {
    [
        0.0,
        f32::from(item.order),
        (u32::from(item.sidn[0]) | (u32::from(item.sidn[1]) << 8) | (u32::from(item.sidn[2]) << 16))
            as f32,
        (kill | (u32::from(item.slot) << 1)) as f32,
    ]
}

/// `StaticGxNode::run`: the world camera's early draws, WMO regions first, then the doodad phase
/// near-first; a cell run always draws, a region's only when its selection key is admitted.
fn collect(
    gx: Res<GxWorld>,
    cache: Res<GxGfxCache>,
    mut list: ResMut<DrawList>,
    camera: Query<Entity, With<StaticGxView>>,
) {
    let Some(camera) = camera.iter().next() else {
        return;
    };
    let mut resolved: Vec<(&GxRegionGfx, &GxCellDraw, Option<&GxSel>)> = Vec::new();
    for (entity, sel) in &gx.visible_wmos {
        if let (Some(r), Some(draw)) = (cache.wmos.get(entity), gx.wmos.get(entity)) {
            resolved.push((r, draw, Some(sel)));
        }
    }
    for vis in &gx.visible {
        match vis {
            GxDoodadVis::Cell(cell) => {
                if let (Some(r), Some(draw)) = (cache.cells.get(cell), gx.cells.get(cell)) {
                    resolved.push((r, draw, None));
                }
            }
            GxDoodadVis::Prop(entity, sel) => {
                if let (Some(r), Some(draw)) = (cache.props.get(entity), gx.props.get(entity)) {
                    resolved.push((r, draw, Some(sel)));
                }
            }
        }
    }
    for (region, draw, sel) in resolved {
        let world_from_local = Mat4::from_translation(draw.origin);
        let mut descs: Vec<Option<u32>> = vec![None; region.descs.len()];
        for run in &region.runs {
            if let (Some(sel), Some(group)) = (sel, run.group) {
                if !sel.drawn.get(usize::from(group)).copied().unwrap_or(false) {
                    continue;
                }
            }
            let desc = *descs[run.slot]
                .get_or_insert_with(|| list.push_desc(region.descs[run.slot].clone()));
            list.push_early(
                camera,
                EarlyDraw {
                    mesh: region.mesh,
                    indices: run.index_range.clone(),
                    world_from_local,
                    desc,
                },
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_rows_carry_every_bit_exactly() {
        let item = GxItemDraw {
            index_range: 0..3,
            texture: None,
            cutout: false,
            two_sided: false,
            vertex_range: 0..3,
            group: Some(1),
            order: u16::MAX,
            sidn: [255, 255, 255],
            slot: 0x1fff,
        };
        let row = record_row(&item, 1);
        assert_eq!(row[1] as u32, u32::from(u16::MAX));
        assert_eq!(row[2] as u32, 0x00ff_ffff);
        // The fog bit the per-frame sync adds on top of the kill bit and the probe slot.
        let flags = row[3] as u32 | RECORD_FOG_BIT;
        assert_eq!((flags as f32) as u32, 0x7fff);
    }
}
