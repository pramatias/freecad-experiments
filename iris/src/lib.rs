//home/emporas/repos/freecad/rust/iris/src/lib.rs
use glam::DVec2;
use pyo3::prelude::*;
use std::f64::consts::TAU;

// ── Parameters ────────────────────────────────────────────────────────────────

#[pyclass]
#[derive(Clone)]
pub struct IrisParams {
    // Pupil
    #[pyo3(get, set)] pub pupil_radius:     f64,
    // Inner recessed ring + fin teeth
    #[pyo3(get, set)] pub ring_inner_r:     f64,
    #[pyo3(get, set)] pub ring_outer_r:     f64,
    #[pyo3(get, set)] pub ring_depth:       f64,   // how far below z=0 the ring sits
    #[pyo3(get, set)] pub fin_count:        u32,
    #[pyo3(get, set)] pub fin_width:        f64,   // tangential
    #[pyo3(get, set)] pub fin_height:       f64,   // protrudes upward
    // Mid-iris curved spokes
    #[pyo3(get, set)] pub spoke_count:      u32,
    #[pyo3(get, set)] pub spoke_inner_r:    f64,
    #[pyo3(get, set)] pub spoke_outer_r:    f64,
    #[pyo3(get, set)] pub spoke_arc_deg:    f64,   // angular width at inner edge
    #[pyo3(get, set)] pub spoke_taper:      f64,   // 0=parallel  1=tapers to a point
    #[pyo3(get, set)] pub spoke_height:     f64,
    // Outer flame / tentacle spines
    #[pyo3(get, set)] pub spine_count:      u32,
    #[pyo3(get, set)] pub spine_base_r:     f64,
    #[pyo3(get, set)] pub spine_length:     f64,
    #[pyo3(get, set)] pub spine_base_width: f64,
    #[pyo3(get, set)] pub spine_curve_deg:  f64,   // total clockwise sweep
    #[pyo3(get, set)] pub spine_height:     f64,
}

#[pymethods]
impl IrisParams {
    #[new]
    #[pyo3(signature = (
        pupil_radius     =   500.0,
        ring_inner_r     =   600.0,
        ring_outer_r     =  1400.0,
        ring_depth       =    80.0,
        fin_count        =    48,
        fin_width        =    30.0,
        fin_height       =   120.0,
        spoke_count      =    12,
        spoke_inner_r    =  1500.0,
        spoke_outer_r    =  3500.0,
        spoke_arc_deg    =    18.0,
        spoke_taper      =     0.6,
        spoke_height     =   200.0,
        spine_count      =    12,
        spine_base_r     =  3500.0,
        spine_length     =  1800.0,
        spine_base_width =   400.0,
        spine_curve_deg  =    40.0,
        spine_height     =   160.0,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        pupil_radius: f64,
        ring_inner_r: f64, ring_outer_r: f64, ring_depth: f64,
        fin_count: u32, fin_width: f64, fin_height: f64,
        spoke_count: u32, spoke_inner_r: f64, spoke_outer_r: f64,
        spoke_arc_deg: f64, spoke_taper: f64, spoke_height: f64,
        spine_count: u32, spine_base_r: f64, spine_length: f64,
        spine_base_width: f64, spine_curve_deg: f64, spine_height: f64,
    ) -> Self {
        Self {
            pupil_radius,
            ring_inner_r, ring_outer_r, ring_depth,
            fin_count, fin_width, fin_height,
            spoke_count, spoke_inner_r, spoke_outer_r,
            spoke_arc_deg, spoke_taper, spoke_height,
            spine_count, spine_base_r, spine_length,
            spine_base_width, spine_curve_deg, spine_height,
        }
    }
}

// ── Spec structs (read by Python) ─────────────────────────────────────────────

#[pyclass] pub struct SphereSpec {
    #[pyo3(get)] pub label:  String,
    #[pyo3(get)] pub radius: f64,
}

#[pyclass] pub struct TorusSpec {
    #[pyo3(get)] pub label:   String,
    #[pyo3(get)] pub radius1: f64,   // major (axis → tube centre)
    #[pyo3(get)] pub radius2: f64,   // minor (tube cross-section)
    #[pyo3(get)] pub z:       f64,
}

/// One fin: a box whose local X axis points radially, centered on its edge.
#[pyclass] pub struct FinSpec {
    #[pyo3(get)] pub label:     String,
    #[pyo3(get)] pub cx:        f64,
    #[pyo3(get)] pub cy:        f64,
    #[pyo3(get)] pub z_base:    f64,
    #[pyo3(get)] pub length:    f64,   // radial extent
    #[pyo3(get)] pub width:     f64,   // tangential thickness
    #[pyo3(get)] pub height:    f64,
    #[pyo3(get)] pub angle_deg: f64,   // rotation about Z
}

/// Flat polygon (open list, Python closes it) + extrusion height.
#[pyclass] pub struct PolygonExtrudeSpec {
    #[pyo3(get)] pub label:  String,
    #[pyo3(get)] pub pts:    Vec<(f64, f64, f64)>,
    #[pyo3(get)] pub height: f64,
    #[pyo3(get)] pub z_base: f64,
}

fn make_sphere(p: &IrisParams) -> SphereSpec {
    SphereSpec { label: "Pupil".into(), radius: p.pupil_radius }
}

fn make_torus(p: &IrisParams) -> TorusSpec {
    let mid_r  = (p.ring_inner_r + p.ring_outer_r) * 0.5;
    let tube_r = (p.ring_outer_r - p.ring_inner_r) * 0.5;
    TorusSpec { label: "InnerRing".into(), radius1: mid_r, radius2: tube_r, z: -p.ring_depth }
}

fn make_fins(p: &IrisParams) -> Vec<FinSpec> {
    let mid_r  = (p.ring_inner_r + p.ring_outer_r) * 0.5;
    let radial = p.ring_outer_r - p.ring_inner_r;
    let z_base = -p.ring_depth;

    (0..p.fin_count).map(|i| {
        let angle  = TAU * i as f64 / p.fin_count as f64;
        let center = DVec2::new(angle.cos(), angle.sin()) * mid_r;
        FinSpec {
            label:     format!("Fin_{i:03}"),
            cx: center.x, cy: center.y, z_base,
            length:    radial,
            width:     p.fin_width,
            height:    p.fin_height,
            angle_deg: angle.to_degrees(),
        }
    }).collect()
}

/// Fan-shaped spoke polygon in XY; inner arc CCW then outer arc CW.
fn spoke_polygon(p: &IrisParams, center_angle: f64) -> Vec<(f64, f64, f64)> {
    const N: usize = 12;
    let inner_half = p.spoke_arc_deg.to_radians() * 0.5;
    let outer_half = inner_half * (1.0 - p.spoke_taper);

    let sample_arc = |r: f64, half: f64, rev: bool| -> Vec<DVec2> {
        let mut pts: Vec<DVec2> = (0..=N).map(|i| {
            let t = i as f64 / N as f64;
            let a = center_angle - half + t * 2.0 * half;
            DVec2::new(a.cos(), a.sin()) * r
        }).collect();
        if rev { pts.reverse(); }
        pts
    };

    sample_arc(p.spoke_inner_r, inner_half, false)
        .into_iter()
        .chain(sample_arc(p.spoke_outer_r, outer_half, true))
        .map(|v| (v.x, v.y, 0.0))
        .collect()
}

fn make_spokes(p: &IrisParams) -> Vec<PolygonExtrudeSpec> {
    (0..p.spoke_count).map(|i| {
        let angle = TAU * i as f64 / p.spoke_count as f64;
        PolygonExtrudeSpec {
            label:  format!("Spoke_{i:03}"),
            pts:    spoke_polygon(p, angle),
            height: p.spoke_height,
            z_base: 0.0,
        }
    }).collect()
}

/// Curved spine: centerline follows an outward spiral curving clockwise;
/// width offset tapers from base_width → 0 at the tip.
fn spine_polygon(p: &IrisParams, base_angle: f64) -> Vec<(f64, f64, f64)> {
    const N: usize = 24;
    let curve = p.spine_curve_deg.to_radians();

    let mut left:  Vec<DVec2> = Vec::with_capacity(N + 1);
    let mut right: Vec<DVec2> = Vec::with_capacity(N + 1);

    for i in 0..=N {
        let t  = i as f64 / N as f64;
        let r  = p.spine_base_r + t * p.spine_length;
        let a  = base_angle - t * curve;             // clockwise curve
        let center = DVec2::new(a.cos(), a.sin()) * r;

        // Finite-difference tangent (clamped at tip)
        let t2 = (t + 1.0 / N as f64).min(1.0);
        let r2 = p.spine_base_r + t2 * p.spine_length;
        let a2 = base_angle - t2 * curve;
        let next = DVec2::new(a2.cos(), a2.sin()) * r2;

        let tangent = if (next - center).length_squared() > 1e-10 {
            (next - center).normalize()
        } else {
            DVec2::new(a.cos(), a.sin())
        };
        // Left normal (spine's "leading" edge)
        let normal = DVec2::new(-tangent.y, tangent.x);

        let half_w = p.spine_base_width * (1.0 - t) * 0.5;
        left.push(center + normal * half_w);
        right.push(center - normal * half_w);
    }

    // Closed polygon: left edge forward → right edge reversed
    right.reverse();
    left.iter().chain(right.iter())
        .map(|v| (v.x, v.y, 0.0))
        .collect()
}

fn make_spines(p: &IrisParams) -> Vec<PolygonExtrudeSpec> {
    (0..p.spine_count).map(|i| {
        let angle = TAU * i as f64 / p.spine_count as f64;
        PolygonExtrudeSpec {
            label:  format!("Spine_{i:03}"),
            pts:    spine_polygon(p, angle),
            height: p.spine_height,
            z_base: 0.0,
        }
    }).collect()
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[pyfunction]
fn build_specs(py: Python<'_>, p: &IrisParams) -> PyResult<Vec<PyObject>> {
    let mut out: Vec<PyObject> = Vec::new();

    out.push(Py::new(py, make_sphere(p))?.into());
    out.push(Py::new(py, make_torus(p))?.into());
    for f in make_fins(p)   { out.push(Py::new(py, f)?.into()); }
    for s in make_spokes(p) { out.push(Py::new(py, s)?.into()); }
    for s in make_spines(p) { out.push(Py::new(py, s)?.into()); }

    Ok(out)
}

#[pymodule]
fn iris(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<IrisParams>()?;
    m.add_class::<SphereSpec>()?;
    m.add_class::<TorusSpec>()?;
    m.add_class::<FinSpec>()?;
    m.add_class::<PolygonExtrudeSpec>()?;
    m.add_function(wrap_pyfunction!(build_specs, m)?)?;
    Ok(())
}
