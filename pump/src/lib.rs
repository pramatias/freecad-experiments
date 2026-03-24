// pump/src/lib.rs
//!
//! PyO3 module exposing the coaxial syringe pump geometry to Python / FreeCAD.
//!
//! **Hot-reload compatible**: the Python macro copies the compiled `.so` to a
//! timestamped shadow file before importing, so FreeCAD never locks the live
//! binary.  See `pump_macro/pump.py` § 0 for the reload dance.
//!
//! Public API mirrors the shard module for structural consistency:
//!
//! ```python
//! import pump as pm
//! specs = pm.build_all_specs()               # default params
//! specs = pm.build_all_specs(pm.PumpParams(stroke=80.0))
//!
//! specs.cylinders   # list[CylinderSpec]
//! specs.clearances  # ClearanceReport
//! specs.n_total     # int
//! ```

mod geometry;
mod types;

use pyo3::prelude::*;
use types::{ClearanceReport, CylinderSpec, PumpParams};
use geometry::make_all_parts;

// ─── AllSpecs ─────────────────────────────────────────────────────────────────

/// All geometry needed to build the pump assembly in FreeCAD.
///
/// | Field       | Contents                                          |
/// |-------------|---------------------------------------------------|
/// | cylinders   | Every `Part::Cylinder` primitive                  |
/// | clearances  | Fit/clearance diagnostics (nalgebra-computed)     |
/// | n_total     | Total primitive count                             |
#[pyclass]
pub struct AllSpecs {
    /// Flat list of every cylindrical primitive in assembly order.
    #[pyo3(get)] pub cylinders:  Vec<PyObject>,
    /// Clearance / fit report — check `.warnings` for geometry violations.
    #[pyo3(get)] pub clearances: PyObject,
    /// Total number of `Part::Cylinder` objects to be created in FreeCAD.
    #[pyo3(get)] pub n_total:    usize,
}

// ─── build_all_specs ─────────────────────────────────────────────────────────

#[pyfunction]
#[pyo3(signature = (params = None))]
fn build_all_specs(py: Python<'_>, params: Option<PumpParams>) -> PyResult<AllSpecs> {
    let p = params.unwrap_or_else(PumpParams::default_params);
    let (cylinders, report) = make_all_parts(&p);

    let n_total = cylinders.len();

    let cylinders: Vec<PyObject> = cylinders
        .into_iter()
        .map(|s| Ok(Py::new(py, s)?.into_any()))
        .collect::<PyResult<_>>()?;

    let clearances: PyObject = Py::new(py, report)?.into_any();

    Ok(AllSpecs { cylinders, clearances, n_total })
}

// ─── Module ───────────────────────────────────────────────────────────────────

#[pymodule]
fn pump(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PumpParams>()?;
    m.add_class::<CylinderSpec>()?;
    m.add_class::<ClearanceReport>()?;
    m.add_class::<AllSpecs>()?;
    m.add_function(wrap_pyfunction!(build_all_specs, m)?)?;
    Ok(())
}
