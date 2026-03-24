// pump/src/types.rs
use pyo3::prelude::*;

// ═══════════════════════════════════════════════════════════════════════════════
// § 1  Build parameters
// ═══════════════════════════════════════════════════════════════════════════════

/// Global build parameters for the coaxial syringe-style pump.
///
/// Coordinate convention (FreeCAD / nalgebra):
///   X+ = right   Y+ = depth (into screen)   Z+ = axial / upward
///
/// **Origin**: geometric centre of the barrel, both radially and axially.
/// Every other component is offset from there along +Z or −Z.
///
/// Wall thickness   = barrel_outer_r − barrel_inner_r   (≈ 3.5 mm default)
/// Rod clearance    = cap_bore_r − rod_r                (≈ 0.5 mm default)
/// Piston clearance = barrel_inner_r − piston_r         (≈ 0.1 mm default)
#[pyclass]
#[derive(Clone, Debug)]
pub struct PumpParams {
    // ── Barrel ────────────────────────────────────────────────────────────────
    /// Outer radius of the main barrel tube.
    #[pyo3(get, set)] pub barrel_outer_r: f64,   // = 30.0 mm
    /// Inner radius (bore) of the main barrel tube.
    #[pyo3(get, set)] pub barrel_inner_r: f64,   // = 26.5 mm
    /// Full axial length of the barrel (between the two end caps).
    #[pyo3(get, set)] pub barrel_length:  f64,   // = 200.0 mm

    // ── End caps ──────────────────────────────────────────────────────────────
    /// Outer radius of each end cap — slightly larger than the barrel to form
    /// the retaining lip.
    #[pyo3(get, set)] pub cap_outer_r:    f64,   // = 34.0 mm
    /// Axial thickness of each end cap disc.
    #[pyo3(get, set)] pub cap_thickness:  f64,   // = 12.0 mm
    /// Radius of the central through-hole in each cap that the rod passes through.
    #[pyo3(get, set)] pub cap_bore_r:     f64,   // = 5.5 mm

    // ── Plunger rod ───────────────────────────────────────────────────────────
    /// Radius of the plunger rod shaft.
    #[pyo3(get, set)] pub rod_r:          f64,   // = 5.0 mm
    /// How far the rod protrudes past each end cap on the exterior.
    #[pyo3(get, set)] pub rod_overhang:   f64,   // = 30.0 mm

    // ── Internal piston ───────────────────────────────────────────────────────
    /// Radius of the piston disc — must be ≤ barrel_inner_r.
    #[pyo3(get, set)] pub piston_r:       f64,   // = 26.4 mm
    /// Axial height (thickness) of the piston disc.
    #[pyo3(get, set)] pub piston_h:       f64,   // = 10.0 mm
    /// Axial offset of the piston bottom-face from the barrel's bottom face.
    /// Set this to vary the "stroke" position shown in the model.
    #[pyo3(get, set)] pub stroke:         f64,   // = 40.0 mm

    // ── Thumb press — right / positive-Z terminal ─────────────────────────────
    /// Radius of the thumb-press disc at the push end of the rod.
    #[pyo3(get, set)] pub thumb_r:        f64,   // = 18.0 mm
    /// Axial thickness of the thumb-press disc.
    #[pyo3(get, set)] pub thumb_h:        f64,   // = 5.0 mm

    // ── Nozzle / stem — left / negative-Z terminal ────────────────────────────
    /// Outer radius of the nozzle body step (the wider section).
    #[pyo3(get, set)] pub nozzle_body_r:  f64,   // = 9.0 mm
    /// Axial length of the nozzle body step.
    #[pyo3(get, set)] pub nozzle_body_l:  f64,   // = 20.0 mm
    /// Outer radius of the narrow outlet tip that follows the nozzle body.
    #[pyo3(get, set)] pub nozzle_tip_r:   f64,   // = 4.5 mm
    /// Axial length of the outlet tip.
    #[pyo3(get, set)] pub nozzle_tip_l:   f64,   // = 10.0 mm
    /// Radius of the terminal flange disc at the very end of the nozzle.
    #[pyo3(get, set)] pub nozzle_flange_r: f64,  // = 13.0 mm
    /// Axial thickness of the nozzle flange disc.
    #[pyo3(get, set)] pub nozzle_flange_t: f64,  // = 4.0 mm

    // ── Feature toggles ───────────────────────────────────────────────────────
    /// Render the inner bore cylinder (transparent, shows internal volume).
    #[pyo3(get, set)] pub show_bore:      bool,
    /// Render the internal piston.
    #[pyo3(get, set)] pub show_piston:    bool,
    /// Render the nozzle assembly (body + tip + flange).
    #[pyo3(get, set)] pub show_nozzle:    bool,
}

impl PumpParams {
    pub fn default_params() -> Self {
        Self::new(
            30.0, 26.5, 200.0,   // barrel
            34.0, 12.0,  5.5,    // caps
             5.0, 30.0,          // rod
            26.4, 10.0, 40.0,    // piston
            18.0,  5.0,          // thumb
             9.0, 20.0, 4.5, 10.0, 13.0, 4.0,  // nozzle
            true, true, true,
        )
    }
}

#[pymethods]
impl PumpParams {
    #[new]
    #[pyo3(signature = (
        barrel_outer_r   = 30.0,
        barrel_inner_r   = 26.5,
        barrel_length    = 200.0,
        cap_outer_r      = 34.0,
        cap_thickness    = 12.0,
        cap_bore_r       = 5.5,
        rod_r            = 5.0,
        rod_overhang     = 30.0,
        piston_r         = 26.4,
        piston_h         = 10.0,
        stroke           = 40.0,
        thumb_r          = 18.0,
        thumb_h          = 5.0,
        nozzle_body_r    = 9.0,
        nozzle_body_l    = 20.0,
        nozzle_tip_r     = 4.5,
        nozzle_tip_l     = 10.0,
        nozzle_flange_r  = 13.0,
        nozzle_flange_t  = 4.0,
        show_bore        = true,
        show_piston      = true,
        show_nozzle      = true,
    ))]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        barrel_outer_r: f64, barrel_inner_r: f64, barrel_length: f64,
        cap_outer_r: f64, cap_thickness: f64, cap_bore_r: f64,
        rod_r: f64, rod_overhang: f64,
        piston_r: f64, piston_h: f64, stroke: f64,
        thumb_r: f64, thumb_h: f64,
        nozzle_body_r: f64, nozzle_body_l: f64,
        nozzle_tip_r: f64, nozzle_tip_l: f64,
        nozzle_flange_r: f64, nozzle_flange_t: f64,
        show_bore: bool, show_piston: bool, show_nozzle: bool,
    ) -> Self {
        Self {
            barrel_outer_r, barrel_inner_r, barrel_length,
            cap_outer_r, cap_thickness, cap_bore_r,
            rod_r, rod_overhang,
            piston_r, piston_h, stroke,
            thumb_r, thumb_h,
            nozzle_body_r, nozzle_body_l,
            nozzle_tip_r, nozzle_tip_l,
            nozzle_flange_r, nozzle_flange_t,
            show_bore, show_piston, show_nozzle,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// § 2  Spec types (Rust → FreeCAD)
// ═══════════════════════════════════════════════════════════════════════════════

/// A cylindrical primitive — maps directly to FreeCAD `Part::Cylinder`.
///
/// `x`, `y`, `z` is the **centre of the bottom face** of the cylinder.
/// The cylinder extends along **+Z** for `height` mm.
/// `transparency` is a FreeCAD value 0 (opaque) – 100 (invisible).
///
/// | part_type        | Component                              |
/// |------------------|----------------------------------------|
/// | barrel_wall      | Main barrel outer shell                |
/// | barrel_bore      | Inner bore volume (rendered ghost)     |
/// | cap_left         | Left (nozzle-side) end cap body        |
/// | cap_right        | Right (thumb-side) end cap body        |
/// | rod              | Full-length plunger rod shaft          |
/// | piston           | Internal piston disc                   |
/// | thumb_press      | Thumb disc at rod's positive-Z end     |
/// | nozzle_body      | Wider stepped section of the nozzle    |
/// | nozzle_tip       | Narrow outlet tube of the nozzle       |
/// | nozzle_flange    | Terminal flange disc at nozzle end     |
#[pyclass]
#[derive(Clone, Debug)]
pub struct CylinderSpec {
    #[pyo3(get)] pub label:        String,
    #[pyo3(get)] pub part_type:    String,
    /// Bottom-face centre — X coordinate (always 0 for coaxial parts).
    #[pyo3(get)] pub x:            f64,
    /// Bottom-face centre — Y coordinate (always 0 for coaxial parts).
    #[pyo3(get)] pub y:            f64,
    /// Bottom-face centre — Z coordinate.
    #[pyo3(get)] pub z:            f64,
    #[pyo3(get)] pub radius:       f64,
    #[pyo3(get)] pub height:       f64,
    #[pyo3(get)] pub color:        (f32, f32, f32),
    /// FreeCAD ViewObject transparency: 0 = solid, 85 = nearly transparent.
    #[pyo3(get)] pub transparency: u8,
}

/// Clearance diagnostics surfaced back to Python after geometry is built.
///
/// All distances computed by nalgebra; Python prints them as warnings.
#[pyclass]
#[derive(Clone, Debug)]
pub struct ClearanceReport {
    /// barrel_inner_r − piston_r  (should be ≥ 0.05 mm)
    #[pyo3(get)] pub piston_radial_gap:   f64,
    /// cap_bore_r − rod_r  (should be ≥ 0.3 mm)
    #[pyo3(get)] pub rod_bore_gap:        f64,
    /// True if piston bottom-face is inside barrel axially.
    #[pyo3(get)] pub piston_in_barrel:    bool,
    /// True if piston top-face is inside barrel axially.
    #[pyo3(get)] pub piston_top_in_barrel: bool,
    /// Full axial length of the assembly (nozzle flange → thumb top).
    #[pyo3(get)] pub total_axial_length:  f64,
    /// Any geometry violations as human-readable strings.
    #[pyo3(get)] pub warnings:            Vec<String>,
}
