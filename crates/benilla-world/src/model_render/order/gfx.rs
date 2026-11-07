//! The M2 transparent order on the gfx renderer: gfx has no render world, so the links and material
//! sorts [`super::order_m2_transparents`] reads there are gathered from the main world each frame,
//! and the same re-sort runs in the gfx draw, after its depth sort
//! (`benilla_gfx::draw::TransparentResort`).

use bevy::ecs::entity::EntityHashMap;
use bevy::prelude::*;
use bevy::render::extract_component::ExtractComponent;

use benilla_assets::materials::WowModelMaterial;
use benilla_gfx::draw::{DrawList, TransparentResort};
use benilla_gfx::{GfxRender, GfxRenderSystems};

use super::{m2_cmp, m2_keys, merge, M2SortLink, MaterialSort, RenderSortLink};

/// This frame's M2 batches: each one's link and its material's sort facts.
struct M2Order(EntityHashMap<(RenderSortLink, Option<MaterialSort>)>);

impl TransparentResort for M2Order {
    fn resort(
        &self,
        eye: Vec3,
        view_from_world: &Mat4,
        items: &[(f32, Entity)],
    ) -> Option<Vec<(usize, f32)>> {
        let mut others = Vec::with_capacity(items.len());
        let mut m2 = Vec::new();
        for (i, &(distance, entity)) in items.iter().enumerate() {
            let keys = self.0.get(&entity).and_then(|(link, sort)| {
                m2_keys(
                    link,
                    sort.as_ref(),
                    |p| view_from_world.transform_point3(p).z,
                    eye,
                )
            });
            match keys {
                Some(keys) => m2.push((keys, (i, keys.merge_depth))),
                None => others.push((distance, (i, distance))),
            }
        }
        if m2.is_empty() {
            return None;
        }
        m2.sort_by(|a, b| m2_cmp(&a.0, &b.0));
        let m2 = m2
            .into_iter()
            .map(|(keys, item)| (keys.merge_depth, item))
            .collect();
        Some(merge(others, m2))
    }
}

/// `extract_material_sorts`, for one material.
fn material_sort(m: &WowModelMaterial) -> MaterialSort {
    let twin = (m.extension.clutter_fade.z as u16) & super::super::ZFILL_MARKER != 0;
    let own = if twin {
        super::super::ZFILL_SORT_BIAS
    } else {
        0.0
    };
    MaterialSort {
        order: m.extension.order,
        rung: m.base.depth_bias - own,
    }
}

/// Gathers every M2 batch's link and material sort into the draw list's re-sort.
fn collect(
    mut list: ResMut<DrawList>,
    links: Query<(Entity, &M2SortLink, &MeshMaterial3d<WowModelMaterial>)>,
    materials: Res<Assets<WowModelMaterial>>,
) {
    let mut batches = EntityHashMap::default();
    for (entity, link, material) in &links {
        let Some(link) = M2SortLink::extract_component(link) else {
            continue;
        };
        batches.insert(
            entity,
            (link, materials.get(&material.0).map(material_sort)),
        );
    }
    list.resort = Some(Box::new(M2Order(batches)));
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(GfxRender, collect.in_set(GfxRenderSystems::Collect));
}
