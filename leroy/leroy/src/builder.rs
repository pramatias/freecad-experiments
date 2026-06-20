// /home/emporas/repos/freecad/rust/leroy/src/builder.rs
//! Exposes `build_logo_specs` as a PyO3 function.

use crate::types::LogoSpecs;
use crate::types::SupportBackSpec;
use pyo3::prelude::*;

#[pyfunction(signature = (leroy_down_offset = 0.0, merlin_down_offset = 0.0))]
pub fn build_logo_specs(leroy_down_offset: f64, merlin_down_offset: f64) -> PyResult<LogoSpecs> {
    Ok(logo::build_logo_specs(leroy_down_offset, merlin_down_offset).into())
}

#[pyfunction]
pub fn build_support_back_spec() -> PyResult<SupportBackSpec> {
    Ok(logo::build_support_back_spec().into())
}
