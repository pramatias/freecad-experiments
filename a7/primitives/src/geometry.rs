// /home/emporas/repos/freecad/rust/a7/primitives/src/geometry.rs
// /home/emporas/repos/freecad/rust/a7/primitives/src/geometry.rs
use crate::types::{A7Params, ChassisPartSpec, BodyPartSpec, WheelSpec, MechanicalPartSpec};

// ═══════════════════════════════════════════════════════════════════════════════
// § 1  Chassis
// ═══════════════════════════════════════════════════════════════════════════════
//
// Ladder-frame layout.  The rails extend 200 mm beyond the rear axle and
// 120 mm ahead of the front axle so the bare frame is visible under the body.
//
//   ◄──── rear overhang ────┼──── wheelbase ────┼── front overhang ──►
//   x = -(wb + 200)         x = -wb             x = 0             x = 120

pub fn make_chassis_parts(p: &A7Params) -> Vec<ChassisPartSpec> {
    let half   = p.track_front / 2.0;
    let cz     = p.chassis_z;

    // Rail section (C-channel profile approximated as a solid rectangle)
    let rw = 55.0;   // Y-width of one rail
    let rh = 90.0;   // Z-height of one rail

    // Rail inner edge: ±(half − 80) from centreline, leaving ~700 mm gap
    let rail_inner_y  =  half - rw - 80.0;   // left  rail inner face (positive Y)
    let rail_outer_yr = -(half - 80.0);        // right rail outer face

    // Total rail length: wheelbase + front overhang + rear overhang
    let front_overhang = 120.0;
    let rear_overhang  = 200.0;
    let rail_len       = p.wheelbase + front_overhang + rear_overhang;
    let rail_x_start   = -(p.wheelbase + rear_overhang);

    // Axle-beam section
    let bw = 60.0;
    let bh = 55.0;
    let span = half + 60.0;   // full Y half-span (to kingpin stub)

    vec![
        // ── Front axle beam (solid I-beam, ahead of front rails) ─────────────
        ChassisPartSpec {
            label:     "Chassis_FrontAxle".into(),
            part_type: "axle_beam".into(),
            x: -bw / 2.0, y: -span, z: cz - bh / 2.0,
            length: bw, width: span * 2.0, height: bh,
        },
        // ── Rear axle beam ────────────────────────────────────────────────────
        ChassisPartSpec {
            label:     "Chassis_RearAxle".into(),
            part_type: "axle_beam".into(),
            x: -p.wheelbase - bw / 2.0, y: -span, z: cz - bh / 2.0,
            length: bw, width: span * 2.0, height: bh,
        },
        // ── Left longitudinal rail ────────────────────────────────────────────
        ChassisPartSpec {
            label:     "Chassis_RailLeft".into(),
            part_type: "rail".into(),
            x: rail_x_start, y: rail_inner_y, z: cz,
            length: rail_len, width: rw, height: rh,
        },
        // ── Right longitudinal rail ───────────────────────────────────────────
        ChassisPartSpec {
            label:     "Chassis_RailRight".into(),
            part_type: "rail".into(),
            x: rail_x_start, y: rail_outer_yr, z: cz,
            length: rail_len, width: rw, height: rh,
        },
        // ── Front bulkhead cross-member (behind front axle) ───────────────────
        ChassisPartSpec {
            label:     "Chassis_FrontBulkhead".into(),
            part_type: "cross_member".into(),
            x: -20.0, y: -(half - 80.0), z: cz,
            length: 20.0, width: (half - 80.0) * 2.0, height: 180.0,
        },
        // ── Mid cross-member (between cowl and tub) ───────────────────────────
        ChassisPartSpec {
            label:     "Chassis_MidCrossMember".into(),
            part_type: "cross_member".into(),
            x: -390.0, y: -(half - 80.0), z: cz,
            length: 20.0, width: (half - 80.0) * 2.0, height: 140.0,
        },
        // ── Rear cross-member ─────────────────────────────────────────────────
        ChassisPartSpec {
            label:     "Chassis_RearCrossMember".into(),
            part_type: "cross_member".into(),
            x: -(p.wheelbase + 20.0), y: -(half - 80.0), z: cz,
            length: 20.0, width: (half - 80.0) * 2.0, height: 100.0,
        },
    ]
}

// ═══════════════════════════════════════════════════════════════════════════════
// § 2  Bodywork — two separate hull design
// ═══════════════════════════════════════════════════════════════════════════════
//
// Hull A — Front Cowl  (behind the radiator / engine)
//   Narrower (~750 mm), lower profile, approximated as a box.
//   Front face at x = –80, rear face at x = –380.
//
// Hull B — Seating Tub  (driver's bucket)
//   Wider (~880 mm), deeper sides.
//   Front face at x = –400, rear face at x = –950.
//   The 20 mm gap between the two hulls (x –380 to –400) is intentional.
//
// Both hulls sit ON the chassis rails (z = body_floor_z) and are wide enough
// to overlap the rail tops, making the rails visible at front and rear.

pub fn make_body_parts(p: &A7Params) -> Vec<BodyPartSpec> {
    let bfz = p.body_floor_z;

    let half_cowl: f64 = 375.0;   // full width 750 mm
    let half_tub:  f64 = 440.0;   // full width 880 mm

    // Hull A — now 600 mm long (was 300)
    let cowl_x_front = -80.0;
    let cowl_length  = 600.0;
    let cowl_x_rear  = cowl_x_front - cowl_length;   // = –680

    // Hull B — 15 mm gap, 1200 mm long (was 20 mm gap, 550 mm)
    let hull_gap    = 15.0;
    let tub_x_front = cowl_x_rear - hull_gap;         // = –695
    let tub_length  = 1200.0;                         // rear face at –1895

    vec![
        // Radiator shell — unchanged
        BodyPartSpec {
            label:         "Body_RadiatorShell".into(),
            part_type:     "radiator_shell".into(),
            x: 80.0, y: -185.0, z: bfz,
            length: 110.0, width: 370.0, height: 400.0,
            fillet_radius: 60.0,
            color: (0.75, 0.75, 0.75),
        },
        // Hull A: Front Cowl — 600 mm long
        BodyPartSpec {
            label:         "Body_CowlFront".into(),
            part_type:     "cowl".into(),
            x: cowl_x_front, y: -half_cowl, z: bfz,
            length: cowl_length, width: half_cowl * 2.0, height: 280.0,
            fillet_radius: 25.0,
            color: (0.72, 0.72, 0.72),
        },
        // Hull B: Seating Tub — 1200 mm long, wider than cowl
        BodyPartSpec {
            label:         "Body_SeatTub".into(),
            part_type:     "seat_tub".into(),
            x: tub_x_front, y: -half_tub, z: bfz,
            length: tub_length, width: half_tub * 2.0, height: 330.0,
            fillet_radius: 15.0,
            color: (0.72, 0.72, 0.72),
        },
        // Floor pan — full wheelbase length
        BodyPartSpec {
            label:         "Body_FloorPan".into(),
            part_type:     "floor_pan".into(),
            x: -p.wheelbase, y: -half_tub, z: bfz - 18.0,
            length: p.wheelbase, width: half_tub * 2.0, height: 18.0,
            fillet_radius: 0.0,
            color: (0.58, 0.58, 0.58),
        },
        // Windshield — thin slab sitting on top of the cowl, near its rear edge
        // Width matches the cowl; angled post-processing can tilt it later.
        BodyPartSpec {
            label:         "Body_Windshield".into(),
            part_type:     "windshield".into(),
            x: cowl_x_front - 480.0, y: -half_cowl + 60.0, z: bfz + 280.0,
            length: 18.0, width: (half_cowl - 60.0) * 2.0, height: 150.0,
            fillet_radius: 10.0,
            color: (0.82, 0.88, 0.92),
        },
    ]
}
// ═══════════════════════════════════════════════════════════════════════════════
// § 3  Wheels — narrow, tall, motorcycle-style wire-spoke assemblies
// ═══════════════════════════════════════════════════════════════════════════════
//
// Geometry recap (all from A7Params):
//   outer_radius  ~ 350 mm  →  outer diameter ~ 700 mm  (vintage tall wheel)
//   tire_section  ~ 105 mm  →  narrow cross-section (bicycle/motorcycle feel)
//   rim_width     ~  95 mm  →  narrow rim
//   torus major r = outer_radius − tire_section = 245 mm
//   torus minor r = tire_section = 105 mm  (outer surface reaches outer_radius ✓)

pub fn make_wheels(p: &A7Params) -> Vec<WheelSpec> {
    let hf = p.track_front / 2.0;
    let hr = p.track_rear  / 2.0;
    let cz = p.wheel_radius;   // wheel centre Z = outer radius (sits on ground)

    [
        ("FL",  0.0,           hf),
        ("FR",  0.0,          -hf),
        ("RL", -p.wheelbase,   hr),
        ("RR", -p.wheelbase,  -hr),
    ]
    .into_iter()
    .map(|(pos, cx, cy)| WheelSpec {
        label:        format!("Wheel_{pos}"),
        position:     pos.into(),
        cx, cy, cz,
        outer_radius: p.wheel_radius,
        hub_radius:   p.hub_radius,
        tire_section: p.tire_section,
        spoke_count:  p.spoke_count,
        rim_width:    p.rim_width,
    })
    .collect()
}

// ═══════════════════════════════════════════════════════════════════════════════
// § 4  Mechanical (engine + steering + interior)
// ═══════════════════════════════════════════════════════════════════════════════

pub fn make_mechanical_parts(p: &A7Params) -> Vec<MechanicalPartSpec> {
    let bfz = p.body_floor_z;
    let cz  = p.chassis_z;
    let mut v: Vec<MechanicalPartSpec> = Vec::new();

    // ── Engine ────────────────────────────────────────────────────────────────
    // Sits forward of the front bulkhead, mostly exposed under the cowl arch.
    if p.compute_engine {
        let block_z   = cz + 80.0;
        let block_top = block_z + 250.0;

// § 4  Seat — centred inside the seating tub
// tub runs from –695 to –1895; place seat at ~60 % of wheelbase back
v.push(MechanicalPartSpec {
    label:     "Interior_Seat".into(),
    part_type: "seat".into(),
    x: -(p.wheelbase * 0.60),   // ≈ –1143 — comfortably inside the tub
    y: -140.0, z: bfz + 18.0,   // raised 18 mm above the floor pan surface
    length: 330.0, width: 280.0, height: 65.0,
    angle_deg: 0.0,
    color: (0.35, 0.25, 0.18),
});

        // Cooling fins — 4 × thin transverse slab on top of the block
        let fin_pitch = 320.0 / 5.0;
        for i in 1..=4u32 {
            v.push(MechanicalPartSpec {
                label:     format!("Engine_Fin{i}"),
                part_type: "engine_fin".into(),
                x: 30.0, y: -175.0 + fin_pitch * i as f64 - 7.5, z: block_top,
                length: 310.0, width: 15.0, height: 30.0,
                angle_deg: 0.0,
                color: (0.35, 0.35, 0.35),
            });
        }

        // Spark plugs — 4 × small cylinder evenly spaced along block top
        for i in 0..4u32 {
            v.push(MechanicalPartSpec {
                label:     format!("Engine_SparkPlug{}", i + 1),
                part_type: "spark_plug".into(),
                x: 55.0 + 65.0 * i as f64, y: -10.0, z: block_top,
                length: 8.0,   // radius
                width:  8.0,
                height: 50.0,  // cylinder height
                angle_deg: 0.0,
                color: (0.85, 0.85, 0.15),
            });
        }
    }

    // ── Steering column ───────────────────────────────────────────────────────
    let col_angle_rad = 30_f64.to_radians();
    v.push(MechanicalPartSpec {
        label:     "Steering_Column".into(),
        part_type: "steering_column".into(),
        x: -120.0, y: 30.0, z: cz + 200.0,
        length: 420.0, width: 12.0, height: 12.0,
        angle_deg: 30.0,
        color: (0.55, 0.55, 0.55),
    });

    // ── Steering wheel ────────────────────────────────────────────────────────
    v.push(MechanicalPartSpec {
        label:     "Steering_Wheel".into(),
        part_type: "steering_wheel".into(),
        x: -120.0 - 420.0 * col_angle_rad.sin(),
        y: 30.0,
        z: cz + 200.0 + 420.0 * col_angle_rad.cos(),
        length: 0.0,
        width:  8.0,    // torus minor (tube) radius
        height: 190.0,  // torus major radius
        angle_deg: 30.0,
        color: (0.22, 0.22, 0.22),
    });

    // ── Seat ──────────────────────────────────────────────────────────────────
    // Placed inside the Seating Tub (centred at ~x –675)
    v.push(MechanicalPartSpec {
        label:     "Interior_Seat".into(),
        part_type: "seat".into(),
        x: -(p.wheelbase * 0.50), y: -140.0, z: bfz,
        length: 330.0, width: 280.0, height: 65.0,
        angle_deg: 0.0,
        color: (0.35, 0.25, 0.18),
    });

    v
}

// ═══════════════════════════════════════════════════════════════════════════════
// § 5  Combined factory
// ═══════════════════════════════════════════════════════════════════════════════

pub fn make_all_parts(p: &A7Params) -> (
    Vec<ChassisPartSpec>,
    Vec<BodyPartSpec>,
    Vec<WheelSpec>,
    Vec<MechanicalPartSpec>,
) {
    (
        make_chassis_parts(p),
        make_body_parts(p),
        make_wheels(p),
        make_mechanical_parts(p),
    )
}
