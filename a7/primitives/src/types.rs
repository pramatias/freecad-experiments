// /home/emporas/repos/freecad/rust/a7/primitives/src/types.rs
// /home/emporas/repos/freecad/rust/a7/primitives/src/types.rs
use pyo3::prelude::*;

// ═══════════════════════════════════════════════════════════════════════════════
// § 1  Build parameters
// ═══════════════════════════════════════════════════════════════════════════════

/// All linear dimensions in millimetres.
///
/// Coordinate origin: centre of the front axle at ground level (Z = 0).
///   X+  = forward (toward the front of the car)
///   Y+  = left (outboard – driver side)
///   Z+  = upward
///
/// Wheel geometry note
/// ───────────────────
/// The outer tyre diameter is 2 × `wheel_radius` (~700 mm → tall vintage wheel).
/// The torus major radius passed to FreeCAD = `wheel_radius − tire_section`.
/// The torus minor radius (tube) = `tire_section`.
/// Keeping `tire_section` ≤ 110 mm gives the narrow, motorcycle-style profile
/// of the original Austin Seven.
#[pyclass]
#[derive(Clone)]
pub struct A7Params {
    #[pyo3(get, set)] pub wheelbase:      f64,  // front-to-rear axle  = 1905
    #[pyo3(get, set)] pub track_front:    f64,  // centre-to-centre    = 1016
    #[pyo3(get, set)] pub track_rear:     f64,  //                     = 1016
    /// Outer radius of the tyre (ground to wheel centre = this value).
    /// Default 350 mm → outer Ø 700 mm, matching the vintage tall wheel.
    #[pyo3(get, set)] pub wheel_radius:   f64,
    /// Tyre cross-section height (= torus minor radius).
    /// Keep ≤ 110 mm for the narrow period-correct profile.
    #[pyo3(get, set)] pub tire_section:   f64,
    /// Rim / tyre width.  ~95 mm gives the narrow look; wider values suit race
    /// tyres.
    #[pyo3(get, set)] pub rim_width:      f64,
    #[pyo3(get, set)] pub hub_radius:     f64,  // central hub r       = 35
    #[pyo3(get, set)] pub spoke_count:    u32,  // wire spokes / wheel = 24
    #[pyo3(get, set)] pub chassis_z:      f64,  // rail bottom Z       = 120
    #[pyo3(get, set)] pub body_floor_z:   f64,  // floor top Z         = 280
    #[pyo3(get, set)] pub compute_engine: bool,
}

#[pymethods]
    impl A7Params {
    #[new]
    #[pyo3(signature = (
        wheelbase      = 1905.0,
        track_front    = 1016.0,
        track_rear     = 1016.0,
        wheel_radius   =  355.0,   // outer Ø 710 mm per spec (was 350)
        tire_section   =  100.0,   // narrow vintage section   (was 105)
        rim_width      =   95.0,   // narrow rim
        hub_radius     =   60.0,   // hub Ø 120 mm per spec    (was 35)
        spoke_count    =     28,   // 28 wire spokes per spec  (was 24)
        chassis_z      =  120.0,
        body_floor_z   =  280.0,
        compute_engine =   true,
    ))]

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        wheelbase: f64, track_front: f64, track_rear: f64,
        wheel_radius: f64, tire_section: f64, rim_width: f64,
        hub_radius: f64, spoke_count: u32,
        chassis_z: f64, body_floor_z: f64,
        compute_engine: bool,
    ) -> Self {
        Self {
            wheelbase, track_front, track_rear,
            wheel_radius, tire_section, rim_width,
            hub_radius, spoke_count,
            chassis_z, body_floor_z,
            compute_engine,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// § 2  Spec types  (Rust → FreeCAD)
// ═══════════════════════════════════════════════════════════════════════════════

/// Axle beams, longitudinal rails, cross-members — all rendered as boxes.
#[pyclass]
pub struct ChassisPartSpec {
    #[pyo3(get)] pub label:     String,
    #[pyo3(get)] pub part_type: String,   // "axle_beam" | "rail" | "cross_member"
    #[pyo3(get)] pub x:         f64,      // min-X corner of bounding box
    #[pyo3(get)] pub y:         f64,
    #[pyo3(get)] pub z:         f64,
    #[pyo3(get)] pub length:    f64,      // X extent
    #[pyo3(get)] pub width:     f64,      // Y extent
    #[pyo3(get)] pub height:    f64,      // Z extent
}

/// Body hulls and panels — all rendered as boxes.
///
/// Two-hull design
/// ───────────────
/// The Austin Seven body is modelled as two separate hulls dropped onto the
/// frame, with a visible physical gap between them:
///
/// | part_type        | Description                                              |
/// |------------------|----------------------------------------------------------|
/// | "radiator_shell" | Tombstone-shaped frontal shell; `fillet_radius` ≥ 50 mm |
/// |                  | guides a post-processing top-corner Fillet/Loft.         |
/// | "cowl"           | Hull A — Front Cowl (behind engine, width ~750 mm).      |
/// |                  | Narrower than the tub; slight forward-taper hint via      |
/// |                  | `fillet_radius`.                                          |
/// | "seat_tub"       | Hull B — Seating Tub (driver bucket, width ~880 mm).     |
/// |                  | Wider than the cowl; 20 mm gap separates it from Hull A. |
/// | "floor_pan"      | Thin slab under both hulls; chassis rails protrude at    |
/// |                  | both ends so the frame is visible.                        |
///
/// `fillet_radius` is a hint for post-processing in FreeCAD (not applied
/// automatically; requires an explicit Chamfer or Fillet operation).
#[pyclass]
pub struct BodyPartSpec {
    #[pyo3(get)] pub label:         String,
    #[pyo3(get)] pub part_type:     String,
    #[pyo3(get)] pub x:             f64,
    #[pyo3(get)] pub y:             f64,
    #[pyo3(get)] pub z:             f64,
    #[pyo3(get)] pub length:        f64,
    #[pyo3(get)] pub width:         f64,
    #[pyo3(get)] pub height:        f64,
    #[pyo3(get)] pub fillet_radius: f64,
    #[pyo3(get)] pub color:         (f32, f32, f32),
}

/// One complete wheel assembly.
/// The macro creates a torus (tyre), a cylinder (hub) and spoke cylinders.
///
/// Tyre torus geometry
/// ───────────────────
///   torus major radius = outer_radius − tire_section
///   torus minor radius = tire_section
///   → outer tyre surface reaches exactly outer_radius from wheel centre.
///
/// For the vintage narrow profile: outer_radius ≈ 350 mm, tire_section ≈ 105 mm.
#[pyclass]
pub struct WheelSpec {
    #[pyo3(get)] pub label:        String,
    #[pyo3(get)] pub position:     String,   // "FL" | "FR" | "RL" | "RR"
    #[pyo3(get)] pub cx:           f64,      // wheel-centre X
    #[pyo3(get)] pub cy:           f64,      // wheel-centre Y (axle axis)
    #[pyo3(get)] pub cz:           f64,      // wheel-centre Z = wheel_radius
    #[pyo3(get)] pub outer_radius: f64,
    #[pyo3(get)] pub hub_radius:   f64,
    #[pyo3(get)] pub tire_section: f64,
    #[pyo3(get)] pub spoke_count:  u32,
    #[pyo3(get)] pub rim_width:    f64,
}

/// Engine, steering and interior parts — dispatched by `part_type` in the macro.
///
/// | part_type          | FreeCAD primitive            | dimension notes                            |
/// |--------------------|------------------------------|--------------------------------------------|
/// | "engine_block"     | Part::Box                    | length/width/height = extents              |
/// | "engine_fin"       | Part::Box (thin, repeated)   | same                                       |
/// | "spark_plug"       | Part::Cylinder               | `length` = radius, `height` = cyl height   |
/// | "steering_wheel"   | Part::Torus                  | `height` = major r, `width` = minor r      |
/// | "steering_column"  | Part::Cylinder               | `length` = cyl height, `width` = radius,   |
/// |                    |                              | `angle_deg` = lean from Z toward -X        |
/// | "seat"             | Part::Box                    | length/width/height = extents              |
#[pyclass]
pub struct MechanicalPartSpec {
    #[pyo3(get)] pub label:     String,
    #[pyo3(get)] pub part_type: String,
    #[pyo3(get)] pub x:         f64,
    #[pyo3(get)] pub y:         f64,
    #[pyo3(get)] pub z:         f64,
    #[pyo3(get)] pub length:    f64,
    #[pyo3(get)] pub width:     f64,
    #[pyo3(get)] pub height:    f64,
    #[pyo3(get)] pub angle_deg: f64,
    #[pyo3(get)] pub color:     (f32, f32, f32),
}
