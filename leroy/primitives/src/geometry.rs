// /home/emporas/repos/freecad/rust/primitives/src/geometry.rs
//! Constants, colour palettes, triangle geometry, ray–triangle clipper,
//! and grid-line computation.  All coordinate arithmetic uses nalgebra.

use nalgebra::{Point2, Point3, Vector2};
use std::f64::consts::SQRT_2;

use crate::datatypes::{Color, LineSpec};

// ── Constants ──────────────────────────────────────────────────────────────

pub const HALF_BASE: f64 = 25.0;
pub const APEX_Y: f64 = 25.0;
pub const LOGO_DEPTH: f64 = 3.0;
pub const SUPPORT_DEPTH: f64 = 3.0;
pub const TEXT_DEPTH: f64 = 0.8;
/// White border on the two slanted sides (enough to clear 6.5 mm text + 2 mm margin).
pub const BORDER_OFFSET_SIDES: f64 = 8.5;
/// White border on the base edge (no letters → 2 mm is enough).
pub const BORDER_OFFSET_BASE: f64 = 2.0;

// ── Colour palette ─────────────────────────────────────────────────────────

pub fn white() -> Color {
    Color::new(1.000, 1.000, 1.000)
}
pub fn leroy_green() -> Color {
    Color::new(0.027, 0.573, 0.188)
} // #079130
pub fn grid_grey() -> Color {
    Color::new(0.600, 0.600, 0.600)
}
pub fn black() -> Color {
    Color::new(0.000, 0.000, 0.000)
}

// ── Triangle vertices ──────────────────────────────────────────────────────

/// Inner green triangle  A = (−25, 0)   B = (25, 0)   C = (0, 25).
/// 45 ° base angles, 90 ° apex, base = 50 mm, height = 25 mm.
pub fn inner_tri_verts() -> Vec<Point2<f64>> {
    vec![
        Point2::new(-HALF_BASE, 0.0),
        Point2::new(HALF_BASE, 0.0),
        Point2::new(0.0, APEX_Y),
    ]
}

/// Outer white triangle with asymmetric inward offsets:
///   AB (base, no letters) : BORDER_OFFSET_BASE outward
///   BC (MERLIN side)      : BORDER_OFFSET_SIDES outward
///   CA (LEROY side)       : BORDER_OFFSET_SIDES outward
///
/// Offset edge equations
///   AB′ :   y = −d_b
///   BC′ :   x +  y = 25 + d_s·√2
///   CA′ :  −x +  y = 25 + d_s·√2
///
/// Corner intersections
///   A′ = (−(25 + d_b + d_s·√2),  −d_b)
///   B′ = (  25 + d_b + d_s·√2,   −d_b)
///   C′ = (0,  25 + d_s·√2)
pub fn outer_tri_verts() -> Vec<Point2<f64>> {
    let d_b = BORDER_OFFSET_BASE;
    let d_s = BORDER_OFFSET_SIDES;
    let dsq = d_s * SQRT_2;

    vec![
        Point2::new(-(HALF_BASE + d_b + dsq), -d_b),
        Point2::new(HALF_BASE + d_b + dsq, -d_b),
        Point2::new(0.0, APEX_Y + dsq),
    ]
}

/// Same outer triangle, named for the backside support.
pub fn support_back_outer_verts() -> Vec<Point2<f64>> {
    outer_tri_verts()
}

/// Uniform inward inset of the support triangle, used as the hollow pocket.
/// Returns `None` if the wall thickness is invalid.
pub fn support_back_inner_verts(wall: f64) -> Option<Vec<Point2<f64>>> {
    let outer = support_back_outer_verts();
    let tri = [outer[0], outer[1], outer[2]];
    inset_ccw_triangle(&tri, wall)
}

/// Turn a segment into a thin rectangle polygon, centered on the segment.
pub fn rib_strip_verts(p1: Point2<f64>, p2: Point2<f64>, width: f64) -> Vec<Point2<f64>> {
    let d = (p2 - p1).normalize();
    let n = Vector2::new(-d.y, d.x) * (width * 0.5);

    vec![
        p1 + n,
        p2 + n,
        p2 - n,
        p1 - n,
    ]
}

/// Inset a CCW triangle inward by `inset` using parallel edge offsets.
/// The returned triangle is the pocket boundary for the hollow support.
pub fn inset_ccw_triangle(tri: &[Point2<f64>; 3], inset: f64) -> Option<Vec<Point2<f64>>> {
    if inset <= 0.0 {
        return Some(tri.to_vec());
    }

    // Each edge is stored as an inward-facing unit normal `n` and constant `c`
    // for the half-plane `n·x >= c`.
    let mut edges: Vec<(Vector2<f64>, f64)> = Vec::with_capacity(3);

    for i in 0..3 {
        let a = tri[i];
        let b = tri[(i + 1) % 3];
        let d = b - a;

        // CCW polygon: inward normal is a left turn of the edge direction.
        let n = Vector2::new(-d.y, d.x).normalize();
        let c = n.dot(&a.coords) + inset;
        edges.push((n, c));
    }

    let mut out = Vec::with_capacity(3);
    for i in 0..3 {
        let (n_prev, c_prev) = edges[(i + 2) % 3];
        let (n_curr, c_curr) = edges[i];

        let p = intersect_lines(n_prev, c_prev, n_curr, c_curr)?;
        out.push(p);
    }

    Some(out)
}

fn intersect_lines(
    n1: Vector2<f64>,
    c1: f64,
    n2: Vector2<f64>,
    c2: f64,
) -> Option<Point2<f64>> {
    let det = n1.x * n2.y - n1.y * n2.x;
    if det.abs() < 1e-12 {
        return None;
    }

    let x = (c1 * n2.y - n1.y * c2) / det;
    let y = (n1.x * c2 - c1 * n2.x) / det;
    Some(Point2::new(x, y))
}

// ── Geometry utilities ─────────────────────────────────────────────────────

/// Euclidean distance between two 2-D points.
pub fn edge_length(a: Point2<f64>, b: Point2<f64>) -> f64 {
    (b - a).norm()
}

/// Unit direction vector from `a` to `b`.
pub fn edge_dir(a: Point2<f64>, b: Point2<f64>) -> Vector2<f64> {
    (b - a).normalize()
}

/// Rough pixel-width estimate for laid-out text.
pub fn estimated_text_width(text: &str, font_height: f64) -> f64 {
    text.chars().count() as f64 * 0.55 * font_height
}

// ── Ray–triangle clipper ───────────────────────────────────────────────────

/// Clip the parametric ray `origin + t·dir` to the interior of the inner
/// logo triangle.  Returns `(entry, exit)` in 2-D world coordinates, or
/// `None` if the ray misses the triangle entirely.
///
/// Half-plane constraints that define the interior (n·x ≥ c):
///   y ≥ 0          (base AB)
///   x − y ≥ −25   (left edge CA)
///   −x − y ≥ −25  (right edge BC, equivalent to x + y ≤ 25)
pub fn clip_to_triangle(
    origin: Point2<f64>,
    dir: Vector2<f64>,
) -> Option<(Point2<f64>, Point2<f64>)> {
    let planes: &[(f64, f64, f64)] = &[
        (0.0, 1.0, 0.0),
        (1.0, -1.0, -HALF_BASE),
        (-1.0, -1.0, -APEX_Y),
    ];

    let mut t_lo = f64::NEG_INFINITY;
    let mut t_hi = f64::INFINITY;

    for &(nx, ny, c) in planes {
        let n_dot_d = nx * dir.x + ny * dir.y;
        let n_dot_o = nx * origin.x + ny * origin.y;
        let slack = n_dot_o - c;

        if n_dot_d.abs() < 1e-12 {
            if slack < -1e-9 {
                return None;
            }
        } else if n_dot_d > 0.0 {
            t_lo = t_lo.max(-slack / n_dot_d);
        } else {
            t_hi = t_hi.min(-slack / n_dot_d);
        }
    }

    if t_hi - t_lo < 1e-6 {
        return None;
    }

    Some((origin + t_lo * dir, origin + t_hi * dir))
}

// ── Grid lines ─────────────────────────────────────────────────────────────

/// 45 ° diagonal grid drawn on the underside of the support (z = −SUPPORT_DEPTH).
/// Three lines parallel to AC/BC and two lines perpendicular to them.
pub fn compute_grid_lines() -> Vec<LineSpec> {
    let z = -SUPPORT_DEPTH;
    let d_par = Vector2::new(1.0 / SQRT_2, 1.0 / SQRT_2);
    let d_perp = Vector2::new(1.0 / SQRT_2, -1.0 / SQRT_2);

    let span = HALF_BASE * SQRT_2;
    let base = Point2::new(-HALF_BASE, 0.0);
    let mut lines = Vec::new();

    for k in 1_u32..=3 {
        let offset = span * (k as f64) / 4.0;
        let origin = base + offset * d_perp;
        if let Some((p1, p2)) = clip_to_triangle(origin, d_par) {
            lines.push(LineSpec {
                p1: Point3::new(p1.x, p1.y, z),
                p2: Point3::new(p2.x, p2.y, z),
                color: grid_grey(),
                label: format!("grid_par_{k}"),
            });
        }
    }

    for k in 1_u32..=2 {
        let s = span * (k as f64) / 3.0;
        let origin = base + s * d_par;
        if let Some((p1, p2)) = clip_to_triangle(origin, d_perp) {
            lines.push(LineSpec {
                p1: Point3::new(p1.x, p1.y, z),
                p2: Point3::new(p2.x, p2.y, z),
                color: grid_grey(),
                label: format!("grid_perp_{k}"),
            });
        }
    }

    lines
}
