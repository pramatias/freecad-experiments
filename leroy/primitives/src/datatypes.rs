// /home/emporas/repos/freecad/rust/primitives/src/datatypes.rs
use nalgebra::{Point2, Point3};

// ── Colour ─────────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

impl Color {
    #[inline]
    pub fn new(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b }
    }

    pub fn to_vec(&self) -> Vec<f32> {
        vec![self.r, self.g, self.b]
    }
}

// ── Text anchor ────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub enum Anchor {
    StartPeak,
    EndPeak,
    StartBase,
    EndBase,
}

impl Anchor {
    pub fn as_str(&self) -> &'static str {
        match self {
            Anchor::StartPeak => "start_peak",
            Anchor::EndPeak => "end_peak",
            Anchor::StartBase => "start_base",
            Anchor::EndBase => "end_base",
        }
    }
}

// ── Geometry specs ─────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct FaceExtrudeSpec {
    pub vertices_2d: Vec<Point2<f64>>,
    pub z_base: f64,
    pub extrude_height: f64,
    pub color: Color,
    pub transparency: i32,
    pub label: String,
}

#[derive(Clone, Debug)]
pub struct LineSpec {
    pub p1: Point3<f64>,
    pub p2: Point3<f64>,
    pub color: Color,
    pub label: String,
}

#[derive(Clone, Debug)]
pub struct TextSpec {
    pub text: String,
    pub position: Point3<f64>,
    pub rotation_deg: f64,
    pub font_height: f64,
    pub extrude_depth: f64,
    pub color: Color,
    pub label: String,
    pub anchor: Anchor,
}

#[derive(Clone, Debug)]
pub struct BoltHoleSpec {
    pub center: Point3<f64>,
    pub radius: f64,
    pub depth: f64,
    pub label: String,
}

/// Back-side support geometry only.
/// This is separate from `LogoSpecs`, so the logo faces/text stay unchanged.
#[derive(Clone, Debug)]
pub struct SupportBackSpec {
    pub outer_vertices_2d: Vec<Point2<f64>>,
    pub inner_vertices_2d: Vec<Point2<f64>>,
    pub z_back: f64,
    pub body_depth: f64,
    pub pocket_depth: f64,
    pub corner_ribs: Vec<FaceExtrudeSpec>,
    pub center_square: FaceExtrudeSpec,
    pub bolt_hole: BoltHoleSpec,
    pub color: Color,
    pub transparency: i32,
    pub label: String,
}

/// Top-level container produced by the logo builder.
#[derive(Debug)]
pub struct LogoSpecs {
    pub faces: Vec<FaceExtrudeSpec>,
    pub lines: Vec<LineSpec>,
    pub texts: Vec<TextSpec>,
}
