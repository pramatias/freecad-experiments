// /home/emporas/repos/freecad/rust/leroy/src/lib.rs
mod builder;
mod types;

use pyo3::prelude::*;

#[pymodule]
fn leroy(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<types::FaceExtrudeSpec>()?;
    m.add_class::<types::BoltHoleSpec>()?;
    m.add_class::<types::LineSpec>()?;
    m.add_class::<types::TextSpec>()?;
    m.add_class::<types::LogoSpecs>()?;
    m.add_class::<types::SupportBackSpec>()?;
    m.add_function(wrap_pyfunction!(builder::build_logo_specs, m)?)?;
    m.add_function(wrap_pyfunction!(builder::build_support_back_spec, m)?)?;
    Ok(())
}
