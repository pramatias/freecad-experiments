// primitives/src/types.rs
use pyo3::prelude::*;

// ═══════════════════════════════════════════════════════════════════════════════
// § 1  Build parameters
// ═══════════════════════════════════════════════════════════════════════════════

/// Global build parameters for the shattered-crystal starburst composition.
///
/// Coordinate system (FreeCAD default):
///   X+  = right
///   Y+  = into the screen (depth / thickness)
///   Z+  = upward
///
/// Default changes vs. v1
/// ──────────────────────
///   inner_shards   20 → 40  (denser facet fill inside the Eye)
///   debris_count   10 → 18  (larger crystalline debris formation)
#[pyclass]
#[derive(Clone)]
pub struct ShardParams {
    // ── Frame ────────────────────────────────────────────────────────────────
    #[pyo3(get, set)] pub frame_rx:        f64,  // = 800
    #[pyo3(get, set)] pub frame_rz:        f64,  // = 520
    #[pyo3(get, set)] pub frame_thickness: f64,  // = 80
    #[pyo3(get, set)] pub frame_depth:     f64,  // = 55

    // ── Inner core ────────────────────────────────────────────────────────────
    #[pyo3(get, set)] pub diamond_h_up:    f64,  // = 360
    #[pyo3(get, set)] pub diamond_h_dn:    f64,  // = 220
    #[pyo3(get, set)] pub diamond_base_r:  f64,  // = 120
    #[pyo3(get, set)] pub blade_pairs:     u32,  // = 4  (was 3)
    #[pyo3(get, set)] pub inner_shards:    u32,  // = 40 (was 20)

    // ── Spikes ────────────────────────────────────────────────────────────────
    #[pyo3(get, set)] pub primary_len:    f64,  // = 1400
    #[pyo3(get, set)] pub primary_base:   f64,  // = 45
    #[pyo3(get, set)] pub secondary_len:  f64,  // = 700
    #[pyo3(get, set)] pub secondary_base: f64,  // = 28

    // ── Foundation ────────────────────────────────────────────────────────────
    #[pyo3(get, set)] pub foundation_tiers: u32,  // = 4
    #[pyo3(get, set)] pub debris_count:     u32,  // = 18 (was 10)

    // ── Feature toggles ───────────────────────────────────────────────────────
    #[pyo3(get, set)] pub compute_core:       bool,
    #[pyo3(get, set)] pub compute_spikes:     bool,
    #[pyo3(get, set)] pub compute_foundation: bool,
}

impl ShardParams {
    pub fn default_params() -> Self {
        Self::new(
            800.0,  // frame_rx
            520.0,  // frame_rz
             80.0,  // frame_thickness
             55.0,  // frame_depth
            360.0,  // diamond_h_up
            220.0,  // diamond_h_dn
            120.0,  // diamond_base_r
              4,    // blade_pairs  (↑ from 3)
             40,    // inner_shards (↑ from 20)
           1400.0,  // primary_len
             45.0,  // primary_base
            700.0,  // secondary_len
             28.0,  // secondary_base
              4,    // foundation_tiers
             18,    // debris_count (↑ from 10)
            true,   // compute_core
            true,   // compute_spikes
            true,   // compute_foundation
        )
    }
}

#[pymethods]
impl ShardParams {
    #[new]
    #[pyo3(signature = (
        frame_rx          = 800.0,
        frame_rz          = 520.0,
        frame_thickness   =  80.0,
        frame_depth       =  55.0,
        diamond_h_up      = 360.0,
        diamond_h_dn      = 220.0,
        diamond_base_r    = 120.0,
        blade_pairs       =     4,
        inner_shards      =    40,
        primary_len       = 1400.0,
        primary_base      =   45.0,
        secondary_len     =  700.0,
        secondary_base    =   28.0,
        foundation_tiers  =     4,
        debris_count      =    18,
        compute_core      =  true,
        compute_spikes    =  true,
        compute_foundation = true,
    ))]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        frame_rx: f64, frame_rz: f64, frame_thickness: f64, frame_depth: f64,
        diamond_h_up: f64, diamond_h_dn: f64, diamond_base_r: f64,
        blade_pairs: u32, inner_shards: u32,
        primary_len: f64, primary_base: f64,
        secondary_len: f64, secondary_base: f64,
        foundation_tiers: u32, debris_count: u32,
        compute_core: bool, compute_spikes: bool, compute_foundation: bool,
    ) -> Self {
        Self {
            frame_rx, frame_rz, frame_thickness, frame_depth,
            diamond_h_up, diamond_h_dn, diamond_base_r,
            blade_pairs, inner_shards,
            primary_len, primary_base,
            secondary_len, secondary_base,
            foundation_tiers, debris_count,
            compute_core, compute_spikes, compute_foundation,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// § 2  Spec types  (Rust → FreeCAD)
// ═══════════════════════════════════════════════════════════════════════════════

/// The central elliptical ring frame (kept for optional direct use).
///
/// NOTE: The main pipeline no longer uses this type.  `make_frame()` now
/// returns `Vec<ShardSpec>` so the ring is built from faceted boxes.
/// This struct is exported so callers can still construct a smooth ring if
/// needed for comparison or debugging.
#[pyclass]
pub struct EllipseFrameSpec {
    #[pyo3(get)] pub label:     String,
    #[pyo3(get)] pub cx:        f64,
    #[pyo3(get)] pub cy:        f64,
    #[pyo3(get)] pub cz:        f64,
    #[pyo3(get)] pub rx:        f64,
    #[pyo3(get)] pub rz:        f64,
    #[pyo3(get)] pub thickness: f64,
    #[pyo3(get)] pub depth:     f64,
    #[pyo3(get)] pub color:     (f32, f32, f32),
}

/// A generic angular shard — always rendered as a rotated `Part::Box`.
///
/// Used for:
///   • "frame_facet"  — one tangential facet of the outer ring
///   • "frame_inner"  — one facet of the inner accent ring
///   • "inner_blade"  — wide horizontal cutting blade
///   • "inner_shard"  — small random inner facet
#[pyclass]
pub struct ShardSpec {
    #[pyo3(get)] pub label:     String,
    #[pyo3(get)] pub part_type: String,
    #[pyo3(get)] pub x:         f64,
    #[pyo3(get)] pub y:         f64,
    #[pyo3(get)] pub z:         f64,
    #[pyo3(get)] pub length:    f64,
    #[pyo3(get)] pub width:     f64,
    #[pyo3(get)] pub height:    f64,
    #[pyo3(get)] pub rot_x:     f64,
    #[pyo3(get)] pub rot_y:     f64,
    #[pyo3(get)] pub rot_z:     f64,
    #[pyo3(get)] pub color:     (f32, f32, f32),
}

/// An elongated pyramidal spike — rendered as `Part::Cone` (tip_radius ≈ 0).
///
/// Polyhedral appearance is achieved by generating multiple SpikeSpec objects
/// per logical spike (one main + flanking sub-cones), each with slightly
/// different rot_y and rot_x.  The Python macro creates one Cone per spec.
///
/// | part_type      | Use                                               |
/// |----------------|---------------------------------------------------|
/// | "diamond_up"   | Upper lobe of the core diamond (+Z)               |
/// | "diamond_down" | Lower lobe of the core diamond (−Z)               |
/// | "primary"      | Large corner spike (±45°) + its flanking cones    |
/// | "secondary"    | Medium axis spike + flanking cones                |
/// | "minor"        | Small fill spike between primary/secondary        |
#[pyclass]
pub struct SpikeSpec {
    #[pyo3(get)] pub label:       String,
    #[pyo3(get)] pub part_type:   String,
    #[pyo3(get)] pub x:           f64,
    #[pyo3(get)] pub y:           f64,
    #[pyo3(get)] pub z:           f64,
    #[pyo3(get)] pub length:      f64,
    #[pyo3(get)] pub base_radius: f64,
    #[pyo3(get)] pub tip_radius:  f64,
    #[pyo3(get)] pub rot_y:       f64,
    #[pyo3(get)] pub rot_x:       f64,
    #[pyo3(get)] pub color:       (f32, f32, f32),
}

/// Foundation element — V-plates or scattered debris.
///
/// | part_type  | Description                                      |
/// |------------|--------------------------------------------------|
/// | "v_plate"  | One arm of a downward V, angled via rot_z        |
/// | "tier_bar" | Thin horizontal bar capping a tier               |
/// | "debris"   | Large crystalline block, fully 3-D rotated       |
#[pyclass]
pub struct FoundationSpec {
    #[pyo3(get)] pub label:     String,
    #[pyo3(get)] pub part_type: String,
    #[pyo3(get)] pub x:         f64,
    #[pyo3(get)] pub y:         f64,
    #[pyo3(get)] pub z:         f64,
    #[pyo3(get)] pub length:    f64,
    #[pyo3(get)] pub width:     f64,
    #[pyo3(get)] pub height:    f64,
    #[pyo3(get)] pub rot_x:     f64,
    #[pyo3(get)] pub rot_y:     f64,
    #[pyo3(get)] pub rot_z:     f64,
    #[pyo3(get)] pub color:     (f32, f32, f32),
}
