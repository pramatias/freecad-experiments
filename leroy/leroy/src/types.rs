// /home/emporas/repos/freecad/rust/leroy/src/types.rs
//! PyO3 mirror types — flat, Python-friendly versions of the primitives
//! structs.  Conversion is a one-way `From` impl (Rust → Python).

use primitives::datatypes as prim;
use pyo3::prelude::*;

// ── FaceExtrudeSpec ────────────────────────────────────────────────────────

#[pyclass]
#[derive(Clone)]
pub struct FaceExtrudeSpec {
    #[pyo3(get)]
    pub vertices_2d: Vec<Vec<f64>>,
    #[pyo3(get)]
    pub z_base: f64,
    #[pyo3(get)]
    pub extrude_height: f64,
    #[pyo3(get)]
    pub color: Vec<f32>,
    #[pyo3(get)]
    pub transparency: i32,
    #[pyo3(get)]
    pub label: String,
}

impl From<prim::FaceExtrudeSpec> for FaceExtrudeSpec {
    fn from(s: prim::FaceExtrudeSpec) -> Self {
        Self {
            vertices_2d: s.vertices_2d.iter().map(|p| vec![p.x, p.y]).collect(),
            z_base: s.z_base,
            extrude_height: s.extrude_height,
            color: s.color.to_vec(),
            transparency: s.transparency,
            label: s.label,
        }
    }
}

// ── BoltHoleSpec ───────────────────────────────────────────────────────────

#[pyclass]
#[derive(Clone)]
pub struct BoltHoleSpec {
    #[pyo3(get)]
    pub x: f64,
    #[pyo3(get)]
    pub y: f64,
    #[pyo3(get)]
    pub z: f64,
    #[pyo3(get)]
    pub radius: f64,
    #[pyo3(get)]
    pub depth: f64,
    #[pyo3(get)]
    pub label: String,
}

impl From<prim::BoltHoleSpec> for BoltHoleSpec {
    fn from(s: prim::BoltHoleSpec) -> Self {
        Self {
            x: s.center.x,
            y: s.center.y,
            z: s.center.z,
            radius: s.radius,
            depth: s.depth,
            label: s.label,
        }
    }
}

// ── SupportBackSpec ────────────────────────────────────────────────────────

#[pyclass]
#[derive(Clone)]
pub struct SupportBackSpec {
    #[pyo3(get)]
    pub outer_vertices_2d: Vec<Vec<f64>>,
    #[pyo3(get)]
    pub inner_vertices_2d: Vec<Vec<f64>>,
    #[pyo3(get)]
    pub z_back: f64,
    #[pyo3(get)]
    pub body_depth: f64,
    #[pyo3(get)]
    pub pocket_depth: f64,
    #[pyo3(get)]
    pub corner_ribs: Vec<FaceExtrudeSpec>,
    #[pyo3(get)]
    pub center_square: FaceExtrudeSpec,
    #[pyo3(get)]
    pub bolt_hole: BoltHoleSpec,
    #[pyo3(get)]
    pub color: Vec<f32>,
    #[pyo3(get)]
    pub transparency: i32,
    #[pyo3(get)]
    pub label: String,
}

impl From<prim::SupportBackSpec> for SupportBackSpec {
    fn from(s: prim::SupportBackSpec) -> Self {
        Self {
            outer_vertices_2d: s.outer_vertices_2d.iter().map(|p| vec![p.x, p.y]).collect(),
            inner_vertices_2d: s.inner_vertices_2d.iter().map(|p| vec![p.x, p.y]).collect(),
            z_back: s.z_back,
            body_depth: s.body_depth,
            pocket_depth: s.pocket_depth,
            corner_ribs: s.corner_ribs.into_iter().map(Into::into).collect(),
            center_square: s.center_square.into(),
            bolt_hole: s.bolt_hole.into(),
            color: s.color.to_vec(),
            transparency: s.transparency,
            label: s.label,
        }
    }
}

// ── LineSpec ───────────────────────────────────────────────────────────────

#[pyclass]
#[derive(Clone)]
pub struct LineSpec {
    #[pyo3(get)]
    pub x1: f64,
    #[pyo3(get)]
    pub y1: f64,
    #[pyo3(get)]
    pub z1: f64,
    #[pyo3(get)]
    pub x2: f64,
    #[pyo3(get)]
    pub y2: f64,
    #[pyo3(get)]
    pub z2: f64,
    #[pyo3(get)]
    pub color: Vec<f32>,
    #[pyo3(get)]
    pub label: String,
}

impl From<prim::LineSpec> for LineSpec {
    fn from(s: prim::LineSpec) -> Self {
        Self {
            x1: s.p1.x,
            y1: s.p1.y,
            z1: s.p1.z,
            x2: s.p2.x,
            y2: s.p2.y,
            z2: s.p2.z,
            color: s.color.to_vec(),
            label: s.label,
        }
    }
}

// ── TextSpec ───────────────────────────────────────────────────────────────

#[pyclass]
#[derive(Clone)]
pub struct TextSpec {
    #[pyo3(get)]
    pub text: String,
    #[pyo3(get)]
    pub x: f64,
    #[pyo3(get)]
    pub y: f64,
    #[pyo3(get)]
    pub z: f64,
    #[pyo3(get)]
    pub rotation_deg: f64,
    #[pyo3(get)]
    pub font_height: f64,
    #[pyo3(get)]
    pub extrude_depth: f64,
    #[pyo3(get)]
    pub color: Vec<f32>,
    #[pyo3(get)]
    pub label: String,
    #[pyo3(get)]
    pub anchor: String,
}

impl From<prim::TextSpec> for TextSpec {
    fn from(s: prim::TextSpec) -> Self {
        Self {
            text: s.text,
            x: s.position.x,
            y: s.position.y,
            z: s.position.z,
            rotation_deg: s.rotation_deg,
            font_height: s.font_height,
            extrude_depth: s.extrude_depth,
            color: s.color.to_vec(),
            label: s.label,
            anchor: s.anchor.as_str().to_owned(),
        }
    }
}

// ── LogoSpecs ──────────────────────────────────────────────────────────────

#[pyclass]
pub struct LogoSpecs {
    #[pyo3(get)]
    pub faces: Vec<FaceExtrudeSpec>,
    #[pyo3(get)]
    pub lines: Vec<LineSpec>,
    #[pyo3(get)]
    pub texts: Vec<TextSpec>,
}

impl From<prim::LogoSpecs> for LogoSpecs {
    fn from(s: prim::LogoSpecs) -> Self {
        Self {
            faces: s.faces.into_iter().map(Into::into).collect(),
            lines: s.lines.into_iter().map(Into::into).collect(),
            texts: s.texts.into_iter().map(Into::into).collect(),
        }
    }
}
