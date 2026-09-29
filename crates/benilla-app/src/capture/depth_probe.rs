//! `WOW_DEPTH`: reads back the depth that won a pixel in the opaque pass, and how far away it is.
//!
//! `WOW_DEPTH="<x>,<y>[;<x>,<y>…]"` logs, per frame, the raw reverse-Z value at each screenshot
//! pixel and the surface's distance both along the pixel's ray (comparable with `WOW_PICK`'s hit
//! distances) and to the camera plane. `WOW_DEPTH_AT=<secs>` (default 20) and
//! `WOW_DEPTH_COUNT=<n>` (default 1) shape the sampling like the screenshot burst and the ray pick.
//! Pixels are used as given: the depth texture is allocated in physical pixels.
//!
//! `WOW_DEPTH_QUADS=<bone>[,<bone>…]` (empty value = every quad emitter) samples a grid inside
//! each live particle quad's projected corners instead, and logs the fraction that survives the
//! depth test and how deep the occluder sits in front, in yards. Frames with no live quad are
//! skipped and not counted.
//!
//! MSAA must be off (`WOW_MSAA=off`): a multisampled depth texture cannot be copied and has no
//! single depth per pixel, so the probe refuses.

use bevy::prelude::*;
use bevy::render::extract_component::{ExtractComponent, ExtractComponentPlugin};
use bevy::render::render_resource::TextureUsages;

use super::probes::ProbeClock;
use benilla_world::particles::ParticleEmitter;
use benilla_world::view::WorldCamera;

mod gfx;

pub(crate) struct DepthProbePlugin;

impl Plugin for DepthProbePlugin {
    fn build(&self, app: &mut App) {
        let pixels = std::env::var("WOW_DEPTH")
            .ok()
            .map(|s| parse_pixels(&s))
            .unwrap_or_default();
        let quad_bones = parse_bones(std::env::var("WOW_DEPTH_QUADS").ok().as_deref());
        if pixels.is_empty() && quad_bones.is_none() {
            warn!(
                "depth: WOW_DEPTH wants \"<x>,<y>[;<x>,<y>…]\" screenshot pixels (or set \
                 WOW_DEPTH_QUADS) — inert"
            );
            return;
        }
        // Quad mode arms at once: frames without a live quad are skipped anyway.
        let at = std::env::var("WOW_DEPTH_AT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(if quad_bones.is_some() { 0.0 } else { 20.0 });
        let count = std::env::var("WOW_DEPTH_COUNT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1u32)
            .max(1);
        app.insert_resource(DepthWatch {
            pixels,
            at,
            count,
            armed: false,
        })
        .insert_resource(QuadWatch(quad_bones))
        .init_resource::<QuadProbes>()
        .add_systems(Update, arm)
        .add_systems(
            PostUpdate,
            collect_quads.after(benilla_world::billboard::BillboardPlace),
        )
        .add_plugins(ExtractComponentPlugin::<DepthProbeView>::default());
        // The gfx draw copies the depth itself.
        gfx::build(app);
    }
}

/// The pixels to read and the sampling window.
#[derive(Resource, Clone)]
struct DepthWatch {
    pixels: Vec<(u32, u32)>,
    at: f32,
    count: u32,
    armed: bool,
}

/// `$WOW_DEPTH_QUADS`'s bone scope: `None` = the mode is off, `Some([])` = every quad emitter.
#[derive(Resource)]
struct QuadWatch(Option<Vec<u16>>);

/// One live particle quad, in the space the depth buffer is read in.
#[derive(Clone, Copy)]
struct QuadProbe {
    bone: u16,
    /// Index within the emitter's quads, written oldest-first: the last was born this frame.
    index: u32,
    /// The four corners in physical pixels, in `expand_quads`' own vertex order.
    corners: [Vec2; 4],
    /// The corners' mid NDC depth; `dspread` is their spread, which the reference's plain
    /// billboard (`0x7b2a50`) keeps at zero.
    dquad: f32,
    dspread: f32,
    /// The quad centre's distance to the camera plane, yards.
    viewz: f32,
}

/// This frame's live quads, in `$WOW_DEPTH_QUADS` scope.
#[derive(Resource, Clone, Default)]
struct QuadProbes(Vec<QuadProbe>);

/// Project every in-scope emitter's live quads into pixels, once a frame, from the shared effect
/// stream `BillboardPlace` fills, so each quad is measured as drawn. Child-pool quads share the
/// emitter's draw records and count under its bone.
fn collect_quads(
    watch: Res<QuadWatch>,
    mut probes: ResMut<QuadProbes>,
    cam: Query<(&Camera, &GlobalTransform, &Projection), With<WorldCamera>>,
    emitters: Query<(Entity, &ParticleEmitter)>,
    quads: Res<benilla_world::particles::buffer::EffectQuads>,
) {
    let Some(bones) = watch.0.as_deref() else {
        return;
    };
    probes.0.clear();
    let (Ok((camera, cam_tf, projection)),) = (cam.single(),) else {
        return;
    };
    let Some(vp) = camera.physical_viewport_size() else {
        return;
    };
    // The frame's own matrices, the same pair `depthdump` projects with.
    let clip_from_world = projection.get_clip_from_view() * cam_tf.to_matrix().inverse();
    for (entity, emitter) in &emitters {
        if !bones.is_empty() && !bones.contains(&emitter.bone()) {
            continue;
        }
        // Every draw record this emitter committed this frame (its own pool + child pools).
        let ranges = quads
            .draws
            .iter()
            .filter(|d| d.main_entity == entity)
            .map(|d| d.range.clone());
        let pos: Vec<[f32; 3]> = ranges
            .flat_map(|r| quads.verts[r.start as usize..r.end as usize].iter())
            .map(|v| v.pos)
            .collect();
        for (index, quad) in pos.as_chunks::<4>().0.iter().enumerate() {
            let mut corners = [Vec2::ZERO; 4];
            let (mut dmin, mut dmax, mut center) = (f32::MAX, f32::MIN, Vec3::ZERO);
            let mut behind = false;
            for (i, v) in quad.iter().enumerate() {
                // Stream verts are world-space (the sort anchor rides the draw record).
                let world = Vec3::from(*v);
                center += world / 4.0;
                let clip = clip_from_world * world.extend(1.0);
                if clip.w <= 0.0 {
                    behind = true;
                    break;
                }
                let ndc = clip.truncate() / clip.w;
                corners[i] = Vec2::new(
                    (ndc.x + 1.0) * 0.5 * vp.x as f32,
                    (1.0 - ndc.y) * 0.5 * vp.y as f32,
                );
                dmin = dmin.min(ndc.z);
                dmax = dmax.max(ndc.z);
            }
            if behind {
                continue;
            }
            probes.0.push(QuadProbe {
                bone: emitter.bone(),
                index: index as u32,
                corners,
                dquad: (dmin + dmax) * 0.5,
                dspread: dmax - dmin,
                viewz: -(cam_tf.to_matrix().inverse() * center.extend(1.0)).z,
            });
        }
    }
}

/// `"60,61"` → the bone scope; an empty value means every quad emitter, an unset one `None`.
fn parse_bones(spec: Option<&str>) -> Option<Vec<u16>> {
    Some(
        spec?
            .split(',')
            .filter_map(|b| b.trim().parse().ok())
            .collect(),
    )
}

/// Marks the one view whose depth to read; the UI camera shares the world camera's depth texture.
#[derive(Component, Clone, Copy, ExtractComponent)]
struct DepthProbeView;

/// Once the sampling window opens, opt the world camera's depth texture into `COPY_SRC` and mark
/// it; the live `Msaa` component being anything but off disables the probe.
fn arm(
    mut watch: ResMut<DepthWatch>,
    time: ProbeClock,
    mut cam: Query<(Entity, &mut Camera3d, &Msaa), With<WorldCamera>>,
    mut commands: Commands,
) {
    if watch.armed || time.elapsed_secs() < watch.at {
        return;
    }
    let Ok((entity, mut camera, msaa)) = cam.single_mut() else {
        return;
    };
    if *msaa != Msaa::Off {
        error!(
            "depth: MSAA is {msaa:?} — a multisampled depth texture cannot be copied, and there is \
             no single depth per pixel to report. Re-run with WOW_MSAA=off. Probe disabled."
        );
        watch.armed = true;
        watch.count = 0;
        return;
    }
    camera.depth_texture_usages =
        (TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC).into();
    commands.entity(entity).insert(DepthProbeView);
    info!(
        "depth: reading {} pixels for {} frames",
        watch.pixels.len(),
        watch.count
    );
    watch.armed = true;
}

/// Log one frame's named pixels and quads, `depth_at` reading the depth at a pixel of the
/// `size` view drawn through `clip_from_view`.
fn report_frame(
    frame: u32,
    size: UVec2,
    clip_from_view: &Mat4,
    pixels: &[(u32, u32)],
    quads: &[QuadProbe],
    depth_at: impl Fn(u32, u32) -> f32,
) {
    // The projection the frame was drawn with, once per burst: the distances derive from it.
    if frame == 0 {
        info!(
            "depth: {}x{} view, clip_from_view P₂₂ {} P₃₂ {} P₀₀ {} P₁₁ {}",
            size.x,
            size.y,
            clip_from_view.z_axis.z,
            clip_from_view.w_axis.z,
            clip_from_view.x_axis.x,
            clip_from_view.y_axis.y,
        );
    }
    let view_from_clip = clip_from_view.inverse();
    for &(x, y) in pixels {
        if x >= size.x || y >= size.y {
            warn!(
                "depth#{frame} ({x}, {y}): outside the {}x{} view",
                size.x, size.y
            );
            continue;
        }
        let d = depth_at(x, y);
        match view_point(&view_from_clip, ndc_of(x, y, size.x, size.y), d) {
            // Along the ray (what `WOW_PICK` reports) and to the camera plane (what depth
            // encodes); they differ by 15% at the frame edge.
            Some(p) => info!(
                "depth#{frame} ({x}, {y}): {d:.9}  =  {:.4} yd along the ray  ({:.4} yd view z)",
                p.length(),
                -p.z
            ),
            // Reverse-Z clears to 0 = infinitely far: nothing drew here at all.
            None => info!("depth#{frame} ({x}, {y}): {d:.9}  =  nothing drew (cleared)"),
        }
    }
    for q in quads {
        report_quad(frame, q, &depth_at, size.x, size.y);
    }
}

/// Samples per side across a quad's own area: 16x16, stable to under a percent.
const QUAD_GRID: usize = 16;

/// Run one quad's depth contest and log it. Reverse-Z with Bevy's default `GreaterEqual`: a
/// fragment survives iff `dquad >= dbuffer`. Samples interpolate the four projected corners, so
/// each lies inside the quad even when it is spun.
fn report_quad(
    frame: u32,
    q: &QuadProbe,
    depth_at: &impl Fn(u32, u32) -> f32,
    width: u32,
    height: u32,
) {
    let (mut passed, mut total) = (0usize, 0usize);
    // Reverse-Z: `dmax` is the nearest occluder, `dmin` the furthest.
    let (mut dmin, mut dmax) = (f32::MAX, f32::MIN);
    let mut cleared = 0usize;
    for iy in 0..QUAD_GRID {
        for ix in 0..QUAD_GRID {
            let u = (ix as f32 + 0.5) / QUAD_GRID as f32;
            let v = (iy as f32 + 0.5) / QUAD_GRID as f32;
            let p = q.corners[0]
                .lerp(q.corners[1], u)
                .lerp(q.corners[3].lerp(q.corners[2], u), v);
            let (x, y) = (p.x.floor(), p.y.floor());
            if x < 0.0 || y < 0.0 || x >= width as f32 || y >= height as f32 {
                continue;
            }
            let d = depth_at(x as u32, y as u32);
            total += 1;
            if q.dquad >= d {
                passed += 1;
            }
            if d <= 0.0 {
                cleared += 1;
            } else {
                dmin = dmin.min(d);
                dmax = dmax.max(d);
            }
        }
    }
    if total == 0 {
        info!(
            "depth#{frame} quad bone={} i={}: entirely off screen",
            q.bone, q.index
        );
        return;
    }
    // On the reverse-Z curve view z scales as 1/d, so the ratio to `dquad` converts to yards.
    let yd = |d: f32| q.viewz * q.dquad / d;
    let (near, far) = (yd(dmax), yd(dmin));
    info!(
        "depth#{frame} quad bone={} i={} px=({:.0},{:.0}) dquad={:.9} spread={:.9} \
         viewz={:.4} pass={:.1}% ({passed}/{total}) occluder {near:.4}..{far:.4} yd \
         burial {:.4}..{:.4} yd cleared={cleared}",
        q.bone,
        q.index,
        q.corners[0].lerp(q.corners[2], 0.5).x,
        q.corners[0].lerp(q.corners[2], 0.5).y,
        q.dquad,
        q.dspread,
        q.viewz,
        passed as f32 / total as f32 * 100.0,
        q.viewz - far,
        q.viewz - near,
    );
}

/// A physical pixel's centre in NDC. Framebuffer rows run down, NDC y runs up.
fn ndc_of(x: u32, y: u32, width: u32, height: u32) -> Vec2 {
    Vec2::new(
        (x as f32 + 0.5) / width as f32 * 2.0 - 1.0,
        1.0 - (y as f32 + 0.5) / height as f32 * 2.0,
    )
}

/// Unproject a pixel's depth through the live matrix to its view-space point, in yards. The whole
/// point is needed: off axis, the ray length exceeds the plane distance by `1/cos θ`.
fn view_point(view_from_clip: &Mat4, ndc: Vec2, d: f32) -> Option<Vec3> {
    let p = *view_from_clip * Vec4::new(ndc.x, ndc.y, d, 1.0);
    // Behind the camera or at infinity (a reverse-Z clear reads 0 ⇒ w 0) means nothing drew here.
    (p.w.abs() > f32::MIN_POSITIVE)
        .then(|| p.truncate() / p.w)
        .filter(|v| v.is_finite() && v.z < 0.0)
}

/// `"x,y;x,y"` → pixels; a malformed pair is dropped with a warning.
fn parse_pixels(spec: &str) -> Vec<(u32, u32)> {
    spec.split(';')
        .filter(|s| !s.trim().is_empty())
        .filter_map(|pair| {
            let (x, y) = pair.split_once(',')?;
            match (x.trim().parse().ok(), y.trim().parse().ok()) {
                (Some(x), Some(y)) => Some((x, y)),
                _ => {
                    warn!("depth: skipping malformed pixel {pair:?}");
                    None
                }
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_pixel_list_and_skips_junk() {
        assert_eq!(
            parse_pixels("412,396; 565,445 ;;nope;1,"),
            vec![(412, 396), (565, 445)]
        );
    }

    /// The projection the world camera actually draws with: Bevy's reverse-Z, infinite far.
    fn proj() -> Mat4 {
        Mat4::perspective_infinite_reverse_rh(
            std::f32::consts::FRAC_PI_4,
            3200.0 / 1800.0,
            benilla_world::view::NEARCLIP_DEFAULT,
        )
    }

    /// What the rasteriser writes for a point `dist` yards straight ahead down the view axis.
    fn depth_of(dist: f32) -> f32 {
        let clip = proj() * Vec4::new(0.0, 0.0, -dist, 1.0);
        clip.z / clip.w
    }

    #[test]
    fn a_depth_unprojects_to_the_distance_it_came_from() {
        let inv = proj().inverse();
        for dist in [2.0f32, 22.0, 46.0253, 46.0897, 3000.0] {
            let p =
                view_point(&inv, Vec2::ZERO, depth_of(dist)).expect("a drawn pixel has a point");
            assert!(
                (p.length() - dist).abs() < dist * 1e-3,
                "{dist} yd -> depth {} -> {} yd",
                depth_of(dist),
                p.length()
            );
        }
    }

    #[test]
    fn off_axis_the_ray_is_longer_than_the_perpendicular_distance() {
        // Straight ahead the two agree; at the frame edge they must not.
        let inv = proj().inverse();
        let d = depth_of(46.0);
        let centre = view_point(&inv, Vec2::ZERO, d).unwrap();
        assert!(
            (centre.length() - (-centre.z)).abs() < 1e-3,
            "on-axis they agree"
        );
        let edge = view_point(&inv, Vec2::new(-0.78, 0.42), d).unwrap();
        assert!(
            (-edge.z - 46.0).abs() < 0.05,
            "the perpendicular distance is what depth encodes: {} yd",
            -edge.z
        );
        assert!(
            edge.length() > 46.0 * 1.1,
            "off-axis the ray must be materially longer, got {} yd",
            edge.length()
        );
    }

    #[test]
    fn a_cleared_pixel_has_no_position() {
        // Reverse-Z clears to 0.0: infinitely far, i.e. nothing drew.
        assert_eq!(view_point(&proj().inverse(), Vec2::ZERO, 0.0), None);
    }

    #[test]
    fn the_awning_and_the_plank_are_thousands_of_ulps_apart() {
        // Two measured surfaces 1.4 cm apart perpendicular: the readback names the winner only if
        // that gap survives `f32`.
        let (awning, plank) = (46.0253f32, 46.0897f32);
        let (da, dp) = (depth_of(awning), depth_of(plank));
        let ulps = ((da.to_bits() as i64) - (dp.to_bits() as i64)).abs();
        assert!(
            ulps > 1000,
            "only {ulps} ULPs apart — a readback could not tell them apart"
        );
        // And they must not round to the same reported distance either.
        let inv = proj().inverse();
        let (ba, bp) = (
            view_point(&inv, Vec2::ZERO, da).unwrap().length(),
            view_point(&inv, Vec2::ZERO, dp).unwrap().length(),
        );
        assert!(
            (ba - bp).abs() > 0.01,
            "{ba} yd vs {bp} yd is not a distinguishable pair"
        );
    }
}
