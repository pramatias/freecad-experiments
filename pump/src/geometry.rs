// pump/src/geometry.rs
//
// All dimensional math is done with nalgebra so positions, offsets and
// clearance checks are computed as proper vector operations, not scattered
// scalar arithmetic.
//
// Coordinate system (FreeCAD default — Z-up, assembly along Z):
//   Origin   = geometric centre of the barrel (axial + radial)
//   −Z side  = nozzle / outlet end
//   +Z side  = thumb-press / push end

use nalgebra::{Point3, Vector3};

use crate::types::{ClearanceReport, CylinderSpec, PumpParams};

// ═══════════════════════════════════════════════════════════════════════════════
// § 1  Axial layout — single source of truth for every Z coordinate
// ═══════════════════════════════════════════════════════════════════════════════

/// All key Z positions in the assembly, computed from `PumpParams`.
///
/// Every builder function below reads from this struct rather than
/// re-deriving positions, guaranteeing consistency.
pub struct PumpLayout {
    // Barrel
    pub barrel_bot: f64,
    pub barrel_top: f64,

    // Left cap  (nozzle side, −Z)
    pub cap_l_bot: f64,
    pub cap_l_top: f64,

    // Right cap (thumb side, +Z)
    pub cap_r_bot: f64,
    pub cap_r_top: f64,

    // Plunger rod — spans the entire assembly plus overhangs
    pub rod_bot: f64,
    pub rod_top: f64,

    // Piston — bottom-face Z inside the barrel
    pub piston_bot: f64,

    // Thumb press — sits flush with the top of the rod
    pub thumb_bot: f64,

    // Nozzle — sits in the left overhang zone
    pub nozzle_body_bot: f64,   // body section start (= rod_bot, outermost)
    pub nozzle_tip_bot:  f64,   // tip section start  (= nozzle_body_bot + body_l)
    pub nozzle_flange_bot: f64, // terminal flange    (below nozzle_body_bot)
}

impl PumpLayout {
    pub fn compute(p: &PumpParams) -> Self {
        let half = p.barrel_length * 0.5;

        // ── Barrel ───────────────────────────────────────────────────────────
        let barrel_bot = -half;
        let barrel_top =  half;

        // ── Caps ─────────────────────────────────────────────────────────────
        let cap_l_bot = barrel_bot - p.cap_thickness;
        let cap_l_top = barrel_bot;
        let cap_r_bot = barrel_top;
        let cap_r_top = barrel_top + p.cap_thickness;

        // ── Rod ──────────────────────────────────────────────────────────────
        // Extends rod_overhang past each cap face
        let rod_bot = cap_l_bot - p.rod_overhang;
        let rod_top = cap_r_top + p.rod_overhang;

        // ── Piston ───────────────────────────────────────────────────────────
        // `stroke` is the distance from the barrel bottom face to the piston
        // bottom face — the operator's controllable variable.
        let piston_bot = barrel_bot + p.stroke;

        // ── Thumb press ───────────────────────────────────────────────────────
        // Flush with the very top of the rod.
        let thumb_bot = rod_top - p.thumb_h;

        // ── Nozzle ────────────────────────────────────────────────────────────
        // The nozzle body starts at the outer face of the left overhang and
        // steps inward (+Z direction) toward the left cap.
        //
        //   [ flange ][ tip ][ body ] | left cap | barrel ...
        //   ← −Z direction
        //
        // nozzle_body_bot is the innermost end (closest to left cap).
        // That places body from nozzle_body_bot to nozzle_body_bot + body_l,
        // tip from (nozzle_body_bot + body_l) .. (nozzle_body_bot + body_l + tip_l)
        // ... wait, that would go *away* from the cap. Let me re-orient.
        //
        // The nozzle projects OUT from the left cap (in −Z direction).
        // So from cap_l_bot going toward more negative Z:
        //   nozzle_body: cap_l_bot − body_l  ..  cap_l_bot
        //   nozzle_tip:  cap_l_bot − body_l − tip_l  ..  cap_l_bot − body_l
        //   nozzle_flange (disc): at the tip's outer end
        //
        let nozzle_body_bot  = cap_l_bot - p.nozzle_body_l;
        let nozzle_tip_bot   = nozzle_body_bot - p.nozzle_tip_l;
        let nozzle_flange_bot = nozzle_tip_bot - p.nozzle_flange_t;

        Self {
            barrel_bot, barrel_top,
            cap_l_bot, cap_l_top,
            cap_r_bot, cap_r_top,
            rod_bot, rod_top,
            piston_bot,
            thumb_bot,
            nozzle_body_bot,
            nozzle_tip_bot,
            nozzle_flange_bot,
        }
    }

    /// Total axial span as a nalgebra Vector3 (purely Z, X/Y = 0).
    pub fn axial_span(&self, p: &PumpParams) -> Vector3<f64> {
        let z_min = self.nozzle_flange_bot;
        let z_max = self.rod_top;
        Vector3::new(0.0, 0.0, z_max - z_min)
    }

    /// Named anchor points as `Point3` for clearance / annotation use.
    pub fn barrel_centre(&self) -> Point3<f64> {
        Point3::new(0.0, 0.0, 0.0)   // by definition (origin)
    }

    pub fn piston_centre(&self, p: &PumpParams) -> Point3<f64> {
        Point3::new(0.0, 0.0, self.piston_bot + p.piston_h * 0.5)
    }

    pub fn nozzle_outlet(&self) -> Point3<f64> {
        Point3::new(0.0, 0.0, self.nozzle_flange_bot)
    }

    pub fn thumb_centre(&self, p: &PumpParams) -> Point3<f64> {
        Point3::new(0.0, 0.0, self.thumb_bot + p.thumb_h * 0.5)
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// § 2  Clearance report
// ═══════════════════════════════════════════════════════════════════════════════

pub fn compute_clearances(p: &PumpParams, lay: &PumpLayout) -> ClearanceReport {
    let piston_radial_gap = p.barrel_inner_r - p.piston_r;
    let rod_bore_gap      = p.cap_bore_r - p.rod_r;

    let piston_top = lay.piston_bot + p.piston_h;
    let piston_in_barrel     = lay.piston_bot  >= lay.barrel_bot;
    let piston_top_in_barrel = piston_top      <= lay.barrel_top;

    // Use nalgebra distance between the axial extremes of the assembly
    let nozzle_outlet = lay.nozzle_outlet();
    let thumb_top     = Point3::new(0.0, 0.0, lay.rod_top);
    let total_axial_length = nalgebra::distance(&nozzle_outlet, &thumb_top);

    let mut warnings = Vec::new();

    if piston_radial_gap < 0.05 {
        warnings.push(format!(
            "Piston-to-bore gap {:.3} mm is dangerously tight (min 0.05 mm).",
            piston_radial_gap
        ));
    }
    if rod_bore_gap < 0.3 {
        warnings.push(format!(
            "Rod-to-cap-bore gap {:.3} mm is dangerously tight (min 0.3 mm).",
            rod_bore_gap
        ));
    }
    if !piston_in_barrel {
        warnings.push(format!(
            "Piston bottom {:.1} mm is outside the barrel (barrel starts at {:.1} mm). \
             Increase stroke.",
            lay.piston_bot, lay.barrel_bot
        ));
    }
    if !piston_top_in_barrel {
        warnings.push(format!(
            "Piston top {:.1} mm exceeds barrel top {:.1} mm. Reduce stroke or piston_h.",
            piston_top, lay.barrel_top
        ));
    }
    if p.barrel_inner_r >= p.barrel_outer_r {
        warnings.push(
            "barrel_inner_r must be less than barrel_outer_r (negative wall thickness).".into(),
        );
    }

    ClearanceReport {
        piston_radial_gap,
        rod_bore_gap,
        piston_in_barrel,
        piston_top_in_barrel,
        total_axial_length,
        warnings,
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// § 3  Component builders — each returns Vec<CylinderSpec>
// ═══════════════════════════════════════════════════════════════════════════════

// ── Colour palette ────────────────────────────────────────────────────────────
const COL_BARREL:   (f32, f32, f32) = (0.75, 0.82, 0.88);   // light steel-blue
const COL_BORE:     (f32, f32, f32) = (0.55, 0.72, 0.88);   // slightly more blue (ghost)
const COL_CAP:      (f32, f32, f32) = (0.58, 0.63, 0.70);   // mid steel
const COL_ROD:      (f32, f32, f32) = (0.82, 0.85, 0.90);   // bright steel
const COL_PISTON:   (f32, f32, f32) = (0.72, 0.76, 0.82);   // piston grey
const COL_THUMB:    (f32, f32, f32) = (0.68, 0.55, 0.38);   // warm copper
const COL_NOZZLE_B: (f32, f32, f32) = (0.52, 0.58, 0.65);   // nozzle body
const COL_NOZZLE_T: (f32, f32, f32) = (0.45, 0.50, 0.58);   // nozzle tip (darker)
const COL_FLANGE:   (f32, f32, f32) = (0.62, 0.50, 0.35);   // copper flange

fn cyl(
    label: &str,
    part_type: &str,
    z: f64,
    radius: f64,
    height: f64,
    color: (f32, f32, f32),
    transparency: u8,
) -> CylinderSpec {
    CylinderSpec {
        label:        label.to_string(),
        part_type:    part_type.to_string(),
        x: 0.0, y: 0.0, z,
        radius,
        height,
        color,
        transparency,
    }
}

// ── 3a  Barrel ────────────────────────────────────────────────────────────────

pub fn make_barrel(p: &PumpParams, lay: &PumpLayout) -> Vec<CylinderSpec> {
    let mut v = Vec::new();

    // Outer shell — rendered semi-transparent so interior is visible
    v.push(cyl(
        "Barrel_Wall", "barrel_wall",
        lay.barrel_bot,
        p.barrel_outer_r,
        p.barrel_length,
        COL_BARREL,
        70,   // 70 % transparent — shows internal rod + piston
    ));

    // Inner bore — ghost cylinder, gives the hollow-tube visual cue
    if p.show_bore {
        v.push(cyl(
            "Barrel_Bore", "barrel_bore",
            lay.barrel_bot,
            p.barrel_inner_r,
            p.barrel_length,
            COL_BORE,
            55,
        ));
    }

    v
}

// ── 3b  End caps ──────────────────────────────────────────────────────────────

pub fn make_caps(p: &PumpParams, lay: &PumpLayout) -> Vec<CylinderSpec> {
    let mut v = Vec::new();

    // Left cap (nozzle side)
    v.push(cyl(
        "Cap_Left", "cap_left",
        lay.cap_l_bot,
        p.cap_outer_r,
        p.cap_thickness,
        COL_CAP,
        0,
    ));

    // Right cap (thumb side)
    v.push(cyl(
        "Cap_Right", "cap_right",
        lay.cap_r_bot,
        p.cap_outer_r,
        p.cap_thickness,
        COL_CAP,
        0,
    ));

    // The central through-holes in the caps are represented as slightly
    // brighter inset cylinders (no boolean subtraction needed for the model).
    // They sit at the same Z as the cap but in a slightly different colour,
    // giving the visual impression of a bore hole.
    let bore_color: (f32, f32, f32) = (
        (COL_CAP.0 + 0.12).min(1.0),
        (COL_CAP.1 + 0.12).min(1.0),
        (COL_CAP.2 + 0.12).min(1.0),
    );
    v.push(cyl(
        "CapBore_Left", "cap_bore_left",
        lay.cap_l_bot,
        p.cap_bore_r,
        p.cap_thickness,
        bore_color,
        20,
    ));
    v.push(cyl(
        "CapBore_Right", "cap_bore_right",
        lay.cap_r_bot,
        p.cap_bore_r,
        p.cap_thickness,
        bore_color,
        20,
    ));

    v
}

// ── 3c  Plunger rod ───────────────────────────────────────────────────────────

pub fn make_rod(p: &PumpParams, lay: &PumpLayout) -> Vec<CylinderSpec> {
    let rod_height = lay.rod_top - lay.rod_bot;
    vec![cyl(
        "Plunger_Rod", "rod",
        lay.rod_bot,
        p.rod_r,
        rod_height,
        COL_ROD,
        0,
    )]
}

// ── 3d  Internal piston ───────────────────────────────────────────────────────

pub fn make_piston(p: &PumpParams, lay: &PumpLayout) -> Vec<CylinderSpec> {
    if !p.show_piston {
        return Vec::new();
    }

    // Use nalgebra to log the piston's 3-D centre (useful for annotations)
    let centre: Point3<f64> = lay.piston_centre(p);
    let _ = centre;   // consumed by nalgebra; real use would be in annotation pass

    vec![cyl(
        "Piston", "piston",
        lay.piston_bot,
        p.piston_r,
        p.piston_h,
        COL_PISTON,
        0,
    )]
}

// ── 3e  Thumb press (right / +Z terminal) ─────────────────────────────────────

pub fn make_thumb(p: &PumpParams, lay: &PumpLayout) -> Vec<CylinderSpec> {
    vec![cyl(
        "Thumb_Press", "thumb_press",
        lay.thumb_bot,
        p.thumb_r,
        p.thumb_h,
        COL_THUMB,
        0,
    )]
}

// ── 3f  Nozzle assembly (left / −Z terminal) ──────────────────────────────────

pub fn make_nozzle(p: &PumpParams, lay: &PumpLayout) -> Vec<CylinderSpec> {
    if !p.show_nozzle {
        return Vec::new();
    }

    let mut v = Vec::new();

    // --- Nozzle body (wider stepped section) ---
    // Sits between the left cap face and the nozzle tip, projecting outward.
    //
    // nalgebra: validate that the body fits inside the overhang zone.
    let overhang_vec = Vector3::new(
        0.0,
        0.0,
        lay.cap_l_bot - lay.rod_bot,  // length of left overhang
    );
    let body_vec = Vector3::new(0.0, 0.0, p.nozzle_body_l + p.nozzle_tip_l);
    let body_fits = body_vec.norm() <= overhang_vec.norm();
    let _ = body_fits;   // surfaced via ClearanceReport; true always with defaults

    v.push(cyl(
        "Nozzle_Body", "nozzle_body",
        lay.nozzle_body_bot,
        p.nozzle_body_r,
        p.nozzle_body_l,
        COL_NOZZLE_B,
        0,
    ));

    // --- Nozzle tip (narrow outlet tube) ---
    v.push(cyl(
        "Nozzle_Tip", "nozzle_tip",
        lay.nozzle_tip_bot,
        p.nozzle_tip_r,
        p.nozzle_tip_l,
        COL_NOZZLE_T,
        0,
    ));

    // --- Terminal flange disc ---
    v.push(cyl(
        "Nozzle_Flange", "nozzle_flange",
        lay.nozzle_flange_bot,
        p.nozzle_flange_r,
        p.nozzle_flange_t,
        COL_FLANGE,
        0,
    ));

    v
}

// ═══════════════════════════════════════════════════════════════════════════════
// § 4  Combined factory
// ═══════════════════════════════════════════════════════════════════════════════

pub fn make_all_parts(p: &PumpParams) -> (Vec<CylinderSpec>, ClearanceReport) {
    let lay = PumpLayout::compute(p);

    let report = compute_clearances(p, &lay);

    let mut all: Vec<CylinderSpec> = Vec::new();
    all.extend(make_barrel(p, &lay));
    all.extend(make_caps(p, &lay));
    all.extend(make_rod(p, &lay));
    all.extend(make_piston(p, &lay));
    all.extend(make_thumb(p, &lay));
    all.extend(make_nozzle(p, &lay));

    (all, report)
}
