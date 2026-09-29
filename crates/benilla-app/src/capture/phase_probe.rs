//! `WOW_PHASE=<uniqueId>`: logs, per frame, which render phase each model batch of one placed
//! object landed in and where in the draw order it sits; absent from every phase means never
//! submitted. `WOW_PHASE_AT=<secs>` (default 20) and `WOW_PHASE_COUNT=<n>` (default 1) shape the
//! sampling like the screenshot burst and the ray pick. Only the main 3D view's phases are read.

use bevy::prelude::*;

use super::probes::ProbeClock;
use benilla_assets::materials::WowModelMaterial;
use benilla_world::interact::WorldObject;

mod gfx;

pub(crate) struct PhaseProbePlugin;

impl Plugin for PhaseProbePlugin {
    fn build(&self, app: &mut App) {
        let raw = std::env::var("WOW_PHASE").ok();
        // `WOW_PHASE=particles[:<bone>,…]` follows every live particle emitter and ribbon trail
        // instead. The bone list is an arming key, not a filter: the watch list is collected once,
        // when every named bone is live, so unrelated emitters that go live first do not latch it.
        let spec = raw.as_deref().map(str::trim);
        let particles = spec.is_some_and(|v| v == "particles" || v.starts_with("particles:"));
        let bones: Vec<u16> = spec
            .and_then(|v| v.strip_prefix("particles:"))
            .map(|list| {
                list.split(',')
                    .filter_map(|b| b.trim().parse().ok())
                    .collect()
            })
            .unwrap_or_default();
        let object = spec.and_then(|v| v.parse::<u32>().ok());
        if !particles && object.is_none() {
            warn!(
                "phase: WOW_PHASE wants a placement uniqueId (e.g. 235256) or \
                 `particles[:<bone>,…]` — inert"
            );
            return;
        }
        let object = object.unwrap_or(0);
        let at = std::env::var("WOW_PHASE_AT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(20.0);
        let count = std::env::var("WOW_PHASE_COUNT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1u32)
            .max(1);
        app.insert_resource(PhaseWatch {
            object,
            particles,
            bones,
            at,
            count,
            batches: Vec::new(),
            armed: false,
        })
        .add_systems(Update, (collect_batches, collect_emitters));
        // The gfx draw keeps its phase lists itself.
        gfx::build(app);
    }
}

/// The watch list and the sampling window.
#[derive(Resource, Clone)]
struct PhaseWatch {
    /// The placement `uniqueId` whose batches to follow.
    object: u32,
    /// `WOW_PHASE=particles`: follow every live particle emitter's quad mesh instead.
    particles: bool,
    /// `WOW_PHASE=particles:<bone>,…`: don't arm until every one of these emitter bones is live.
    bones: Vec<u16>,
    at: f32,
    count: u32,
    batches: Vec<WatchedBatch>,
    /// Set once the object is found and sampling has started.
    armed: bool,
}

/// One watched batch: entity, WMO batch order (or emitter bone), how it draws, and in particles
/// mode the mesh and texture asset ids the report resolves to device-side state.
type WatchedBatch = (
    Entity,
    i32,
    String,
    Option<(Option<AssetId<Mesh>>, AssetId<Image>)>,
);

/// Find the object's batch entities once it has streamed in, then hold the list.
fn collect_batches(
    mut watch: ResMut<PhaseWatch>,
    time: ProbeClock,
    // `WorldObject` rides on each submesh entity, not on a parent, so this filters, not walks.
    parts: Query<(
        Entity,
        &WorldObject,
        &benilla_world::model_render::ModelPart,
        &MeshMaterial3d<WowModelMaterial>,
    )>,
    materials: Res<Assets<WowModelMaterial>>,
) {
    if watch.armed || time.elapsed_secs() < watch.at {
        return;
    }
    let want = watch.object;
    let mut found: Vec<WatchedBatch> = parts
        .iter()
        .filter(|(_, obj, _, _)| obj.id == want)
        .map(|(e, _, part, mat)| {
            // The WMO batch order rides in the material's `sun_scale.y` (`model_render`), the
            // number `WOW_PICK` names a batch by.
            let order = materials
                .get(&mat.0)
                .map_or(-1, |m| m.extension.sun_scale.y as i32);
            (e, order, format!("{:?}", part.blend), None)
        })
        .collect();
    if found.is_empty() {
        return;
    }
    // Sort by batch order so the report reads in the WMO's authored order, not spawn order.
    found.sort_by_key(|&(_, order, _, _)| order);
    info!(
        "phase: watching {} batches of #{want} for {} frames",
        found.len(),
        watch.count
    );
    watch.batches = found;
    watch.armed = true;
}

/// The `particles` mode's collector: every live particle emitter and ribbon trail, labelled by
/// emitter bone and blend, plus the model batches near them. Effects and their model's blend
/// batches share one back-to-front `Transparent3d` list, so which draws last decides what shows.
/// One-shot, armed once the named bones are live rather than on the clock alone.
fn collect_emitters(
    mut watch: ResMut<PhaseWatch>,
    time: ProbeClock,
    emitters: Query<(Entity, &benilla_world::particles::ParticleEmitter)>,
    trails: Query<(
        Entity,
        &benilla_world::ribbons::RibbonTrail,
        &GlobalTransform,
    )>,
    parts: Query<(
        Entity,
        &benilla_world::model_render::ModelPart,
        &GlobalTransform,
    )>,
) {
    if !watch.particles || watch.armed || time.elapsed_secs() < watch.at {
        return;
    }
    let mut found: Vec<WatchedBatch> = emitters
        .iter()
        .filter(|(_, e)| e.live() > 0)
        .map(|(ent, e)| {
            let def = e.def();
            (
                ent,
                i32::from(def.bone),
                format!("EMIT {:?}(pool {})", def.blend, e.live()),
                // The vertices ride the shared effect lane's one buffer; only the texture is
                // a per-emitter GPU asset.
                Some((None, e.texture().id())),
            )
        })
        .collect();
    // A trail with fewer than two edges builds an empty mesh: submitted, but it draws nothing.
    found.extend(trails.iter().filter_map(|(ent, t, _)| {
        let (blend, edges) = t.shape();
        (edges > 0).then(|| {
            (
                ent,
                i32::from(t.bone()),
                format!("RIBB {blend:?}(edges {edges})"),
                None,
            )
        })
    }));
    if found.is_empty() {
        return;
    }
    // The arming key: every named bone must be live before the list freezes.
    if !watch
        .bones
        .iter()
        .all(|b| found.iter().any(|&(_, bone, _, _)| bone == i32::from(*b)))
    {
        return;
    }
    // The model batches within this radius of a watched anchor, the ones that can overlap it.
    const NEAR: f32 = 6.0;
    let anchors: Vec<Vec3> = emitters
        .iter()
        .filter(|(_, e)| e.live() > 0)
        .map(|(_, e)| e.anchor_world())
        // A trail's translation is its sort point: the sim writes the live head node there.
        .chain(trails.iter().map(|(_, _, gt)| gt.translation()))
        .collect();
    found.extend(
        parts
            .iter()
            .filter(|(_, _, gt)| {
                anchors
                    .iter()
                    .any(|a| gt.translation().distance(*a) <= NEAR)
            })
            .map(|(ent, part, _)| (ent, -1, format!("PART {:?}", part.blend), None)),
    );
    info!(
        "phase: watching {} live effect meshes + model batches for {} frames",
        found.len(),
        watch.count
    );
    watch.batches = found;
    watch.armed = true;
}
