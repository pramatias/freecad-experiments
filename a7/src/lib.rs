// /home/emporas/repos/freecad/rust/a7/src/lib.rs
// /home/emporas/repos/freecad/rust/a7/src/lib.rs
//!
//! Exposes the Austin Seven part specs to Python in one shot.
//!
//! There is no database and no incremental diff.  Every call to
//! `build_all_specs()` recomputes the full set of parts from `A7Params`
//! and returns them as four typed lists ready for the FreeCAD macro to
//! consume.
use pyo3::prelude::*;
use primitives::{
    A7Params, BodyPartSpec, ChassisPartSpec, MechanicalPartSpec, WheelSpec,
    make_all_parts,
};

// ─── AllSpecs ─────────────────────────────────────────────────────────────────
/// All parts needed to build the A7 model in FreeCAD.
///
/// Each list contains typed Python objects that the macro dispatches on.
#[pyclass]
pub struct AllSpecs {
    /// `Vec<ChassisPartSpec>` — axle beams, rails, cross-members
    #[pyo3(get)] pub chassis_parts:    Vec<PyObject>,
    /// `Vec<BodyPartSpec>`   — radiator shell, cowl hull, seat-tub hull, floor pan
    #[pyo3(get)] pub body_parts:       Vec<PyObject>,
    /// `Vec<WheelSpec>`      — one entry per wheel position (FL/FR/RL/RR)
    #[pyo3(get)] pub wheels:           Vec<PyObject>,
    /// `Vec<MechanicalPartSpec>` — engine, steering, seat
    #[pyo3(get)] pub mechanical_parts: Vec<PyObject>,
    /// Total object count across all four lists.
    #[pyo3(get)] pub n_total:          usize,
}

// ─── build_all_specs ─────────────────────────────────────────────────────────
/// Compute all part specs and return them as an `AllSpecs` bundle.
///
/// # Arguments
/// * `params` — optional `A7Params`; uses the built-in defaults when omitted.
///
/// # Defaults (period-correct Austin Seven geometry)
/// | Parameter      | Value   | Note                                  |
/// |----------------|---------|---------------------------------------|
/// | wheelbase      | 1905 mm |                                       |
/// | track_front    | 1016 mm |                                       |
/// | wheel_radius   |  350 mm | outer radius → Ø 700 mm tall wheel    |
/// | tire_section   |  105 mm | narrow cross-section                  |
/// | rim_width      |   95 mm | narrow rim                            |
/// | spoke_count    |      24 | wire-spoke count per wheel            |
///
/// # Returns
/// An `AllSpecs` instance.  The macro iterates each list and creates the
/// matching FreeCAD primitives.
#[pyfunction]
#[pyo3(signature = (params = None))]
fn build_all_specs(py: Python<'_>, params: Option<A7Params>) -> PyResult<AllSpecs> {
let p = params.unwrap_or_else(|| A7Params::new(
    1905.0,  // wheelbase
    1016.0,  // track_front
    1016.0,  // track_rear
     355.0,  // wheel_radius  → outer Ø 710 mm
     100.0,  // tire_section  → narrow vintage section
      95.0,  // rim_width
      60.0,  // hub_radius    → hub Ø 120 mm (was 35 — far too small)
        28,  // spoke_count   → 28 wire spokes (was 24)
     120.0,  // chassis_z
     280.0,  // body_floor_z
      true,  // compute_engine
));

    let (chassis, body, wheels, mech) = make_all_parts(&p);
    let n_total = chassis.len() + body.len() + wheels.len() + mech.len();

    let chassis_parts: Vec<PyObject> = chassis
        .into_iter()
        .map(|s| Py::new(py, s).map(Into::into))
        .collect::<PyResult<_>>()?;
    let body_parts: Vec<PyObject> = body
        .into_iter()
        .map(|s| Py::new(py, s).map(Into::into))
        .collect::<PyResult<_>>()?;
    let wheels: Vec<PyObject> = wheels
        .into_iter()
        .map(|s| Py::new(py, s).map(Into::into))
        .collect::<PyResult<_>>()?;
    let mechanical_parts: Vec<PyObject> = mech
        .into_iter()
        .map(|s| Py::new(py, s).map(Into::into))
        .collect::<PyResult<_>>()?;

    Ok(AllSpecs { chassis_parts, body_parts, wheels, mechanical_parts, n_total })
}

// ─── Module ───────────────────────────────────────────────────────────────────
#[pymodule]
fn a7(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<A7Params>()?;
    m.add_class::<ChassisPartSpec>()?;
    m.add_class::<BodyPartSpec>()?;
    m.add_class::<WheelSpec>()?;
    m.add_class::<MechanicalPartSpec>()?;
    m.add_class::<AllSpecs>()?;
    m.add_function(wrap_pyfunction!(build_all_specs, m)?)?;
    Ok(())
}
