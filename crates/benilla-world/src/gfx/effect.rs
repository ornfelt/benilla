//! The effect lane (`particles::render`) on the gfx renderer: the program `effect.{vs,fs}.gfxs`, a
//! port of `wow_effect.wgsl`. The frame's [`EffectQuads`] stream is rebased and written into one
//! mesh, rewritten in place on the device (its streams padded to a power of two, so their sizes
//! hold), and each draw record goes into its camera's transparent phase at its anchor's view z
//! plus its rung, as `queue_effects` adds its `Transparent3d` items.

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;

use benilla_gfx::draw::{DrawList, GfxRenderer, SortedDraw};
use benilla_gfx::material::MAX_PARAMS;
use benilla_gfx::meshes::VertexInput;
use benilla_gfx::pipelines::Blend;
use benilla_gfx::{GfxAlpha, GfxDrawState, GfxMaterialDesc, GfxProgram, GfxTextureSlot};

use crate::lighting::SharedLightBuffer;
use crate::particles::buffer::{EffectBlend, EffectQuads, EffectTopology};

pub const EFFECT: GfxProgram = GfxProgram {
    name: "effect",
    inputs: &[
        VertexInput::new(Mesh::ATTRIBUTE_POSITION, [0.0; 4]),
        VertexInput::new(Mesh::ATTRIBUTE_UV_0, [0.0; 4]),
        VertexInput::new(Mesh::ATTRIBUTE_COLOR, [1.0; 4]),
    ],
    params: 3,
    samplers: 2,
};

/// The mesh the stream is written into, and the capacities its streams are padded to.
#[derive(Resource)]
pub(crate) struct GfxEffectStream {
    mesh: Handle<Mesh>,
    vertices: usize,
    indices: usize,
}

pub(crate) fn init(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    let mut stream = GfxEffectStream {
        mesh: Handle::default(),
        vertices: 1024,
        indices: 1536,
    };
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD,
    );
    fill(&mut mesh, &mut stream, &[], &[], &[], &[]);
    stream.mesh = meshes.add(mesh);
    commands.insert_resource(stream);
}

/// Writes the padded streams into `mesh`, growing the capacities to fit first.
fn fill(
    mesh: &mut Mesh,
    stream: &mut GfxEffectStream,
    positions: &[[f32; 3]],
    uvs: &[[f32; 2]],
    colors: &[[f32; 4]],
    indices: &[u32],
) {
    stream.vertices = stream.vertices.max(positions.len().next_power_of_two());
    stream.indices = stream.indices.max(indices.len().next_power_of_two());
    let pad = |n: usize| stream.vertices - n;
    let mut p = positions.to_vec();
    p.extend(std::iter::repeat_n([0.0; 3], pad(positions.len())));
    let mut u = uvs.to_vec();
    u.extend(std::iter::repeat_n([0.0; 2], pad(uvs.len())));
    let mut c = colors.to_vec();
    c.extend(std::iter::repeat_n([0.0; 4], pad(colors.len())));
    let mut i = indices.to_vec();
    i.resize(stream.indices, 0);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, p);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, u);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, c);
    mesh.insert_indices(Indices::U32(i));
}

/// The fog row of `wow_params` per policy, in [`crate::particles::buffer::EffectFog::slot`] order.
fn fog_row(slot: u32) -> [f32; 4] {
    match slot {
        5 => [
            0.0,
            1.0,
            crate::weather::RAIN_FOG_START,
            crate::weather::RAIN_FOG_END,
        ],
        s => [s as f32, 0.0, 0.0, 0.0],
    }
}

/// The blend number the shader branches on, the blend state and the depth write of each lane
/// blend (`EffectPipeline::specialize`).
fn blend_state(blend: EffectBlend) -> (f32, Blend, bool) {
    match blend {
        EffectBlend::Add => (0.0, Blend::Premultiplied, false),
        EffectBlend::Alpha => (1.0, Blend::Alpha, false),
        EffectBlend::Opaque => (2.0, Blend::Replace, true),
        EffectBlend::AlphaKey => (3.0, Blend::Replace, true),
        EffectBlend::Multiply => (4.0, Blend::Multiply, false),
        EffectBlend::Mod2x => (5.0, Blend::Modulate2x, false),
    }
}

/// Rebases and writes this frame's stream, then places one sorted draw per record.
#[allow(clippy::too_many_arguments)]
pub(crate) fn collect(
    quads: Option<Res<EffectQuads>>,
    light: Option<Res<SharedLightBuffer>>,
    mut stream: ResMut<GfxEffectStream>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut renderer: NonSendMut<GfxRenderer>,
    mut list: ResMut<DrawList>,
    cameras: Query<&GlobalTransform, With<Camera>>,
) {
    let (Some(quads), Some(light)) = (quads, light) else {
        return;
    };
    // `FPS_PROBE`'s `fx=`: gfx merges no runs, so each record is one draw.
    let n = quads.draws.len() as u32;
    for stat in &crate::particles::render::EFFECT_DRAW_STATS {
        stat.store(n, std::sync::atomic::Ordering::Relaxed);
    }
    if n == 0 {
        return;
    }
    // Camera-relative for f32 precision, as `prepare_effects` rebases: every draw but a coplanar
    // decal (absolute through `clip_from_world`) and one its producer wrote relative already.
    let mut positions: Vec<[f32; 3]> = quads.verts.iter().map(|v| v.pos).collect();
    let mut cams: HashMap<Entity, Vec3> = HashMap::default();
    for draw in &quads.draws {
        if draw.raster_bias != 0 || draw.cam_relative {
            continue;
        }
        let cam = *cams.entry(draw.cam).or_insert_with(|| {
            cameras
                .get(draw.cam)
                .map_or(Vec3::ZERO, |t| t.translation())
        });
        for p in &mut positions[draw.range.start as usize..draw.range.end as usize] {
            *p = (Vec3::from_array(*p) - cam).to_array();
        }
    }
    let uvs: Vec<[f32; 2]> = quads.verts.iter().map(|v| v.uv).collect();
    let colors: Vec<[f32; 4]> = quads.verts.iter().map(|v| v.color).collect();
    let mut indices: Vec<u32> = Vec::new();
    let mut ranges = Vec::with_capacity(quads.draws.len());
    for draw in &quads.draws {
        let start = indices.len() as u32;
        match draw.topology {
            EffectTopology::Quads => {
                for b in draw.range.clone().step_by(4) {
                    indices.extend_from_slice(&[b, b + 1, b + 2, b, b + 2, b + 3]);
                }
            }
            EffectTopology::Tris => indices.extend(draw.range.clone()),
        }
        ranges.push(start..indices.len() as u32);
    }

    let id = stream.mesh.id();
    let Some(mesh) = meshes.get_mut_untracked(id) else {
        return;
    };
    fill(mesh, &mut stream, &positions, &uvs, &colors, &indices);
    // Untracked, so the device copy is marked here, before this frame's draw rewrites it.
    renderer.meshes.modified(id);

    let light = light.0;
    // The lane's instruments, as `EffectPipeline::specialize` reads them.
    let no_depth = std::env::var_os("WOW_PARTICLE_NODEPTH").is_some();
    let flat = if std::env::var_os("WOW_PARTICLE_FLAT").is_some() {
        1.0
    } else {
        0.0
    };
    for (draw, indices) in quads.draws.iter().zip(ranges) {
        let (blend_number, blend, depth_write) = blend_state(draw.blend);
        let mut params = [[0.0; 4]; MAX_PARAMS];
        params[0] = fog_row(draw.fog.slot());
        params[1] = [
            blend_number,
            if draw.lit { 1.0 } else { 0.0 },
            if draw.raster_bias != 0 { 1.0 } else { 0.0 },
            flat,
        ];
        params[2] = draw.clip.map_or([0.0; 4], |c| c.to_array());
        let desc = list.push_desc(GfxMaterialDesc {
            program: EFFECT,
            // A booth's record reads the booth's own light buffer.
            textures: [
                GfxTextureSlot::Image(draw.texture),
                GfxTextureSlot::Data(draw.light.unwrap_or(light)),
                GfxTextureSlot::White,
                GfxTextureSlot::White,
            ],
            params,
            alpha: GfxAlpha::Blend,
            cull: None,
            state: GfxDrawState {
                blend: Some(blend),
                depth_write: Some(depth_write),
                depth_always: draw.no_depth_test || no_depth,
                raster_bias: draw.raster_bias,
                raster_slope: draw.raster_slope,
                ..default()
            },
        });
        list.push_sorted(
            draw.cam,
            SortedDraw {
                entity: draw.main_entity,
                mesh: id,
                indices,
                world_from_local: Mat4::IDENTITY,
                desc,
                anchor: draw.anchor,
                bias: draw.bias,
            },
        );
    }
}
