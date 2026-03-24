// /home/emporas/repos/freecad/rust/roy/src/types.rs
use pyo3::prelude::*;

// ═══════════════════════════════════════════════════════════════════════════════
// § 1  Build parameters
// ═══════════════════════════════════════════════════════════════════════════════

#[pyclass]
#[derive(Clone)]
pub struct RoyParams {
    #[pyo3(get, set)] pub side:           f64,
    #[pyo3(get, set)] pub floor_height:   f64,
    #[pyo3(get, set)] pub slab_thickness: f64,
    #[pyo3(get, set)] pub wall_thickness: f64,
    #[pyo3(get, set)] pub shelf_width:    f64,
    #[pyo3(get, set)] pub shelf_depth:    f64,
    #[pyo3(get, set)] pub shelf_height:   f64,
    #[pyo3(get, set)] pub shelf_levels:   u32,
    #[pyo3(get, set)] pub internal_rows:  u32,
    #[pyo3(get, set)] pub compute_items:  bool,
}

#[pymethods]
impl RoyParams {
    #[new]
    #[pyo3(signature = (
        side           = 100_000.0,
        floor_height   =  10_000.0,
        slab_thickness =     300.0,
        wall_thickness =     400.0,
        shelf_width    =   4_000.0,
        shelf_depth    =     600.0,
        shelf_height   =   4_000.0,
        shelf_levels   =         4,
        internal_rows  =         4,
        compute_items  =      true,
    ))]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        side: f64, floor_height: f64, slab_thickness: f64, wall_thickness: f64,
        shelf_width: f64, shelf_depth: f64, shelf_height: f64,
        shelf_levels: u32, internal_rows: u32,
        compute_items: bool,
    ) -> Self {
        Self {
            side, floor_height, slab_thickness, wall_thickness,
            shelf_width, shelf_depth, shelf_height, shelf_levels, internal_rows,
            compute_items,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// § 2  Spec types  (Rust → FreeCAD)
// ═══════════════════════════════════════════════════════════════════════════════

/// A flat horizontal structural plate belonging to one store building.
#[pyclass]
pub struct SlabSpec {
    #[pyo3(get)] pub label:     String,
    #[pyo3(get)] pub store:     String,
    #[pyo3(get)] pub x:         f64,
    #[pyo3(get)] pub y:         f64,
    #[pyo3(get)] pub z:         f64,
    #[pyo3(get)] pub length:    f64,
    #[pyo3(get)] pub width:     f64,
    #[pyo3(get)] pub thickness: f64,
}

/// A BIM wall belonging to one store building.
#[pyclass]
pub struct WallSpec {
    #[pyo3(get)] pub label:  String,
    #[pyo3(get)] pub store:  String,
    #[pyo3(get)] pub x1:     f64,
    #[pyo3(get)] pub y1:     f64,
    #[pyo3(get)] pub x2:     f64,
    #[pyo3(get)] pub y2:     f64,
    #[pyo3(get)] pub z:      f64,
    #[pyo3(get)] pub height: f64,
    #[pyo3(get)] pub width:  f64,
    #[pyo3(get)] pub align:  String,
}

/// One shelf unit (gondola) belonging to one store building.
#[pyclass]
pub struct ShelfSpec {
    #[pyo3(get)] pub label:    String,
    #[pyo3(get)] pub store:    String,
    #[pyo3(get)] pub x:        f64,
    #[pyo3(get)] pub y:        f64,
    #[pyo3(get)] pub z:        f64,
    #[pyo3(get)] pub sx:       f64,
    #[pyo3(get)] pub sy:       f64,
    #[pyo3(get)] pub sz:       f64,
    #[pyo3(get)] pub role:     String,
    #[pyo3(get)] pub color:    (f32, f32, f32),
    #[pyo3(get)] pub quadrant: String,
    #[pyo3(get)] pub row:      u32,
    #[pyo3(get)] pub col:      u32,
}

/// One inventory slot — one vertical level on one shelf unit.
///
/// `color` is the per-slot highlight colour set via the CLI (`--red`, `--blue`,
/// `--yellow`, `--green`).  Defaults to white `(1.0, 1.0, 1.0)` which the
/// FreeCAD macro interprets as "no highlight".
#[pyclass]
pub struct ShelfItemSpec {
    #[pyo3(get)] pub coord_label: String,
    #[pyo3(get)] pub shelf_label: String,
    #[pyo3(get)] pub price:       f64,
    #[pyo3(get)] pub quantity:    u32,
    #[pyo3(get)] pub world_x:     f64,
    #[pyo3(get)] pub world_y:     f64,
    #[pyo3(get)] pub world_z:     f64,
    #[pyo3(get)] pub level:       u32,
    /// Per-slot highlight colour: (r, g, b) each in [0.0, 1.0].
    #[pyo3(get)] pub color:       (f32, f32, f32),
}

// ═══════════════════════════════════════════════════════════════════════════════
// § 3  Current-scene snapshot types  (FreeCAD → Python → Rust)
// ═══════════════════════════════════════════════════════════════════════════════

#[pyclass]
#[derive(Clone)]
pub struct CurrentSlabState {
    #[pyo3(get, set)] pub label:     String,
    #[pyo3(get, set)] pub x:         f64,
    #[pyo3(get, set)] pub y:         f64,
    #[pyo3(get, set)] pub z:         f64,
    #[pyo3(get, set)] pub length:    f64,
    #[pyo3(get, set)] pub width:     f64,
    #[pyo3(get, set)] pub thickness: f64,
}

#[pymethods]
impl CurrentSlabState {
    #[new]
    pub fn new(label: String, x: f64, y: f64, z: f64,
               length: f64, width: f64, thickness: f64) -> Self {
        Self { label, x, y, z, length, width, thickness }
    }
}

#[pyclass]
#[derive(Clone)]
pub struct CurrentWallState {
    #[pyo3(get, set)] pub label:  String,
    #[pyo3(get, set)] pub x1:     f64,
    #[pyo3(get, set)] pub y1:     f64,
    #[pyo3(get, set)] pub x2:     f64,
    #[pyo3(get, set)] pub y2:     f64,
    #[pyo3(get, set)] pub z:      f64,
    #[pyo3(get, set)] pub height: f64,
    #[pyo3(get, set)] pub width:  f64,
}

#[pymethods]
impl CurrentWallState {
    #[new]
    #[allow(clippy::too_many_arguments)]
    pub fn new(label: String, x1: f64, y1: f64, x2: f64, y2: f64,
               z: f64, height: f64, width: f64) -> Self {
        Self { label, x1, y1, x2, y2, z, height, width }
    }
}

#[pyclass]
#[derive(Clone)]
pub struct CurrentShelfState {
    #[pyo3(get, set)] pub label:    String,
    #[pyo3(get, set)] pub x:        f64,
    #[pyo3(get, set)] pub y:        f64,
    #[pyo3(get, set)] pub z:        f64,
    #[pyo3(get, set)] pub sx:       f64,
    #[pyo3(get, set)] pub sy:       f64,
    #[pyo3(get, set)] pub sz:       f64,
    #[pyo3(get, set)] pub role:     String,
    #[pyo3(get, set)] pub quadrant: String,
    #[pyo3(get, set)] pub row:      u32,
    #[pyo3(get, set)] pub col:      u32,
}

#[pymethods]
impl CurrentShelfState {
    #[new]
    #[allow(clippy::too_many_arguments)]
    pub fn new(label: String, x: f64, y: f64, z: f64, sx: f64, sy: f64, sz: f64,
               role: String, quadrant: String, row: u32, col: u32) -> Self {
        Self { label, x, y, z, sx, sy, sz, role, quadrant, row, col }
    }
}

#[pyclass]
#[derive(Clone)]
pub struct CurrentItemState {
    #[pyo3(get, set)] pub coord_label: String,
    #[pyo3(get, set)] pub price:       f64,
    #[pyo3(get, set)] pub quantity:    u32,
    #[pyo3(get, set)] pub world_x:     f64,
    #[pyo3(get, set)] pub world_y:     f64,
    #[pyo3(get, set)] pub world_z:     f64,
    #[pyo3(get, set)] pub color_r:     f32,
    #[pyo3(get, set)] pub color_g:     f32,
    #[pyo3(get, set)] pub color_b:     f32,
}

#[pymethods]
impl CurrentItemState {
    #[new]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        coord_label: String, price: f64, quantity: u32,
        world_x: f64, world_y: f64, world_z: f64,
        color_r: f32, color_g: f32, color_b: f32,
    ) -> Self {
        Self { coord_label, price, quantity, world_x, world_y, world_z,
               color_r, color_g, color_b }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// § 4  Diff result types  (Rust → Python)
// ═══════════════════════════════════════════════════════════════════════════════

#[pyclass]
pub struct BuildingDiff {
    #[pyo3(get)] pub slabs_add:      Vec<PyObject>,
    #[pyo3(get)] pub slabs_update:   Vec<PyObject>,
    #[pyo3(get)] pub slabs_remove:   Vec<String>,
    #[pyo3(get)] pub walls_add:      Vec<PyObject>,
    #[pyo3(get)] pub walls_update:   Vec<PyObject>,
    #[pyo3(get)] pub walls_remove:   Vec<String>,
    #[pyo3(get)] pub shelves_add:    Vec<PyObject>,
    #[pyo3(get)] pub shelves_update: Vec<PyObject>,
    #[pyo3(get)] pub shelves_remove: Vec<String>,
}

#[pyclass]
pub struct ItemDiff {
    #[pyo3(get)] pub add:    Vec<PyObject>,
    #[pyo3(get)] pub update: Vec<PyObject>,
    #[pyo3(get)] pub remove: Vec<String>,
}
