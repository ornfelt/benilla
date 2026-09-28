//! bevy_gizmos' immediate-mode lines on the gfx renderer: bevy_gizmos_render 0.18.1's 3D line
//! pipeline (`pipeline_3d.rs`, `lines.wgsl`) through the program `gizmo_line.{vs,fs}.gfxs`. Each
//! config group's [`GizmoAsset`] (built in the main world's `Last` by `update_gizmo_meshes`) is
//! expanded into one quad per segment, in one mesh rewritten in place, and drawn in every active
//! 3D camera whose layers meet the group's, in its transparent phase at distance 0, list then
//! strip, as `queue_line_gizmos_3d` adds its `Transparent3d` items. Solid lines only: dotted and
//! dashed styles, joints, retained `Gizmo` components and 2D views are not drawn (benilla uses
//! none) and each logs once.

use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::RenderLayers;
use bevy::gizmos::config::{GizmoConfigStore, GizmoLineJoint, GizmoLineStyle};
use bevy::gizmos::{GizmoAsset, GizmoHandles};
use bevy::mesh::{Indices, MeshVertexAttribute, PrimitiveTopology, VertexFormat};
use bevy::prelude::*;

use crate::draw::{DrawList, GfxRenderer, SortedDraw};
use crate::material::MAX_PARAMS;
use crate::material::{GfxAlpha, GfxDrawState, GfxMaterialDesc, GfxProgram, GfxTextureSlot};
use crate::meshes::VertexInput;
use crate::pipelines::Blend;

/// The segment's far end (`lines.wgsl`'s `position_b`); `ATTRIBUTE_POSITION` is `position_a`.
pub const ATTRIBUTE_GIZMO_B: MeshVertexAttribute =
    MeshVertexAttribute::new("GfxGizmo_B", 988_301, VertexFormat::Float32x3);
/// The far end's colour (`color_b`); `ATTRIBUTE_COLOR` is `color_a`.
pub const ATTRIBUTE_GIZMO_COLOR_B: MeshVertexAttribute =
    MeshVertexAttribute::new("GfxGizmo_ColorB", 988_302, VertexFormat::Float32x4);
/// The quad corner the WGSL picks by `vertex_index`: x across the line (-0.5, 0.5), y along it
/// (0 at a, 1 at b).
pub const ATTRIBUTE_GIZMO_CORNER: MeshVertexAttribute =
    MeshVertexAttribute::new("GfxGizmo_Corner", 988_303, VertexFormat::Float32x2);

pub const GIZMO_LINE: GfxProgram = GfxProgram {
    name: "gizmo_line",
    inputs: &[
        VertexInput::new(Mesh::ATTRIBUTE_POSITION, [0.0; 4]),
        VertexInput::new(ATTRIBUTE_GIZMO_B, [0.0; 4]),
        VertexInput::new(Mesh::ATTRIBUTE_COLOR, [1.0; 4]),
        VertexInput::new(ATTRIBUTE_GIZMO_COLOR_B, [1.0; 4]),
        VertexInput::new(ATTRIBUTE_GIZMO_CORNER, [0.0; 4]),
    ],
    params: 1,
    samplers: 0,
};

/// `lines.wgsl`'s six corners, as a quad of four vertices and two triangles in the same winding.
const CORNERS: [[f32; 2]; 4] = [[-0.5, 0.0], [-0.5, 1.0], [0.5, 1.0], [0.5, 0.0]];
const QUAD: [u32; 6] = [0, 1, 2, 0, 2, 3];

/// The mesh the frame's segments are written into and the vertex capacity its streams are padded
/// to (a power of two, so the device buffers keep their size across frames).
#[derive(Resource)]
pub(crate) struct GfxGizmoStream {
    mesh: Handle<Mesh>,
    vertices: usize,
}

/// The frame's segment streams.
#[derive(Default)]
struct Segments {
    a: Vec<[f32; 3]>,
    b: Vec<[f32; 3]>,
    color_a: Vec<[f32; 4]>,
    color_b: Vec<[f32; 4]>,
    corner: Vec<[f32; 2]>,
    indices: Vec<u32>,
}

impl Segments {
    /// One quad for the segment `a`-`b`; a segment with a non-finite end (the NaN that closes a
    /// strip) is skipped, as its NaN clip position drops it on the GPU.
    fn push(&mut self, (a, ca): (Vec3, LinearRgba), (b, cb): (Vec3, LinearRgba)) {
        if !a.is_finite() || !b.is_finite() {
            return;
        }
        let base = self.a.len() as u32;
        for corner in CORNERS {
            self.a.push(a.to_array());
            self.b.push(b.to_array());
            self.color_a.push(ca.to_f32_array());
            self.color_b.push(cb.to_f32_array());
            self.corner.push(corner);
        }
        self.indices.extend(QUAD.map(|i| base + i));
    }
}

pub(crate) fn init(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    let mut stream = GfxGizmoStream {
        mesh: Handle::default(),
        vertices: 256,
    };
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD,
    );
    fill(&mut mesh, &mut stream, &Segments::default());
    stream.mesh = meshes.add(mesh);
    commands.insert_resource(stream);
}

/// Writes the padded streams into `mesh`, growing the capacity to fit first.
fn fill(mesh: &mut Mesh, stream: &mut GfxGizmoStream, s: &Segments) {
    stream.vertices = stream.vertices.max(s.a.len().next_power_of_two());
    let (n, indices) = (stream.vertices, stream.vertices / 4 * 6);
    let padded = |v: &[[f32; 4]]| {
        let mut v = v.to_vec();
        v.resize(n, [0.0; 4]);
        v
    };
    let mut a = s.a.clone();
    a.resize(n, [0.0; 3]);
    let mut b = s.b.clone();
    b.resize(n, [0.0; 3]);
    let mut corner = s.corner.clone();
    corner.resize(n, [0.0; 2]);
    let mut i = s.indices.clone();
    i.resize(indices, 0);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, a);
    mesh.insert_attribute(ATTRIBUTE_GIZMO_B, b);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, padded(&s.color_a));
    mesh.insert_attribute(ATTRIBUTE_GIZMO_COLOR_B, padded(&s.color_b));
    mesh.insert_attribute(ATTRIBUTE_GIZMO_CORNER, corner);
    mesh.insert_indices(Indices::U32(i));
}

/// Writes this frame's gizmo segments and places one sorted draw per config group and camera.
#[allow(clippy::too_many_arguments)]
pub(crate) fn collect(
    handles: Option<Res<GizmoHandles>>,
    store: Option<Res<GizmoConfigStore>>,
    assets: Option<Res<Assets<GizmoAsset>>>,
    retained: Query<(), With<bevy::gizmos::retained::Gizmo>>,
    cameras: Query<(Entity, &Camera, &GlobalTransform, Option<&RenderLayers>), With<Camera3d>>,
    mut stream: ResMut<GfxGizmoStream>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut renderer: NonSendMut<GfxRenderer>,
    mut list: ResMut<DrawList>,
    mut warned: Local<bool>,
) {
    let (Some(handles), Some(store), Some(assets)) = (handles, store, assets) else {
        return;
    };
    let mut segments = Segments::default();
    // Per group with anything to draw: its config and index range, in `GizmoHandles` order.
    let mut groups = Vec::new();
    for (ty, handle) in handles.handles() {
        let Some((config, _)) = store.get_config_dyn(ty) else {
            continue;
        };
        let Some(gizmo) = handle.as_ref().and_then(|h| assets.get(h)) else {
            continue;
        };
        if !config.enabled {
            continue;
        }
        let unported = !matches!(config.line.style, GizmoLineStyle::Solid)
            || !matches!(config.line.joints, GizmoLineJoint::None);
        if unported && !*warned {
            *warned = true;
            warn!("gfx: a gizmo line style or joint other than solid and none draws as solid");
        }
        let buffer = gizmo.buffer();
        let start = segments.indices.len() as u32;
        let list_points = buffer.list_positions.iter().zip(&buffer.list_colors);
        let list_points: Vec<_> = list_points.map(|(p, c)| (*p, *c)).collect();
        for [a, b] in list_points.as_chunks::<2>().0 {
            segments.push(*a, *b);
        }
        let strip: Vec<_> = buffer
            .strip_positions
            .iter()
            .zip(&buffer.strip_colors)
            .map(|(p, c)| (*p, *c))
            .collect();
        for pair in strip.windows(2) {
            segments.push(pair[0], pair[1]);
        }
        let end = segments.indices.len() as u32;
        if end > start {
            groups.push((config.clone(), start..end));
        }
    }
    if !retained.is_empty() && !*warned {
        *warned = true;
        warn!("gfx: retained Gizmo components are not drawn");
    }
    if groups.is_empty() {
        return;
    }

    let id = stream.mesh.id();
    let Some(mesh) = meshes.get_mut_untracked(id) else {
        return;
    };
    fill(mesh, &mut stream, &segments);
    // Untracked, so the device copy is marked here, before this frame's draw rewrites it.
    renderer.meshes.modified(id);

    for (config, indices) in groups {
        let mut params = [[0.0; 4]; MAX_PARAMS];
        params[0] = [
            config.line.width,
            config.depth_bias,
            if config.line.perspective { 1.0 } else { 0.0 },
            0.0,
        ];
        // `LineGizmoPipeline::specialize`: alpha blending, depth writes, reverse-Z `Greater`,
        // no culling.
        let desc = list.push_desc(GfxMaterialDesc {
            program: GIZMO_LINE,
            textures: [GfxTextureSlot::White; 4],
            params,
            alpha: GfxAlpha::Blend,
            cull: None,
            state: GfxDrawState {
                blend: Some(Blend::Alpha),
                depth_write: Some(true),
                depth_strict: true,
                ..default()
            },
        });
        for (camera, cam, transform, layers) in &cameras {
            if !cam.is_active || !config.render_layers.intersects(layers.unwrap_or_default()) {
                continue;
            }
            // `distance: 0.`: the camera's own point is at view z 0.
            list.push_sorted(
                camera,
                SortedDraw {
                    entity: Entity::PLACEHOLDER,
                    mesh: id,
                    indices: indices.clone(),
                    world_from_local: Mat4::IDENTITY,
                    desc,
                    anchor: transform.translation(),
                    bias: 0.0,
                },
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A strip's closing NaN makes no quad, and each real segment makes one.
    #[test]
    fn a_strip_closes_on_its_nan() {
        let c = LinearRgba::WHITE;
        let mut s = Segments::default();
        let strip = [
            (Vec3::ZERO, c),
            (Vec3::X, c),
            (Vec3::Y, c),
            (Vec3::NAN, LinearRgba::NAN),
        ];
        for pair in strip.windows(2) {
            s.push(pair[0], pair[1]);
        }
        assert_eq!(s.a.len(), 8);
        assert_eq!(s.indices, vec![0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7]);
    }
}
