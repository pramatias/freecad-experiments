// /home/emporas/repos/freecad/rust/logo/src/logo.rs
//! Assembles the complete `LogoSpecs` from the geometry primitives.

use nalgebra::{Point2, Point3, Vector2};
use std::f64::consts::SQRT_2;

use primitives::datatypes::{
    Anchor, BoltHoleSpec, FaceExtrudeSpec, LogoSpecs, SupportBackSpec, TextSpec,
};
use primitives::geometry::{
    APEX_Y, BORDER_OFFSET_SIDES, LOGO_DEPTH, SUPPORT_DEPTH, TEXT_DEPTH, black, compute_grid_lines,
    inner_tri_verts, leroy_green, outer_tri_verts, rib_strip_verts, support_back_inner_verts,
    support_back_outer_verts, white,
};

const LEROY_GAPS: [f64; 4] = [3.0, 4.0, 4.0, 4.0]; // L→E, E→R, R→O, O→Y
const MERLIN_GAPS: [f64; 5] = [3.7, 4.7, 3.1, 0.3, 6.3]; // M→E, E→R, R→L, L→I, I→N

const SUPPORT_WALL: f64 = 1.0;
const SUPPORT_RIB_WIDTH: f64 = 1.2;
const SUPPORT_RIB_DEPTH: f64 = 1.8;

// ── Offset directions ──────────────────────────────────────────────
#[derive(Clone, Copy)]
enum TriangleEdge {
    CA,
    BC,
}

impl TriangleEdge {
    /// Inward normal from the chosen white-triangle edge.
    fn inward(self) -> Vector2<f64> {
        match self {
            // Edge C→A : inward points down-right.
            TriangleEdge::CA => Vector2::new(1.0 / SQRT_2, -1.0 / SQRT_2),

            // Edge B→C : inward points down-left.
            TriangleEdge::BC => Vector2::new(-1.0 / SQRT_2, -1.0 / SQRT_2),
        }
    }

    /// The 45° direction the word should slide along.
    fn down_slant(self) -> Vector2<f64> {
        match self {
            TriangleEdge::CA => Vector2::new(-1.0 / SQRT_2, -1.0 / SQRT_2),
            TriangleEdge::BC => Vector2::new(1.0 / SQRT_2, -1.0 / SQRT_2),
        }
    }
}

#[derive(Clone, Copy)]
struct TextMotion {
    edge: TriangleEdge,
    edge_offset: f64,
    slide_down: f64,
}

impl TextMotion {
    fn apply(self, base: Point3<f64>) -> Point3<f64> {
        let n = self.edge.inward();
        let s = self.edge.down_slant();

        Point3::new(
            base.x + n.x * self.edge_offset + s.x * self.slide_down,
            base.y + n.y * self.edge_offset + s.y * self.slide_down,
            base.z,
        )
    }
}

fn build_texts(leroy_down_offset: f64, merlin_down_offset: f64) -> Vec<TextSpec> {
    let z = LOGO_DEPTH - TEXT_DEPTH;

    let leroy_base0 = Point3::new(
        -BORDER_OFFSET_SIDES / SQRT_2,
        APEX_Y + BORDER_OFFSET_SIDES / SQRT_2,
        z,
    );

    let merlin_base0 = Point3::new(
        BORDER_OFFSET_SIDES / SQRT_2,
        APEX_Y + BORDER_OFFSET_SIDES / SQRT_2,
        z,
    );

    let leroy_motion = TextMotion {
        edge: TriangleEdge::CA,
        edge_offset: leroy_down_offset,
        slide_down: leroy_down_offset,
    };

    let merlin_motion = TextMotion {
        edge: TriangleEdge::BC,
        edge_offset: merlin_down_offset,
        slide_down: (merlin_down_offset / 8.0),
    };

    let leroy_base = leroy_motion.apply(leroy_base0);
    let merlin_base = merlin_motion.apply(merlin_base0);

    let left_dir = Vector2::new(-1.0 / SQRT_2, -1.0 / SQRT_2);
    let right_dir = Vector2::new(1.0 / SQRT_2, -1.0 / SQRT_2);

    let mut texts = build_letter_run(
        "LEROY",
        leroy_base,
        45.0,
        Anchor::EndPeak,
        left_dir,
        &LEROY_GAPS,
        true,
        "text_LEROY",
    );

    texts.extend(build_letter_run(
        "MERLIN",
        merlin_base,
        -45.0,
        Anchor::EndPeak,
        right_dir,
        &MERLIN_GAPS,
        false,
        "text_MERLIN",
    ));

    texts
}

// ── Text relief ────────────────────────────────────────────────────────────

fn glyph_advance(ch: char, normal: f64) -> f64 {
    match ch {
        'I' => normal / 1.5,
        _ => normal,
    }
}

fn build_letter_run(
    word: &str,
    base: Point3<f64>,
    rotation_deg: f64,
    anchor: Anchor,
    step_dir: Vector2<f64>,
    gaps_between: &[f64],
    anchor_at_end: bool,
    label_prefix: &str,
) -> Vec<TextSpec> {
    let font_h = 6.5_f64;
    let text_z_bottom = LOGO_DEPTH - TEXT_DEPTH;
    let normal_advance = 0.55 * font_h;

    let chars: Vec<char> = word.chars().collect();
    assert_eq!(
        gaps_between.len(),
        chars.len().saturating_sub(1),
        "gaps_between must have one entry per gap between letters"
    );

    let advances: Vec<f64> = chars
        .iter()
        .map(|&ch| glyph_advance(ch, normal_advance))
        .collect();

    let mut starts = Vec::with_capacity(chars.len());
    let mut cursor = 0.0;
    for i in 0..chars.len() {
        starts.push(cursor);
        if i + 1 < chars.len() {
            cursor += advances[i] + gaps_between[i];
        }
    }

    let total_width = cursor + advances.last().copied().unwrap_or(0.0);

    chars
        .into_iter()
        .enumerate()
        .map(|(i, ch)| {
            let along = if anchor_at_end {
                total_width - starts[i] - advances[i]
            } else {
                starts[i]
            };

            let offset = step_dir * along;

            TextSpec {
                text: ch.to_string(),
                position: Point3::new(base.x + offset.x, base.y + offset.y, text_z_bottom),
                rotation_deg,
                font_height: font_h,
                extrude_depth: TEXT_DEPTH,
                color: black(),
                label: format!("{}_{}", label_prefix, i + 1),
                anchor: anchor.clone(),
            }
        })
        .collect()
}

// ── Public entry point ─────────────────────────────────────────────────────

pub fn build_logo_specs(leroy_down_offset: f64, merlin_down_offset: f64) -> LogoSpecs {
    LogoSpecs {
        faces: build_faces(),
        lines: compute_grid_lines(),
        texts: build_texts(leroy_down_offset, merlin_down_offset),
    }
}

// ── Faces ──────────────────────────────────────────────────────────────────

fn build_faces() -> Vec<FaceExtrudeSpec> {
    let outer_white = FaceExtrudeSpec {
        vertices_2d: outer_tri_verts(),
        z_base: -SUPPORT_DEPTH,
        extrude_height: SUPPORT_DEPTH + LOGO_DEPTH - 1.0,
        color: white(),
        transparency: 0,
        label: "OuterWhite".into(),
    };

    let logo = FaceExtrudeSpec {
        vertices_2d: inner_tri_verts(),
        z_base: 0.0,
        extrude_height: LOGO_DEPTH,
        color: leroy_green(),
        transparency: 0,
        label: "Logo".into(),
    };

    vec![outer_white, logo]
}

pub fn build_support_back_spec() -> SupportBackSpec {
    let outer = support_back_outer_verts();
    let inner = support_back_inner_verts(SUPPORT_WALL)
        .expect("invalid SUPPORT_WALL for support back inset");

    let z_back = -SUPPORT_DEPTH;

    let apex = outer[2];
    let left_base = outer[0];
    let right_base = outer[1];

    let rib_len = 8.0;

    let left_rib = FaceExtrudeSpec {
        vertices_2d: rib_strip_verts(
            left_base,
            Point2::new(
                left_base.x + rib_len * (1.0 / SQRT_2),
                left_base.y + rib_len * (1.0 / SQRT_2),
            ),
            SUPPORT_RIB_WIDTH,
        ),
        z_base: z_back,
        extrude_height: SUPPORT_RIB_DEPTH,
        color: white(),
        transparency: 0,
        label: "support_corner_rib_left".into(),
    };

    let right_rib = FaceExtrudeSpec {
        vertices_2d: rib_strip_verts(
            right_base,
            Point2::new(
                right_base.x - rib_len * (1.0 / SQRT_2),
                right_base.y + rib_len * (1.0 / SQRT_2),
            ),
            SUPPORT_RIB_WIDTH,
        ),
        z_base: z_back,
        extrude_height: SUPPORT_RIB_DEPTH,
        color: white(),
        transparency: 0,
        label: "support_corner_rib_right".into(),
    };

    let apex_rib = FaceExtrudeSpec {
        vertices_2d: rib_strip_verts(
            apex,
            Point2::new(apex.x, apex.y - rib_len),
            SUPPORT_RIB_WIDTH,
        ),
        z_base: z_back,
        extrude_height: SUPPORT_RIB_DEPTH,
        color: white(),
        transparency: 0,
        label: "support_corner_rib_apex".into(),
    };

    let center = Point2::new(0.0, 8.0);

    let center_square = FaceExtrudeSpec {
        vertices_2d: square_verts(center, 12.0),
        z_base: z_back,
        extrude_height: SUPPORT_RIB_DEPTH,
        color: white(),
        transparency: 0,
        label: "support_center_square".into(),
    };

    let bolt_hole = BoltHoleSpec {
        center: Point3::new(center.x, center.y, z_back),
        radius: 1.8,
        depth: SUPPORT_RIB_DEPTH,
        label: "support_bolt_hole".into(),
    };

    SupportBackSpec {
        outer_vertices_2d: outer,
        inner_vertices_2d: inner,
        z_back,
        body_depth: SUPPORT_DEPTH,
        pocket_depth: SUPPORT_DEPTH - SUPPORT_WALL,
        corner_ribs: vec![left_rib, right_rib, apex_rib],
        center_square,
        bolt_hole,
        color: white(),
        transparency: 0,
        label: "Support".into(),
    }
}

fn square_verts(center: Point2<f64>, side: f64) -> Vec<Point2<f64>> {
    let h = side * 0.5;
    vec![
        Point2::new(center.x - h, center.y - h),
        Point2::new(center.x + h, center.y - h),
        Point2::new(center.x + h, center.y + h),
        Point2::new(center.x - h, center.y + h),
    ]
}
