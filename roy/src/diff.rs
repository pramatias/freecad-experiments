// /home/emporas/repos/freecad/rust/roy/src/diff.rs
//! DB-backed diff — returns the minimal set of Python objects for FreeCAD.
//!
//! On the first call pass all four `prev_*` maps as empty `{}` and `since = None`
//! for a full load.  On every subsequent call pass back the four `*_fingerprints`
//! maps and the `sync_at` string that were returned in the previous `DiffResult`.
//!
//! Each `prev_*` map is kept strictly per-entity-type so that keys from one
//! table can never be mistaken for deletions in another table.

use std::collections::HashMap;
use std::path::Path;

use pyo3::prelude::*;
use pyo3::exceptions::PyRuntimeError;

use db::queries::{
    get_items, get_slabs, get_walls, get_shelves,
    ItemRow, SlabRow, WallRow, ShelfRow,
};
use primitives::{ShelfItemSpec, ShelfSpec, SlabSpec, WallSpec};

// ─── DiffResult ───────────────────────────────────────────────────────────────

#[pyclass]
pub struct DiffResult {
    #[pyo3(get)] pub building_add:    Vec<PyObject>,
    #[pyo3(get)] pub building_update: Vec<PyObject>,
    #[pyo3(get)] pub building_remove: Vec<String>,

    #[pyo3(get)] pub item_add:    Vec<PyObject>,
    #[pyo3(get)] pub item_update: Vec<PyObject>,
    #[pyo3(get)] pub item_remove: Vec<String>,

    /// Per-entity fingerprint maps — pass each back as the matching `prev_*`
    /// argument on the next call.  Keeping them separate prevents keys from one
    /// table being mis-classified as deletions in another table.
    #[pyo3(get)] pub slab_fingerprints:  HashMap<String, u64>,
    #[pyo3(get)] pub wall_fingerprints:  HashMap<String, u64>,
    #[pyo3(get)] pub shelf_fingerprints: HashMap<String, u64>,
    #[pyo3(get)] pub item_fingerprints:  HashMap<String, u64>,

    /// ISO-8601 timestamp captured at the start of this diff.
    /// Store it and pass it back as `since` on the next call so the DB
    /// queries only scan rows modified after this point.
    #[pyo3(get)] pub sync_at: String,

    #[pyo3(get)] pub n_add:       usize,
    #[pyo3(get)] pub n_update:    usize,
    #[pyo3(get)] pub n_remove:    usize,
    #[pyo3(get)] pub n_unchanged: usize,
}

// ─── Row → Spec conversions ───────────────────────────────────────────────────

fn slab_row_to_spec(r: SlabRow) -> SlabSpec {
    SlabSpec {
        label:     r.label,
        store:     r.store,
        x:         r.x,
        y:         r.y,
        z:         r.z,
        length:    r.length,
        width:     r.width,
        thickness: r.thickness,
    }
}

fn wall_row_to_spec(r: WallRow) -> WallSpec {
    WallSpec {
        label:  r.label,
        store:  r.store,
        x1:     r.x1,
        y1:     r.y1,
        x2:     r.x2,
        y2:     r.y2,
        z:      r.z,
        height: r.height,
        width:  r.width,
        align:  r.align,
    }
}

fn shelf_row_to_spec(r: ShelfRow) -> ShelfSpec {
    ShelfSpec {
        label:    r.label,
        store:    r.store,
        x:        r.x,
        y:        r.y,
        z:        r.z,
        sx:       r.sx,
        sy:       r.sy,
        sz:       r.sz,
        role:     r.role,
        color:    (r.color_r, r.color_g, r.color_b),
        quadrant: r.quadrant,
        row:      r.row,
        col:      r.col,
    }
}

fn item_row_to_spec(r: ItemRow) -> ShelfItemSpec {
    ShelfItemSpec {
        coord_label: r.coord_label,
        shelf_label: r.shelf_label,
        price:       r.price,
        quantity:    r.quantity,
        world_x:     r.world_x,
        world_y:     r.world_y,
        world_z:     r.world_z,
        level:       r.level,
        color:       (r.color_r, r.color_g, r.color_b),
    }
}

// ─── Core diff ────────────────────────────────────────────────────────────────

pub fn compute_diff(
    py:           Python<'_>,
    db_path:      &Path,
    prev_slabs:   &HashMap<String, u64>,
    prev_walls:   &HashMap<String, u64>,
    prev_shelves: &HashMap<String, u64>,
    prev_items:   &HashMap<String, u64>,
    since:        Option<&str>,
) -> PyResult<DiffResult> {

    let conn = db::open(db_path)
        .map_err(|e| PyRuntimeError::new_err(
            format!("Cannot open database at {db_path:?}: {e}")
        ))?;

    // Capture the DB clock before touching any rows.  Any write that races in
    // after this instant will have updated_at > sync_at and will therefore be
    // picked up on the *next* incremental call — no updates are ever skipped.
    let sync_at: String = conn
        .query_row(
            "SELECT strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
            [],
            |r| r.get(0),
        )
        .map_err(|e| PyRuntimeError::new_err(format!("Failed to read DB clock: {e}")))?;

    // Each get_* receives only its own per-entity fingerprint map, so keys from
    // other tables can never be mistaken for deletions in this table.
    let slab_delta  = get_slabs(&conn, Some(prev_slabs), None, since)
        .map_err(|e| PyRuntimeError::new_err(format!("get_slabs failed: {e}")))?;
    let wall_delta  = get_walls(&conn, Some(prev_walls), None, since)
        .map_err(|e| PyRuntimeError::new_err(format!("get_walls failed: {e}")))?;
    let shelf_delta = get_shelves(&conn, Some(prev_shelves), None, None, since)
        .map_err(|e| PyRuntimeError::new_err(format!("get_shelves failed: {e}")))?;
    let item_delta  = get_items(&conn, Some(prev_items), None, since)
        .map_err(|e| PyRuntimeError::new_err(format!("get_items failed: {e}")))?;

    // ── Build new per-entity fingerprint maps ─────────────────────────────────
    //
    // Start from the previous per-entity maps so unchanged rows are carried
    // forward without re-fetching.

    let mut slab_fingerprints  = prev_slabs.clone();
    let mut wall_fingerprints  = prev_walls.clone();
    let mut shelf_fingerprints = prev_shelves.clone();
    let mut item_fingerprints  = prev_items.clone();

    for key in &slab_delta.removed  { slab_fingerprints.remove(key);  }
    for key in &wall_delta.removed  { wall_fingerprints.remove(key);  }
    for key in &shelf_delta.removed { shelf_fingerprints.remove(key); }
    for key in &item_delta.removed  { item_fingerprints.remove(key);  }

    // ── Building additions ────────────────────────────────────────────────────

    let mut building_add: Vec<PyObject> = Vec::new();

    for row in slab_delta.added {
        slab_fingerprints.insert(row.label.clone(), row.content_hash);
        building_add.push(Py::new(py, slab_row_to_spec(row))?.into());
    }
    for row in wall_delta.added {
        wall_fingerprints.insert(row.label.clone(), row.content_hash);
        building_add.push(Py::new(py, wall_row_to_spec(row))?.into());
    }
    for row in shelf_delta.added {
        shelf_fingerprints.insert(row.label.clone(), row.content_hash);
        building_add.push(Py::new(py, shelf_row_to_spec(row))?.into());
    }

    // ── Building updates ──────────────────────────────────────────────────────

    let mut building_update: Vec<PyObject> = Vec::new();

    for row in slab_delta.updated {
        slab_fingerprints.insert(row.label.clone(), row.content_hash);
        building_update.push(Py::new(py, slab_row_to_spec(row))?.into());
    }
    for row in wall_delta.updated {
        wall_fingerprints.insert(row.label.clone(), row.content_hash);
        building_update.push(Py::new(py, wall_row_to_spec(row))?.into());
    }
    for row in shelf_delta.updated {
        shelf_fingerprints.insert(row.label.clone(), row.content_hash);
        building_update.push(Py::new(py, shelf_row_to_spec(row))?.into());
    }

    // ── Building removals ─────────────────────────────────────────────────────

    let mut building_remove: Vec<String> = Vec::new();
    building_remove.extend(slab_delta.removed);
    building_remove.extend(wall_delta.removed);
    building_remove.extend(shelf_delta.removed);

    // ── Item additions ────────────────────────────────────────────────────────

    let mut item_add: Vec<PyObject> = Vec::with_capacity(item_delta.added.len());
    for row in item_delta.added {
        item_fingerprints.insert(row.coord_label.clone(), row.content_hash);
        item_add.push(Py::new(py, item_row_to_spec(row))?.into());
    }

    // ── Item updates ──────────────────────────────────────────────────────────

    let mut item_update: Vec<PyObject> = Vec::with_capacity(item_delta.updated.len());
    for row in item_delta.updated {
        item_fingerprints.insert(row.coord_label.clone(), row.content_hash);
        item_update.push(Py::new(py, item_row_to_spec(row))?.into());
    }

    // ── Counts ────────────────────────────────────────────────────────────────

    let n_add    = building_add.len()    + item_add.len();
    let n_update = building_update.len() + item_update.len();
    let n_remove = building_remove.len() + item_delta.removed.len();
    let n_unchanged = (slab_fingerprints.len() + wall_fingerprints.len()
        + shelf_fingerprints.len() + item_fingerprints.len())
        .saturating_sub(n_add + n_update);

    Ok(DiffResult {
        building_add,
        building_update,
        building_remove,
        item_remove: item_delta.removed,
        item_add,
        item_update,
        slab_fingerprints,
        wall_fingerprints,
        shelf_fingerprints,
        item_fingerprints,
        sync_at,
        n_add,
        n_update,
        n_remove,
        n_unchanged,
    })
}
