// shard/src/lib.rs
//!
//! Exposes the shattered-crystal starburst geometry to Python.
//!
//! **Breaking change from v1**: `AllSpecs.frame_specs` now contains
//! `ShardSpec` objects (one per faceted ring segment) instead of a single
//! `EllipseFrameSpec`.  The Python macro must call `_make_shard()` for each
//! frame spec and dispatch on `part_type` ("frame_facet" | "frame_inner").
use pyo3::prelude::*;
use primitives::{
    ShardParams,
    EllipseFrameSpec, SpikeSpec, ShardSpec, FoundationSpec,
    make_all_parts,
};

// ─── AllSpecs ─────────────────────────────────────────────────────────────────

/// All parts needed to build the shattered crystal model in FreeCAD.
///
/// | Field            | Rust type           | part_type values                         |
/// |------------------|---------------------|------------------------------------------|
/// | frame_specs      | Vec<ShardSpec>      | "frame_facet", "frame_inner"             |
/// | spike_specs      | Vec<SpikeSpec>      | "diamond_up/down", "primary", "secondary"|
/// | shard_specs      | Vec<ShardSpec>      | "inner_blade", "inner_shard"             |
/// | foundation_specs | Vec<FoundationSpec> | "v_plate", "tier_bar", "debris"          |
#[pyclass]
pub struct AllSpecs {
    /// Faceted ring frame — Vec<ShardSpec> with part_type "frame_facet" / "frame_inner"
    #[pyo3(get)] pub frame_specs:      Vec<PyObject>,
    /// Diamond cones + all radial spikes — Vec<SpikeSpec>
    #[pyo3(get)] pub spike_specs:      Vec<PyObject>,
    /// Horizontal blades + inner shard facets — Vec<ShardSpec>
    #[pyo3(get)] pub shard_specs:      Vec<PyObject>,
    /// V-plate tiers + corner debris — Vec<FoundationSpec>
    #[pyo3(get)] pub foundation_specs: Vec<PyObject>,
    /// Total object count across all four lists.
    #[pyo3(get)] pub n_total:          usize,
}

// ─── build_all_specs ─────────────────────────────────────────────────────────

#[pyfunction]
#[pyo3(signature = (params = None))]
fn build_all_specs(py: Python<'_>, params: Option<ShardParams>) -> PyResult<AllSpecs> {
    let p = params.unwrap_or_else(ShardParams::default_params);

    let (frame, spikes, shards, foundation) = make_all_parts(&p);

    let n_total = frame.len() + spikes.len() + shards.len() + foundation.len();

    // frame is now Vec<ShardSpec> — same serialisation path as shard_specs
    let frame_specs: Vec<PyObject> = frame
        .into_iter()
        .map(|s| Ok(Py::new(py, s)?.into_any()))
        .collect::<PyResult<_>>()?;

    let spike_specs: Vec<PyObject> = spikes
        .into_iter()
        .map(|s| Ok(Py::new(py, s)?.into_any()))
        .collect::<PyResult<_>>()?;

    let shard_specs: Vec<PyObject> = shards
        .into_iter()
        .map(|s| Ok(Py::new(py, s)?.into_any()))
        .collect::<PyResult<_>>()?;

    let foundation_specs: Vec<PyObject> = foundation
        .into_iter()
        .map(|s| Ok(Py::new(py, s)?.into_any()))
        .collect::<PyResult<_>>()?;

    Ok(AllSpecs {
        frame_specs,
        spike_specs,
        shard_specs,
        foundation_specs,
        n_total,
    })
}

// ─── Module ───────────────────────────────────────────────────────────────────

#[pymodule]
fn shard(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<ShardParams>()?;
    m.add_class::<EllipseFrameSpec>()?;   // kept for optional direct use
    m.add_class::<SpikeSpec>()?;
    m.add_class::<ShardSpec>()?;
    m.add_class::<FoundationSpec>()?;
    m.add_class::<AllSpecs>()?;
    m.add_function(wrap_pyfunction!(build_all_specs, m)?)?;
    Ok(())
}
