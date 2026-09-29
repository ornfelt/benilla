//! The effect lane's draw counters; the lane itself is drawn by the gfx renderer
//! (`crate::gfx::effect`), whose decals ride their receiver's matrix (the tests below).

/// The last frame's `[effect items, merged draws]`, which `FPS_PROBE` prints as `fx=`.
pub static EFFECT_DRAW_STATS: [std::sync::atomic::AtomicU32; 2] = [
    std::sync::atomic::AtomicU32::new(0),
    std::sync::atomic::AtomicU32::new(0),
];

#[cfg(test)]
mod tests {
    use bevy::math::{Mat3, Mat4, Vec3};

    /// One constant-bias unit at a `Depth32Float` depth: `2^(e−23)`, the step at its exponent.
    fn bias_unit(depth: f32) -> f32 {
        f32::from_bits(depth.to_bits() + 1) - depth
    }

    /// The world meshes' and `DECAL_WORLD_CLIP`'s route: `clip_from_world × p`.
    fn depth_world(clip_from_world: &Mat4, p: Vec3) -> f32 {
        let c = *clip_from_world * p.extend(1.0);
        c.z / c.w
    }

    /// The cam-relative route: prepare's rebase, then rotation + `clip_from_view`.
    fn depth_cam_relative(
        view_from_world: &Mat4,
        clip_from_view: &Mat4,
        p: Vec3,
        cam: Vec3,
    ) -> f32 {
        let view_pos = Mat3::from_mat4(*view_from_world) * (p - cam);
        let c = *clip_from_view * view_pos.extend(1.0);
        c.z / c.w
    }

    /// At WoW-scale coordinates the cam-relative route misses the world-mesh depth by the order of
    /// a 4096-unit bias up close, while the same matrix matches bitwise: a decal rides its
    /// receiver's matrix.
    #[test]
    fn decal_depth_ties_only_through_the_mesh_matrix() {
        // A floor vertex at Kalimdor-scale coordinates (bevy = (−wow.y, wow.z, −wow.x)).
        let ground = Vec3::new(3807.13, 7.42, 7093.87);
        let clip_from_view =
            Mat4::perspective_infinite_reverse_rh(std::f32::consts::FRAC_PI_4, 16.0 / 9.0, 0.1);
        let dir = Vec3::new(0.35, 0.55, 0.76).normalize();
        let mut worst_route_over_bias = 0.0_f32;
        for step in 0..80 {
            // The zoom sweep: camera 1.5..41 yd out along a fixed off-axis orbit offset.
            let d = 1.5 + step as f32 * 0.5;
            let cam = ground + dir * d;
            // Bevy's own construction (`ExtractedView`), the rounding the mesh shaders see.
            let world_from_view = Mat4::look_at_rh(cam, ground, Vec3::Y).inverse();
            let view_from_world = world_from_view.inverse();
            let clip_from_world = clip_from_view * view_from_world;
            let mesh = depth_world(&clip_from_world, ground);
            let bias = 4096.0 * bias_unit(mesh);
            // Same matrix, same input: bitwise the same depth.
            assert_eq!(depth_world(&clip_from_world, ground), mesh);
            let route =
                (depth_cam_relative(&view_from_world, &clip_from_view, ground, cam) - mesh).abs();
            worst_route_over_bias = worst_route_over_bias.max(route / bias);
        }
        // 0.93× at d = 1.5 on this sweep; the bound leaves headroom for platform rounding.
        assert!(
            worst_route_over_bias > 0.5,
            "cam-relative route divergence stayed far inside the bias \
             (worst {worst_route_over_bias:.2}× across the sweep) — a decal would not need its \
             receiver's matrix"
        );
    }

    /// CPU-baked decal verts and GPU-transformed receiver verts differ by a few ulps of the world
    /// coordinate, which a sloped receiver takes onto its normal as depth. Sized at 3 ulps on the
    /// Stormwind gate ramp across zoom and slope: above an 8192-unit bias, under half of
    /// `Rung::DECAL_RASTER`. One bake path, so one number serves every ground decal.
    #[test]
    fn raised_bias_dominates_the_bake_residual() {
        use crate::sky_order::Rung;
        // The Stormwind gate ramp, wow (−8843.41, 642.68, 95.92) in bevy axes.
        let ground = Vec3::new(-642.68, 95.92, 8843.41);
        let clip_from_view =
            Mat4::perspective_infinite_reverse_rh(std::f32::consts::FRAC_PI_4, 16.0 / 9.0, 0.1);
        let ulp = |v: f32| f32::from_bits(v.to_bits() + 1) - v;
        // 3 ulps per world axis, signs free: the worst normal offset is the absolute sum.
        let worst_normal_offset = |n: Vec3| {
            3.0 * (n.x.abs() * ulp(ground.x)
                + n.y.abs() * ulp(ground.y)
                + n.z.abs() * ulp(ground.z))
        };
        let cam_dir = Vec3::new(0.35, 0.55, 0.76).normalize();
        let mut worst_over_old = 0.0_f32;
        let mut worst_over_retired_ring = 0.0_f32;
        let mut worst_over_new = 0.0_f32;
        // Receiver grades from level street to a steep ramp, tilted along 8 azimuths.
        for slope in [0.0_f32, 0.08, 0.2, 0.35] {
            for az in 0..8 {
                let a = az as f32 * std::f32::consts::FRAC_PI_4;
                let tilt = Vec3::new(a.cos(), 0.0, a.sin());
                let normal = (Vec3::Y - tilt * slope).normalize();
                let delta_n = worst_normal_offset(normal);
                for step in 0..80 {
                    let d = 1.0 + step as f32 * 0.5;
                    let cam = ground + cam_dir * d;
                    let world_from_view = Mat4::look_at_rh(cam, ground, Vec3::Y).inverse();
                    let view_from_world = world_from_view.inverse();
                    let clip_from_world = clip_from_view * view_from_world;
                    let depth = depth_world(&clip_from_world, ground);
                    // The offset's window-depth cost, differenced in f64 to keep f32 noise out.
                    let m = clip_from_world.as_dmat4();
                    let z_at = |p: bevy::math::DVec3| {
                        let c = m * p.extend(1.0);
                        c.z / c.w
                    };
                    let p = ground.as_dvec3();
                    let eps = 0.05;
                    let grad = (z_at(p + normal.as_dvec3() * eps) - z_at(p)).abs() / eps;
                    let conflict = delta_n as f64 * grad;
                    let unit = bias_unit(depth) as f64;
                    worst_over_old = worst_over_old.max((conflict / (4096.0 * unit)) as f32);
                    worst_over_retired_ring =
                        worst_over_retired_ring.max((conflict / (8192.0 * unit)) as f32);
                    worst_over_new =
                        worst_over_new.max((conflict / (Rung::DECAL_RASTER as f64 * unit)) as f32);
                }
            }
        }
        eprintln!(
            "bake residual worst: {worst_over_old:.3}x the old 4096 margin, \
             {worst_over_retired_ring:.3}x the retired 8192 ring bias, \
             {worst_over_new:.3}x Rung::DECAL_RASTER"
        );
        assert!(
            worst_over_old > 0.5,
            "the 3-ulp residual stayed far inside the old 4096 margin \
             (worst {worst_over_old:.2}×) — the resize's premise would be unfounded"
        );
        assert!(
            worst_over_retired_ring > 1.0,
            "the 3-ulp residual fits inside the retired +8192 (worst {worst_over_retired_ring:.2}×) \
             — then the ring and reticle were NOT under-biased and their raise wants re-arguing"
        );
        assert!(
            worst_over_new < 0.5,
            "Rung::DECAL_RASTER leaves under 2× headroom against the 3-ulp residual \
             (worst {worst_over_new:.2}×) — raise it"
        );
    }
}
