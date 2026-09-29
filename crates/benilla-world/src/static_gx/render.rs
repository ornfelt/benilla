//! The retained pass's published half: the baked regions and this frame's visibility, which the
//! gfx renderer (`super::gfx`) draws for the world camera before its opaque phase.

use bevy::camera::primitives::Aabb;
use bevy::image::Image;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::render::extract_component::{ExtractComponent, ExtractComponentPlugin};
use std::ops::Range;

/// One baked item's draw facts, in bake order: the vertex word's low bits index its record.
#[derive(Clone)]
pub(crate) struct GxItemDraw {
    pub index_range: Range<u32>,
    pub texture: Option<AssetId<Image>>,
    pub cutout: bool,
    pub two_sided: bool,
    #[allow(dead_code)] // bake-side bookkeeping; the node draws by index range alone
    pub vertex_range: Range<u32>,
    /// The selection key (a WMO group or a prop referrer set); `None` on cell items.
    pub group: Option<u16>,
    /// The authored batch order (the coplanar-MOBA clip-z nudge; 0 on cell items).
    pub order: u16,
    /// The MOMT SIDN night-glow colour (gamma bytes; zero on cell items).
    pub sidn: [u8; 3],
    /// The interior prop's SH-probe slot, record column w bits 1..=13; 0 elsewhere.
    pub slot: u16,
}

/// One baked cell or region, published by the main-world flush.
#[derive(Clone)]
pub(crate) struct GxCellDraw {
    pub mesh: Handle<Mesh>,
    /// The recentring origin: shader world = vertex + origin.
    pub origin: Vec3,
    /// Mesh-local bound (recentred); world bound = origin + this.
    pub aabb: Aabb,
    pub draws: Vec<GxItemDraw>,
    /// The exile kill bitmap, bit i dropping item i; all-zero on regions.
    pub killed: Vec<u64>,
    /// Bumped on every bitmap change; the render side re-syncs the kill column on a new one.
    pub killed_rev: u32,
    /// Mesh-local bounds per selection key (group or referrer set) for the cull; empty on cells.
    pub groups: Vec<(u16, Aabb)>,
    /// A prop region's distinct referrer sets, indexed like `groups`.
    pub sets: Vec<std::sync::Arc<[u16]>>,
}

/// Marks the world camera, the one view the pass draws into, so no portrait-booth view gets
/// world cells.
#[derive(Component, Clone, Copy, Default, ExtractComponent)]
pub(crate) struct StaticGxView;

/// Mark the world camera, again whenever it respawns.
fn mark_world_camera(
    mut commands: Commands,
    cam: Query<Entity, (With<crate::view::WorldCamera>, Without<StaticGxView>)>,
) {
    for e in &cam {
        commands.entity(e).insert(StaticGxView);
    }
}

/// One entry of the doodad-phase draw list: ADT-doodad cells and WMO-prop regions are one phase
/// in the 1.12 order (the M2 scene, after the WMOs), so they sort near-first together.
#[derive(Clone, PartialEq)]
pub(crate) enum GxDoodadVis {
    Cell((i32, i32)),
    Prop(Entity, GxSel),
}

/// One region's verdicts this frame, indexed by group (WMO) or referrer set (props).
#[derive(Clone, Default, PartialEq)]
pub(crate) struct GxSel {
    /// Drawn this frame: PVS ∧ frustum ∧ farclip ∧ the exterior window gate.
    pub drawn: Vec<bool>,
    /// On the interior fog lane: the client's per-group `[0xca7f00]`, from the portal flood
    /// ([`crate::wmo_portal::GroupPvs::interior_fog`]).
    pub fog: Vec<bool>,
}

/// The published half the gfx pass reads each frame. Regions sit behind `Arc`, so a publish is a
/// refcount bump and the kill scan, the one writer, copies a region on write.
#[derive(Clone, Default, Resource)]
pub(crate) struct GxWorld {
    pub cells: HashMap<(i32, i32), std::sync::Arc<GxCellDraw>>,
    /// This frame's doodad-phase draw list, near-first across cells and prop regions.
    pub visible: Vec<GxDoodadVis>,
    pub wmos: HashMap<Entity, std::sync::Arc<GxCellDraw>>,
    /// The prop regions, keyed like `wmos` but apart, so a prop arrival never re-bakes a building.
    pub props: HashMap<Entity, std::sync::Arc<GxCellDraw>>,
    /// This frame's per-group admission per WMO region; the node draws only admitted runs.
    pub visible_wmos: Vec<(Entity, GxSel)>,
}

/// Record column w, bit 14: the item's interior fog lane, read by `static_gx.wgsl`. Bit 0 is the
/// kill bit and bits 1..=13 the probe slot.
pub(super) const RECORD_FOG_BIT: u32 = 1 << 14;

/// One coalesced draw run of adjacent live items sharing bind-group slot, pipeline bucket and
/// selection key; a killed item is never in one.
pub(super) struct GxRun {
    /// Index into the region's `bind_groups`, not a pool class.
    pub(super) slot: usize,
    cutout: bool,
    two_sided: bool,
    pub(super) index_range: Range<u32>,
    /// The run's selection key; `None` for a cell run, which always draws.
    pub(super) group: Option<u16>,
}

/// Coalesce adjacent live items sharing (slot, bucket, group) into draw runs; a killed item is
/// skipped, so a fully gone cell submits nothing.
pub(super) fn build_runs(draws: &[GxItemDraw], item_slot: &[u16], killed: &[u64]) -> Vec<GxRun> {
    let mut runs: Vec<GxRun> = Vec::new();
    for (i, item) in draws.iter().enumerate() {
        if kill_bit(killed, i) != 0 {
            continue;
        }
        let slot = usize::from(item_slot[i]);
        match runs.last_mut() {
            Some(r)
                if r.slot == slot
                    && r.cutout == item.cutout
                    && r.two_sided == item.two_sided
                    && r.group == item.group
                    && r.index_range.end == item.index_range.start =>
            {
                r.index_range.end = item.index_range.end;
            }
            _ => runs.push(GxRun {
                slot,
                cutout: item.cutout,
                two_sided: item.two_sided,
                index_range: item.index_range.clone(),
                group: item.group,
            }),
        }
    }
    runs
}

/// Record-table column 3: item `i`'s exile kill bit from the published bitmap.
pub(super) fn kill_bit(killed: &[u64], i: usize) -> u32 {
    u32::from(
        killed
            .get(i / 64)
            .is_some_and(|w| w & (1u64 << (i % 64)) != 0),
    )
}

/// Wire the draw half (called by the plugin only when armed).
pub(super) fn build(app: &mut App) {
    super::gfx::build(app);
    // `publish_gx_world` mirrors `StaticGx`'s published half into this resource.
    app.add_plugins(ExtractComponentPlugin::<StaticGxView>::default());
    app.init_resource::<GxWorld>();
    app.add_systems(Update, mark_world_camera);
}

/// Copy the collector's published half into [`GxWorld`].
pub(super) fn publish_gx_world(gx: Res<super::StaticGx>, mut out: ResMut<GxWorld>) {
    let _t = super::gx_perf_guard(2);
    // Write only what changed, so the resource reads as changed only then. The maps hold `Arc`s,
    // so identity is pointer identity.
    fn same_arcs<K: std::hash::Hash + Eq>(
        a: &HashMap<K, std::sync::Arc<GxCellDraw>>,
        b: &HashMap<K, std::sync::Arc<GxCellDraw>>,
    ) -> bool {
        a.len() == b.len()
            && a.iter()
                .all(|(k, v)| b.get(k).is_some_and(|w| std::sync::Arc::ptr_eq(v, w)))
    }
    let w = &gx.world;
    if !same_arcs(&out.cells, &w.cells) {
        out.cells.clone_from(&w.cells);
    }
    if out.visible != w.visible {
        out.visible.clone_from(&w.visible);
    }
    if !same_arcs(&out.wmos, &w.wmos) {
        out.wmos.clone_from(&w.wmos);
    }
    if !same_arcs(&out.props, &w.props) {
        out.props.clone_from(&w.props);
    }
    if out.visible_wmos != w.visible_wmos {
        out.visible_wmos.clone_from(&w.visible_wmos);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(index_range: Range<u32>, cutout: bool) -> GxItemDraw {
        GxItemDraw {
            index_range,
            texture: None,
            cutout,
            two_sided: false,
            vertex_range: 0..0,
            group: None,
            order: 0,
            sidn: [0; 3],
            slot: 0,
        }
    }

    #[test]
    fn runs_fuse_live_items_and_split_at_kills() {
        let draws = vec![
            item(0..3, false),
            item(3..6, false),
            item(6..9, false),
            item(9..12, true), // bucket change
            item(12..15, true),
        ];
        let slots = vec![0, 0, 0, 0, 1]; // the last item binds another pool class
        let runs = build_runs(&draws, &slots, &[]);
        assert_eq!(runs.len(), 3, "opaque span fused; cutout split by slot");
        assert_eq!(runs[0].index_range, 0..9);
        assert_eq!(runs[1].index_range, 9..12);
        assert!(runs[1].cutout);
        assert_eq!(runs[2].slot, 1);
        // Kill the middle opaque item: the fused run splits around it.
        let runs = build_runs(&draws, &slots, &[0b010u64]);
        assert_eq!(runs.len(), 4);
        assert_eq!(runs[0].index_range, 0..3);
        assert_eq!(runs[1].index_range, 6..9);
        // Kill everything: nothing is submitted.
        assert!(build_runs(&draws, &slots, &[0b11111u64]).is_empty());
    }
}
