// /home/emporas/repos/freecad/rust/roy/src/lib.rs
mod diff;

pub use primitives::{
    RoyParams, ShelfItemSpec, ShelfSpec, SlabSpec, WallSpec,
};

use diff::{DiffResult, compute_diff};
use pyo3::prelude::*;

/// Load changes from the database and diff against per-entity fingerprint caches.
///
/// # Arguments
/// * `prev_slabs`   — `label → content_hash` for slabs from the previous call.
/// * `prev_walls`   — `label → content_hash` for walls from the previous call.
/// * `prev_shelves` — `label → content_hash` for shelves from the previous call.
/// * `prev_items`   — `coord_label → content_hash` for items from the previous call.
///
/// Pass empty dicts `{}` for all four on the very first call (full load).
///
/// * `db_path` — filesystem path to the SQLite database file.
///               Falls back to the XDG default (`~/.local/share/roy/roy.db`).
/// * `since`   — ISO-8601 timestamp returned as `DiffResult.sync_at` from the
///               previous call.  When supplied, only rows with
///               `updated_at >= since` are fetched in full; a separate
///               key-only scan detects deletions.  Pass `None` on the first
///               call to force a complete fetch.
///
/// # Returns
/// A `DiffResult` describing the minimal FreeCAD mutations needed.
/// Store the four `*_fingerprints` maps and `sync_at`, then pass them back
/// on the next call.
///
/// # Why four separate maps?
/// Passing a single combined map to per-entity `get_*` functions caused every
/// key from *other* tables to appear as a deletion in each table, wiping the
/// entire building on every incremental update.  Separate maps eliminate that
/// cross-contamination entirely.
#[pyfunction]
#[pyo3(signature = (
    prev_slabs,
    prev_walls,
    prev_shelves,
    prev_items,
    db_path = None,
    since   = None,
))]
fn diff_all_specs(
    py:           Python<'_>,
    prev_slabs:   std::collections::HashMap<String, u64>,
    prev_walls:   std::collections::HashMap<String, u64>,
    prev_shelves: std::collections::HashMap<String, u64>,
    prev_items:   std::collections::HashMap<String, u64>,
    db_path:      Option<String>,
    since:        Option<String>,
) -> PyResult<DiffResult> {
    let path = db_path
        .map(std::path::PathBuf::from)
        .unwrap_or_else(db::default_db_path);
    compute_diff(py, &path, &prev_slabs, &prev_walls, &prev_shelves, &prev_items, since.as_deref())
}

#[pymodule]
fn roy(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<RoyParams>()?;
    m.add_class::<SlabSpec>()?;
    m.add_class::<WallSpec>()?;
    m.add_class::<ShelfSpec>()?;
    m.add_class::<ShelfItemSpec>()?;
    m.add_class::<DiffResult>()?;

    m.add_function(wrap_pyfunction!(diff_all_specs, m)?)?;
    Ok(())
}
