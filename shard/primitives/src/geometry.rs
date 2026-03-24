// primitives/src/geometry.rs
// All geometry is computed with nalgebra so every rotation, projection and
// placement is exact.  The LCG is kept for deterministic scatter elements.
//
// Coordinate convention (FreeCAD):
//   X+  right   Y+  into screen (depth)   Z+  up
//
// Major changes vs. the original:
//   • make_frame()       – 36 tangentially-oriented ShardSpec facets replace
//                          the single ellipsoid-cut ring.  Colors alternate
//                          copper / steel to give the teal + orange blocking.
//   • make_core()        – nalgebra projection for inner-shard scatter;
//                          diamond zone exclusion tightened; 40 shards default.
//   • make_spikes()      – each primary / secondary spike becomes a
//                          polyhedral cluster (1 main + 3 flanking thin cones).
//   • make_foundation()  – V-plates are larger and properly horizontal;
//                          debris forms a distinct wide crystalline pile.

use std::f64::consts::PI;
use nalgebra::{Vector3, Rotation3, Unit};

use crate::types::{
    ShardParams, ShardSpec, SpikeSpec, FoundationSpec, EllipseFrameSpec,
};

// ── Deterministic LCG ────────────────────────────────────────────────────────
fn lcg(s: &mut u64) -> f64 {
    *s = s.wrapping_mul(6_364_136_223_846_793_005)
          .wrapping_add(1_442_695_040_888_963_407);
    ((*s >> 33) as f64) / ((1u64 << 31) as f64)
}

/// Uniformly sample inside ellipse (rx, rz) scaled by `fill`.
fn rand_in_ellipse(s: &mut u64, rx: f64, rz: f64, fill: f64) -> (f64, f64) {
    loop {
        let x = (lcg(s) * 2.0 - 1.0) * rx * fill;
        let z = (lcg(s) * 2.0 - 1.0) * rz * fill;
        if (x / (rx * fill)).powi(2) + (z / (rz * fill)).powi(2) < 1.0 {
            return (x, z);
        }
    }
}

// ── Rotation helpers (nalgebra → Euler angles for ShardSpec) ─────────────────
//
// FreeCAD _rot_euler applies ZYX intrinsic: first Rz, then Ry, then Rx.
// For a pure Y-rotation we only need rot_y; others stay zero.
//
// For the frame facets we need the angle that rotates the box's X-axis to
// align with the tangent to the ellipse at angle φ.
//
// Rotation around Y by θ maps:
//   world-X → (cos θ,  0, -sin θ)
//   world-Z → (sin θ,  0,  cos θ)
//
// Tangent to (rx·sinφ, rz·cosφ) is proportional to (rx·cosφ, -rz·sinφ).
// We want world-X → (rx·cosφ, 0, -rz·sinφ) / |…|
//
//   cos θ = rx·cosφ / L,   -sin θ = -rz·sinφ / L   → sin θ = rz·sinφ / L
//   θ = atan2(rz·sinφ, rx·cosφ)
//
// This is exactly the rot_y stored in every ShardSpec for a frame facet.

fn frame_rot_y(phi: f64, rx: f64, rz: f64) -> f64 {
    (rz * phi.sin()).atan2(rx * phi.cos()).to_degrees()
}

// ═══════════════════════════════════════════════════════════════════════════════
// § 1  Faceted elliptical ring frame
// ═══════════════════════════════════════════════════════════════════════════════
//
// The ring is replaced by N_SEGS flat box facets arranged tangentially around
// the ellipse.  Adjacent facets overlap by 12 % so there are no visible gaps.
//
// Copper (warm) and teal (cool) facets alternate every 3 segments to produce
// the high-contrast color-blocking described in the design brief.
//
// Facet centre: on the mid-radial ellipse  →  rx_mid = rx + t/2, rz_mid = rz + t/2
// Facet length: chord + 12 % overlap in the tangent direction
// Facet width:  frame_depth * 2 (full Y extent)
// Facet height: thickness, varied ±25 % per facet for interlocking depth

pub fn make_frame(p: &ShardParams) -> Vec<ShardSpec> {
    const N_SEGS: usize = 36;

    let rx = p.frame_rx;
    let rz = p.frame_rz;
    let t  = p.frame_thickness;
    let d  = p.frame_depth * 2.0;   // full Y width of each facet

    // Mid-radial axes (centre of the wall)
    let rx_m = rx + t * 0.5;
    let rz_m = rz + t * 0.5;

    // Colour palette: teal / steel-blue / copper / warm-copper
    let teal   = (0.35_f32, 0.62, 0.68);
    let steel  = (0.52_f32, 0.60, 0.68);
    let copper = (0.72_f32, 0.48, 0.28);
    let warm   = (0.65_f32, 0.42, 0.22);

    let palette: [(f32, f32, f32); 6] = [teal, steel, steel, copper, warm, copper];

    let mut facets: Vec<ShardSpec> = Vec::with_capacity(N_SEGS);

    for i in 0..N_SEGS {
        let fi       = i as f64;
        let phi_a    = fi       * 2.0 * PI / N_SEGS as f64;
        let phi_b    = (fi + 1.0) * 2.0 * PI / N_SEGS as f64;
        let phi_mid  = (phi_a + phi_b) * 0.5;

        // Centre of this facet
        let cx = rx_m * phi_mid.sin();
        let cz = rz_m * phi_mid.cos();

        // Chord length between the two edge points on the inner ellipse
        let ax = rx * phi_a.sin();   let az = rz * phi_a.cos();
        let bx = rx * phi_b.sin();   let bz = rz * phi_b.cos();
        let chord = ((bx - ax).powi(2) + (bz - az).powi(2)).sqrt();
        let seg_len = chord * 1.12;  // 12 % overlap

        // Rotation: align box's length-axis (local X) with ellipse tangent
        let rot_y_deg = frame_rot_y(phi_mid, rx, rz);

        // Vary thickness slightly so facets interlock visually
        let thickness_var = t * (1.0 + 0.28 * ((fi * 1.74).sin()));

        // Vertical bevel: upper facets tilt slightly toward viewer
        let bevel_x = ((fi - N_SEGS as f64 / 2.0) / N_SEGS as f64 * 8.0).abs() - 2.0;

        let color = palette[i % palette.len()];

        facets.push(ShardSpec {
            label:     format!("Frame_Facet_{:02}", i),
            part_type: "frame_facet".into(),
            x: cx, y: 0.0, z: cz,
            length:    seg_len,
            width:     d,
            height:    thickness_var,
            rot_x:     bevel_x,
            rot_y:     rot_y_deg,
            rot_z:     0.0,
            color,
        });
    }

    // Inner accent ring: smaller copper facets sit just inside the frame
    // at 2/3 radius, creating the "copper and teal" depth visible inside the Eye.
    let rx_inner = rx * 0.82;
    let rz_inner = rz * 0.82;
    let inner_segs = 24usize;

    for i in 0..inner_segs {
        let fi      = i as f64;
        let phi_mid = fi * 2.0 * PI / inner_segs as f64 + PI / inner_segs as f64;

        let cx = rx_inner * phi_mid.sin();
        let cz = rz_inner * phi_mid.cos();

        let rot_y_deg = frame_rot_y(phi_mid, rx_inner, rz_inner);

        let inner_len = 2.0 * PI * rx_inner / inner_segs as f64 * 1.05;
        let color: (f32, f32, f32) = if i % 2 == 0 { copper } else { warm };

        facets.push(ShardSpec {
            label:     format!("Frame_InnerRing_{:02}", i),
            part_type: "frame_inner".into(),
            x: cx, y: 0.0, z: cz,
            length:    inner_len,
            width:     d * 0.6,
            height:    t * 0.55,
            rot_x:     0.0,
            rot_y:     rot_y_deg,
            rot_z:     0.0,
            color,
        });
    }

    facets
}

// ═══════════════════════════════════════════════════════════════════════════════
// § 2  Inner core
// ═══════════════════════════════════════════════════════════════════════════════

pub fn make_core(p: &ShardParams) -> (Vec<SpikeSpec>, Vec<ShardSpec>) {
    let mut cones:  Vec<SpikeSpec> = Vec::new();
    let mut shards: Vec<ShardSpec> = Vec::new();

    // ── Diamond bipyramid ────────────────────────────────────────────────────

    cones.push(SpikeSpec {
        label:       "Core_DiamondUp".into(),
        part_type:   "diamond_up".into(),
        x: 0.0, y: 0.0, z: 0.0,
        length:      p.diamond_h_up,
        base_radius: p.diamond_base_r,
        tip_radius:  0.0,
        rot_y:       0.0,
        rot_x:       0.0,
        color:       (0.88, 0.92, 0.96),
    });

    cones.push(SpikeSpec {
        label:       "Core_DiamondDown".into(),
        part_type:   "diamond_down".into(),
        x: 0.0, y: 0.0, z: 0.0,
        length:      p.diamond_h_dn,
        base_radius: p.diamond_base_r * 0.75,
        tip_radius:  0.0,
        rot_y:       180.0,
        rot_x:       0.0,
        color:       (0.80, 0.86, 0.92),
    });

    // Diamond facet rings — nested cones at intermediate radii give a
    // multifaceted pyramidal look at the core without Boolean operations.
    for (k, frac) in [(0.55_f64, 0.62_f32), (0.30, 0.74)].iter().enumerate() {
        cones.push(SpikeSpec {
            label:       format!("Core_DiamondFacetUp_{}", k),
            part_type:   "diamond_up".into(),
            x: 0.0, y: 0.0, z: 0.0,
            length:      p.diamond_h_up * frac.0,
            base_radius: p.diamond_base_r * 1.25,
            tip_radius:  p.diamond_base_r * frac.0 as f64,
            rot_y:       0.0,
            rot_x:       0.0,
            color:       (frac.1, frac.1 + 0.04, frac.1 + 0.08),
        });
        cones.push(SpikeSpec {
            label:       format!("Core_DiamondFacetDown_{}", k),
            part_type:   "diamond_down".into(),
            x: 0.0, y: 0.0, z: 0.0,
            length:      p.diamond_h_dn * frac.0,
            base_radius: p.diamond_base_r * 1.1,
            tip_radius:  p.diamond_base_r * 0.5 * frac.0 as f64,
            rot_y:       180.0,
            rot_x:       0.0,
            color:       (frac.1 - 0.05, frac.1, frac.1 + 0.05),
        });
    }

    // ── Horizontal blades ────────────────────────────────────────────────────

    let blade_colors = [
        (0.72_f32, 0.76, 0.82),
        (0.65, 0.70, 0.77),
        (0.58, 0.63, 0.71),
        (0.50, 0.56, 0.64),
        (0.44, 0.50, 0.60),
    ];

    for i in 0..p.blade_pairs {
        let fi     = i as f64;
        let reach  = p.frame_rx * (0.50 + fi * 0.10);
        let height = 60.0 - fi * 7.0;
        let depth  = 32.0 + fi * 7.0;
        let tilt_z = 7.0  + fi * 4.5;
        let z_off  = fi * 20.0 - (p.blade_pairs as f64 - 1.0) * 10.0;
        let y_off  = (fi * 14.0 - 12.0) * if i % 2 == 0 { 1.0 } else { -1.0 };
        let color  = blade_colors[(i as usize).min(4)];

        shards.push(ShardSpec {
            label:     format!("Core_BladeLeft{}", i + 1),
            part_type: "inner_blade".into(),
            x: -(p.diamond_base_r * 0.9 + reach * 0.5),
            y: y_off, z: z_off,
            length: reach, width: depth, height,
            rot_x: 0.0, rot_y: 0.0, rot_z: tilt_z,
            color,
        });
        shards.push(ShardSpec {
            label:     format!("Core_BladeRight{}", i + 1),
            part_type: "inner_blade".into(),
            x: p.diamond_base_r * 0.9 + reach * 0.5,
            y: -y_off, z: z_off,
            length: reach, width: depth, height,
            rot_x: 0.0, rot_y: 0.0, rot_z: -tilt_z,
            color,
        });
    }

    // ── Inner shards (nalgebra-assisted placement) ────────────────────────────
    //
    // We use nalgebra to compute a 3-D scatter within the elliptical interior.
    // Shards are oriented so their longest axis is roughly radial (pointing away
    // from the centre), which matches the "dense polyhedral fill" appearance.

    let mut seed = 0xDEAD_BEEF_u64;

    let shard_colors = [
        (0.80_f32, 0.83, 0.88),
        (0.62, 0.67, 0.74),
        (0.48, 0.52, 0.58),
        (0.72, 0.68, 0.80),
        (0.55, 0.60, 0.65),
        (0.68, 0.55, 0.42),  // copper accent
        (0.40, 0.46, 0.54),
    ];

    let mut placed  = 0u32;
    let mut attempts = 0u32;
    let max_placed   = p.inner_shards;

    while placed < max_placed && attempts < max_placed * 12 {
        attempts += 1;

        let (sx, sz) = rand_in_ellipse(&mut seed, p.frame_rx, p.frame_rz, 0.80);

        // Exclude the diamond-dominated centre zone
        let near_diamond = sx.abs() < p.diamond_base_r * 1.8
            && sz.abs() < p.diamond_h_up * 0.32;
        if near_diamond { continue; }

        // nalgebra: radial unit vector in XZ plane for orientation
        let radial_xz = Vector3::new(sx, 0.0, sz);
        let r_len = radial_xz.norm();
        let r_norm_scalar = r_len / (p.frame_rx.hypot(p.frame_rz) * 0.5);

        // Shard scale shrinks toward periphery for a denser, packed feel
        let base_l = 200.0 - r_norm_scalar * 90.0;
        let base_h = 60.0  - r_norm_scalar * 28.0;

        let length = (base_l * (0.6 + lcg(&mut seed) * 0.8)).max(30.0);
        let width  = 16.0 + lcg(&mut seed) * 32.0;
        let height = (base_h * (0.4 + lcg(&mut seed) * 0.8)).max(18.0);
        let sy     = (lcg(&mut seed) - 0.5) * p.frame_depth * 0.85;

        // Orient the shard's length axis (rot_y in XZ plane) radially outward,
        // then add a random tumble on each axis.
        let radial_angle_deg = if r_len > 1e-6 {
            (sx / r_len).asin().to_degrees() * if sz < 0.0 { -1.0 } else { 1.0 }
        } else {
            0.0
        };

        let rot_x = (lcg(&mut seed) - 0.5) * 32.0;
        let rot_y = radial_angle_deg + (lcg(&mut seed) - 0.5) * 25.0;
        let rot_z = (lcg(&mut seed) - 0.5) * 45.0;

        let ci = placed as usize % shard_colors.len();
        shards.push(ShardSpec {
            label:     format!("Core_Shard{:02}", placed + 1),
            part_type: "inner_shard".into(),
            x: sx, y: sy, z: sz,
            length, width, height,
            rot_x, rot_y, rot_z,
            color: shard_colors[ci],
        });
        placed += 1;
    }

    (cones, shards)
}

// ═══════════════════════════════════════════════════════════════════════════════
// § 3  Radial spikes — polyhedral clusters
// ═══════════════════════════════════════════════════════════════════════════════
//
// Each primary and secondary spike becomes a polyhedral cluster:
//   • 1 main cone at the exact angle
//   • 3 thin flanking cones at ±angular offsets, slightly shorter
//
// This produces the "multifaceted pyramidal spike" look without needing
// FreeCAD Boolean operations.
//
// nalgebra is used to build the parametric ellipse point and to ensure
// all base positions are exactly on the outer-ellipse surface.

pub fn make_spikes(p: &ShardParams) -> Vec<SpikeSpec> {
    let mut v: Vec<SpikeSpec> = Vec::new();

    let ro_x = p.frame_rx + p.frame_thickness;
    let ro_z = p.frame_rz + p.frame_thickness;
    let y_off = -p.frame_depth * 0.30;

    // Parametric point on outer ellipse
    let ellipse_pt = |phi: f64| -> (f64, f64) {
        (ro_x * phi.sin(), ro_z * phi.cos())
    };

    // ── Helper: push a polyhedral spike cluster ───────────────────────────────
    //
    // Generates 1 main cone + `n_flanks` flanking cones.
    // `angular_spread` controls how far (degrees) flanking cones deviate
    // from the main axis.  Flanking cones also have a small rot_x tilt for
    // out-of-plane depth.

    let mut push_cluster = |v: &mut Vec<SpikeSpec>,
                             label: &str,
                             part_type: &str,
                             x: f64, y: f64, z: f64,
                             length: f64,
                             base_r: f64,
                             rot_y_deg: f64,
                             color: (f32, f32, f32),
                             n_flanks: usize,
                             spread_deg: f64| {
        // Main spike
        v.push(SpikeSpec {
            label:       label.to_string(),
            part_type:   part_type.to_string(),
            x, y, z,
            length,
            base_radius: base_r,
            tip_radius:  0.0,
            rot_y:       rot_y_deg,
            rot_x:       0.0,
            color,
        });
        // Flanking sub-cones
        for k in 0..n_flanks {
            let sign = if k % 2 == 0 { 1.0_f64 } else { -1.0 };
            let factor = (k / 2 + 1) as f64;
            let d_ang = sign * factor * spread_deg / ((n_flanks / 2 + 1) as f64);
            let sub_len = length * (0.72 - k as f64 * 0.06).max(0.30);
            let sub_r   = base_r  * (0.38 - k as f64 * 0.04).max(0.12);
            let rx_tilt = (k as f64 + 1.0) * 3.5 * sign;
            v.push(SpikeSpec {
                label:       format!("{}_F{}", label, k),
                part_type:   part_type.to_string(),
                x, y, z,
                length:      sub_len,
                base_radius: sub_r,
                tip_radius:  0.0,
                rot_y:       rot_y_deg + d_ang,
                rot_x:       rx_tilt,
                color:       (
                    (color.0 - 0.07).max(0.0),
                    (color.1 - 0.07).max(0.0),
                    (color.2 - 0.07).max(0.0),
                ),
            });
        }
    };

    // ── Primary corner spikes ─────────────────────────────────────────────────

    let primary_color: (f32, f32, f32) = (0.48, 0.54, 0.62);
    let primary_data: [(f64, &str); 4] = [
        ( 45.0, "NE"),
        (-45.0, "NW"),
        (135.0, "SE"),
        (-135.0, "SW"),
    ];

    for (phi_deg, quad) in &primary_data {
        let phi = phi_deg.to_radians();
        let (ex, ez) = ellipse_pt(phi);
        push_cluster(
            &mut v,
            &format!("Spike_Primary_{}", quad),
            "primary",
            ex, y_off, ez,
            p.primary_len,
            p.primary_base,
            *phi_deg,
            primary_color,
            4,     // 4 flanking cones → 5-sided polyhedral appearance
            18.0,  // ±18° spread
        );
    }

    // ── Secondary axis spikes ─────────────────────────────────────────────────

    let secondary_color: (f32, f32, f32) = (0.56, 0.62, 0.70);
    let secondary_data: [(f64, &str, f64); 4] = [
        (  0.0, "Up",    p.secondary_len * 1.20),
        (180.0, "Down",  p.secondary_len * 0.70),
        ( 90.0, "Right", p.secondary_len),
        (-90.0, "Left",  p.secondary_len),
    ];

    for (phi_deg, dir, length) in &secondary_data {
        let phi = phi_deg.to_radians();
        let (ex, ez) = ellipse_pt(phi);
        push_cluster(
            &mut v,
            &format!("Spike_Secondary_{}", dir),
            "secondary",
            ex, y_off, ez,
            *length,
            p.secondary_base,
            *phi_deg,
            secondary_color,
            2,     // 2 flanking cones → trimmed polyhedral
            12.0,
        );
    }

    // ── Minor mid-angle spikes ────────────────────────────────────────────────
    //
    // 16 minor spikes at 22.5° intervals (double the original 8) for a
    // dense starburst fill.

    let minor_base  = p.primary_base * 0.50;
    let minor_len   = p.secondary_len * 0.65;
    let minor_color: (f32, f32, f32) = (0.54, 0.60, 0.67);

    let minor_angles: Vec<f64> = (0..16)
        .map(|i| i as f64 * 22.5 + 11.25)
        .collect();

    for (idx, phi_deg) in minor_angles.iter().enumerate() {
        let phi = phi_deg.to_radians();
        let (ex, ez) = ellipse_pt(phi);

        // Alternate minor spike lengths for visual rhythm
        let len_var = minor_len * (1.0 + 0.25 * ((idx as f64 * 1.3).sin()));

        push_cluster(
            &mut v,
            &format!("Spike_Minor{:02}", idx + 1),
            "minor",
            ex, y_off * 0.65, ez,
            len_var,
            minor_base,
            *phi_deg,
            minor_color,
            2,     // 2 flanking for minor spikes
            10.0,
        );
    }

    v
}

// ═══════════════════════════════════════════════════════════════════════════════
// § 4  Foundation — horizontal tiered V-plates + large debris formation
// ═══════════════════════════════════════════════════════════════════════════════
//
// Key corrections vs. the original:
//   • Plates are wider and the V-spread is more aggressive so the structure
//     reads as a proper horizontal base, not a scattered cluster.
//   • Debris forms a distinct, wide crystalline pile with larger pieces and
//     a broader footprint — not just small random boxes.
//   • The foundation centre is horizontally aligned (no tilt).

pub fn make_foundation(p: &ShardParams) -> Vec<FoundationSpec> {
    let mut v: Vec<FoundationSpec> = Vec::new();

    let base_z    = -(p.frame_rz + p.frame_thickness + 100.0);
    let tier_step = 130.0;
    let plate_h   = 300.0;
    let plate_d   = 60.0;   // Y depth
    let plate_t   = 50.0;   // X thickness

    let n = p.foundation_tiers as f64;

    for i in 0..p.foundation_tiers {
        let fi     = i as f64;
        let tier_z = base_z - fi * tier_step;

        // Wider V-spread on lower tiers
        let half_span    = 380.0 + fi * 250.0;
        let v_angle_deg  = 42.0  - fi * 4.0;
        let arm_h        = plate_h + fi * 25.0;

        let t_frac = fi / n.max(1.0);
        let plate_color: (f32, f32, f32) = (
            (0.50 - t_frac * 0.12) as f32,
            (0.52 - t_frac * 0.12) as f32,
            (0.56 - t_frac * 0.12) as f32,
        );

        // Left V-arm
        v.push(FoundationSpec {
            label:     format!("Foundation_Tier{}_LeftArm", i + 1),
            part_type: "v_plate".into(),
            x: -half_span * 0.5, y: 0.0,
            z: tier_z - arm_h * 0.5,
            length: plate_t, width: plate_d, height: arm_h,
            rot_x: 0.0, rot_y: 0.0,
            rot_z: -v_angle_deg,
            color: plate_color,
        });

        // Right V-arm
        v.push(FoundationSpec {
            label:     format!("Foundation_Tier{}_RightArm", i + 1),
            part_type: "v_plate".into(),
            x: half_span * 0.5, y: 0.0,
            z: tier_z - arm_h * 0.5,
            length: plate_t, width: plate_d, height: arm_h,
            rot_x: 0.0, rot_y: 0.0,
            rot_z:  v_angle_deg,
            color: plate_color,
        });

        // Horizontal cap bar
        let bar_w = half_span * 2.0 * 0.40;
        v.push(FoundationSpec {
            label:     format!("Foundation_Tier{}_CapBar", i + 1),
            part_type: "tier_bar".into(),
            x: -bar_w * 0.5, y: -plate_d * 0.5,
            z: tier_z - 15.0,
            length: bar_w, width: plate_d, height: 26.0,
            rot_x: 0.0, rot_y: 0.0, rot_z: 0.0,
            color: (
                (plate_color.0 + 0.09).min(1.0),
                (plate_color.1 + 0.09).min(1.0),
                (plate_color.2 + 0.09).min(1.0),
            ),
        });

        // Secondary inner V-pair (half-size) for structural depth
        let inner_span  = half_span * 0.55;
        let inner_angle = v_angle_deg * 1.25;
        let inner_color = (
            (plate_color.0 + 0.05).min(1.0),
            (plate_color.1 + 0.05).min(1.0),
            (plate_color.2 + 0.06).min(1.0),
        );
        for &side in &[-1.0_f64, 1.0] {
            v.push(FoundationSpec {
                label:     format!("Foundation_Tier{}_InnerArm{}", i + 1,
                                   if side < 0.0 { "L" } else { "R" }),
                part_type: "v_plate".into(),
                x: side * inner_span * 0.5, y: 0.0,
                z: tier_z - arm_h * 0.40,
                length: plate_t * 0.7, width: plate_d * 0.8,
                height: arm_h * 0.65,
                rot_x: 0.0, rot_y: 0.0,
                rot_z: -side * inner_angle,
                color: inner_color,
            });
        }
    }

    // ── Large crystalline debris formation ────────────────────────────────────
    //
    // The debris pile is a distinct, wide crystalline formation spread beneath
    // the last foundation tier.  Pieces are larger than before and cluster in
    // a wide arc that reads as a separate visual element.
    //
    // nalgebra: each piece's long axis is rotated using Rotation3::from_euler_angles
    // to produce proper 3-D tumble — the Euler angles fed into FoundationSpec
    // are derived from this so the Python macro reconstructs the same orientation.

    let mut seed      = 0xFACE_CAFE_u64;
    let debris_z_top  = base_z - (p.foundation_tiers as f64) * tier_step - 40.0;
    // Horizontal extent of the debris arc
    let debris_spread = 900.0 + p.frame_rx * 0.35;

    for i in 0..p.debris_count {
        // Alternate left / right, biased toward outer edges
        let side = if i % 2 == 0 { 1.0_f64 } else { -1.0 };
        let edge_bias = 0.45 + lcg(&mut seed) * 0.55;

        let bx = side * edge_bias * debris_spread;
        let bz = debris_z_top - lcg(&mut seed) * 380.0;
        let by = (lcg(&mut seed) - 0.5) * 55.0;

        // Larger pieces than original
        let bl = 60.0  + lcg(&mut seed) * 130.0;
        let bw = 30.0  + lcg(&mut seed) * 55.0;
        let bh = 50.0  + lcg(&mut seed) * 110.0;

        // Use nalgebra to compute a random 3-D rotation, then extract Euler angles
        let ax = (lcg(&mut seed) - 0.5) * 2.0;
        let ay = (lcg(&mut seed) - 0.5) * 2.0;
        let az = (lcg(&mut seed) - 0.5) * 2.0;
        let axis_vec = Vector3::new(ax, ay, az);
        let angle    = lcg(&mut seed) * PI * 0.9;

        let (rx_e, ry_e, rz_e) = if let Some(unit_ax) = Unit::try_new(axis_vec, 1e-9) {
            let rot = Rotation3::from_axis_angle(&unit_ax, angle);
            let (r, p_ang, y_ang) = rot.euler_angles();   // (roll, pitch, yaw) = (rx, ry, rz)
            (r.to_degrees(), p_ang.to_degrees(), y_ang.to_degrees())
        } else {
            ((lcg(&mut seed) - 0.5) * 60.0,
             (lcg(&mut seed) - 0.5) * 60.0,
             (lcg(&mut seed) - 0.5) * 60.0)
        };

        let dark = (0.30 + lcg(&mut seed) * 0.18) as f32;

        // A portion of the debris gets copper colouring
        let is_copper = (i % 5) == 0;
        let color = if is_copper {
            (0.58_f32, 0.35, 0.15)
        } else {
            (dark, dark + 0.02, dark + 0.05)
        };

        v.push(FoundationSpec {
            label:     format!("Foundation_Debris{:02}", i + 1),
            part_type: "debris".into(),
            x: bx, y: by, z: bz,
            length: bl, width: bw, height: bh,
            rot_x: rx_e, rot_y: ry_e, rot_z: rz_e,
            color,
        });
    }

    v
}

// ═══════════════════════════════════════════════════════════════════════════════
// § 5  Combined factory
// ═══════════════════════════════════════════════════════════════════════════════
//
// Return type change: frame is now Vec<ShardSpec> (faceted boxes)
// instead of Vec<EllipseFrameSpec> (smooth ellipsoid cut).

pub fn make_all_parts(p: &ShardParams) -> (
    Vec<ShardSpec>,          // frame facets
    Vec<SpikeSpec>,          // diamond cones + outer spikes
    Vec<ShardSpec>,          // inner blades + core shards
    Vec<FoundationSpec>,     // V-plates + debris
) {
    let frame = make_frame(p);

    let (mut all_spikes, shards) = if p.compute_core {
        make_core(p)
    } else {
        (Vec::new(), Vec::new())
    };

    let outer_spikes = if p.compute_spikes {
        make_spikes(p)
    } else {
        Vec::new()
    };
    all_spikes.extend(outer_spikes);

    let foundation = if p.compute_foundation {
        make_foundation(p)
    } else {
        Vec::new()
    };

    (frame, all_spikes, shards, foundation)
}
