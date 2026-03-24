// /home/emporas/repos/freecad/rust/fc_house/src/lib.rs
// source /home/emporas/.venv/bin/activate
use pyo3::prelude::*;

// ── Parameters ────────────────────────────────────────────────────────────────

#[pyclass]
#[derive(Clone)]
pub struct HouseParams {
    #[pyo3(get, set)] pub length:          f64,
    #[pyo3(get, set)] pub width:           f64,
    #[pyo3(get, set)] pub wall_height:     f64,
    #[pyo3(get, set)] pub wall_thickness:  f64,
    #[pyo3(get, set)] pub base_height:     f64,
    #[pyo3(get, set)] pub slope_deg:       f64,
    #[pyo3(get, set)] pub roof_thickness:  f64,
    #[pyo3(get, set)] pub roof_overhang:   f64,
}

#[pymethods]
impl HouseParams {
    #[new]
    #[pyo3(signature = (
        length         = 10_000.0,
        width          = 10_000.0,
        wall_height    =  3_000.0,
        wall_thickness =    400.0,
        base_height    =    150.0,
        slope_deg      =     35.0,
        roof_thickness =    300.0,
        roof_overhang  =    300.0,
    ))]
    fn new(
        length: f64, width: f64, wall_height: f64, wall_thickness: f64,
        base_height: f64, slope_deg: f64, roof_thickness: f64, roof_overhang: f64,
    ) -> Self {
        Self { length, width, wall_height, wall_thickness,
               base_height, slope_deg, roof_thickness, roof_overhang }
    }
}

// ── BIM spec structs ──────────────────────────────────────────────────────────

/// Arch::Structure used as a foundation slab.
#[pyclass]
pub struct SlabSpec {
    #[pyo3(get)] pub label:     String,
    #[pyo3(get)] pub x:         f64,
    #[pyo3(get)] pub y:         f64,
    #[pyo3(get)] pub z:         f64,
    #[pyo3(get)] pub length:    f64,
    #[pyo3(get)] pub width:     f64,
    #[pyo3(get)] pub thickness: f64,
}

/// Arch::Wall driven by a Draft line (center-line convention).
#[pyclass]
pub struct WallSpec {
    #[pyo3(get)] pub label:  String,
    /// Center-line start/end in XY; wall rises from z for `height` mm.
    #[pyo3(get)] pub x1:     f64,
    #[pyo3(get)] pub y1:     f64,
    #[pyo3(get)] pub x2:     f64,
    #[pyo3(get)] pub y2:     f64,
    #[pyo3(get)] pub z:      f64,
    #[pyo3(get)] pub height: f64,
    #[pyo3(get)] pub width:  f64,   // wall thickness
    #[pyo3(get)] pub align:  String, // "Center" | "Left" | "Right"
}

/// Arch::Roof driven by a closed wire polygon + per-edge slopes.
/// Gable roof: slopes on the long sides, 90° on the gable ends.
#[pyclass]
pub struct RoofSpec {
    #[pyo3(get)] pub label:     String,
    /// Outline corners (x, y, z) at wall-top elevation, open polygon
    /// (Python closes it before building the Part wire).
    #[pyo3(get)] pub pts:       Vec<(f64, f64, f64)>,
    /// One slope (degrees from horizontal) per edge; len == pts.len().
    #[pyo3(get)] pub slopes:    Vec<f64>,
    #[pyo3(get)] pub thickness: f64,
    #[pyo3(get)] pub overhang:  f64,
}

// ── Geometry builders (pure Rust, no FreeCAD) ────────────────────────────────

fn make_slab(p: &HouseParams) -> SlabSpec {
    SlabSpec {
        label:     "Slab_Foundation".into(),
        x: 0.0, y: 0.0, z: 0.0,
        length:    p.length,
        width:     p.width,
        thickness: p.base_height,
    }
}

fn make_walls(p: &HouseParams) -> Vec<WallSpec> {
    let (l, w, wh, t, bh) = (
        p.length, p.width, p.wall_height, p.wall_thickness, p.base_height,
    );
    let ht = t / 2.0; // half-thickness offset puts endpoints on center-line

    // Each wall's center-line runs at z = base_height.
    // "Center" alignment means the wall body straddles the line by ±width/2.
    macro_rules! wall {
        ($label:expr, $x1:expr,$y1:expr, $x2:expr,$y2:expr) => {
            WallSpec {
                label:  $label.into(),
                x1: $x1, y1: $y1, x2: $x2, y2: $y2,
                z:      bh,
                height: wh,
                width:  t,
                align:  "Center".into(),
            }
        };
    }

    vec![
        wall!("Wall_South", 0.0,    ht,      l,      ht    ),
        wall!("Wall_North", 0.0,    w - ht,  l,      w - ht),
        wall!("Wall_West",  ht,     0.0,     ht,     w     ),
        wall!("Wall_East",  l - ht, 0.0,     l - ht, w     ),
    ]
}

fn make_roof(p: &HouseParams) -> RoofSpec {
    let z = p.base_height + p.wall_height;

    // Outline: S-W → S-E → N-E → N-W  (counterclockwise from above)
    // Ridge runs along X (the long axis) → gable ends on West & East edges.
    //   south edge slope = slope_deg
    //   east  edge       = 90°  (vertical gable end)
    //   north edge slope = slope_deg
    //   west  edge       = 90°  (vertical gable end)
    RoofSpec {
        label: "Roof".into(),
        pts: vec![
            (0.0,     0.0,     z),
            (p.length, 0.0,    z),
            (p.length, p.width, z),
            (0.0,      p.width, z),
        ],
        slopes:    vec![p.slope_deg, 90.0, p.slope_deg, 90.0],
        thickness: p.roof_thickness,
        overhang:  p.roof_overhang,
    }
}

// ── Public entry point ────────────────────────────────────────────────────────

#[pyfunction]
fn build_specs(py: Python<'_>, p: &HouseParams) -> PyResult<Vec<PyObject>> {
    let mut out: Vec<PyObject> = Vec::new();

    out.push(Py::new(py, make_slab(p))?.into());

    for w in make_walls(p) {
        out.push(Py::new(py, w)?.into());
    }

    out.push(Py::new(py, make_roof(p))?.into());

    Ok(out)
}

#[pymodule]
fn fc_house(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<HouseParams>()?;
    m.add_class::<SlabSpec>()?;
    m.add_class::<WallSpec>()?;
    m.add_class::<RoofSpec>()?;
    m.add_function(wrap_pyfunction!(build_specs, m)?)?;
    Ok(())
}
