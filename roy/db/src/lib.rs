// /home/emporas/repos/freecad/rust/roy/db/src/lib.rs
pub mod queries;
pub mod schema;

use rusqlite::{Connection, Result, params};
use std::path::Path;
use crate::queries::ShelfRow;

// ── Open helpers ──────────────────────────────────────────────────────────────

/// Run every migration statement, silently skipping already-applied ones
/// (SQLite reports "duplicate column name" as an ordinary error, not a warning).
fn run_migrations(conn: &Connection) -> Result<()> {
    for sql in schema::MIGRATIONS {
        if let Err(e) = conn.execute(sql, []) {
            let msg = e.to_string();
            if !msg.contains("duplicate column name") {
                return Err(e);
            }
        }
    }
    Ok(())
}

pub fn default_db_path() -> std::path::PathBuf {
    let home = std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            directories::UserDirs::new()
                .map(|u| u.home_dir().to_path_buf())
                .unwrap_or_else(|| std::path::PathBuf::from("."))
        });
    home.join(".local").join("share").join("roy").join("roy.db")
}

pub fn open(path: &Path) -> Result<Connection> {
    let conn = Connection::open(path)?;
    conn.execute_batch(schema::DDL)?;
    run_migrations(&conn)?;
    Ok(conn)
}

pub fn open_in_memory() -> Result<Connection> {
    let conn = Connection::open_in_memory()?;
    conn.execute_batch(schema::DDL)?;
    run_migrations(&conn)?;
    Ok(conn)
}

// ── Colour helpers (mirrors queries.rs) ──────────────────────────────────────

pub fn color_from_flag(red: bool, blue: bool, yellow: bool, green: bool) -> (f32, f32, f32) {
    if red    { return (1.0, 0.0, 0.0); }
    if blue   { return (0.0, 0.0, 1.0); }
    if yellow { return (1.0, 1.0, 0.0); }
    if green  { return (0.0, 1.0, 0.0); }
    (1.0, 1.0, 1.0)
}

// ── Content-hash helpers (mirrors queries.rs) ─────────────────────────────────

use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

fn make_hasher() -> DefaultHasher { DefaultHasher::new() }

/// Hash for an inventory slot.  Color is part of the fingerprint so that a
/// CLI colour-change triggers the change-detection diff on the next build.
pub fn fp_item(price: f64, quantity: u32, level: u32, cr: f32, cg: f32, cb: f32) -> u64 {
    let mut h = make_hasher();
    price.to_bits().hash(&mut h);
    quantity.hash(&mut h);
    level.hash(&mut h);
    (cr.to_bits() as u64).hash(&mut h);
    (cg.to_bits() as u64).hash(&mut h);
    (cb.to_bits() as u64).hash(&mut h);
    h.finish()
}

/// Hash for a shelf gondola.  Position AND colour are both included so that
/// either a move or a colour change is detected as a diff on the next build.
pub fn fp_shelf(x: f64, y: f64, z: f64, r: f32, g: f32, b: f32) -> u64 {
    let mut h = make_hasher();
    x.to_bits().hash(&mut h);
    y.to_bits().hash(&mut h);
    z.to_bits().hash(&mut h);
    (r.to_bits() as u64).hash(&mut h);
    (g.to_bits() as u64).hash(&mut h);
    (b.to_bits() as u64).hash(&mut h);
    h.finish()
}

// ── Store-level write helpers ─────────────────────────────────────────────────

pub fn ensure_shelf(conn: &Connection, row: &ShelfRow) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO shelves
             (label, store, x, y, z, sx, sy, sz, role,
              color_r, color_g, color_b, quadrant, row, col, content_hash)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)",
        params![
            row.label, row.store,
            row.x, row.y, row.z,
            row.sx, row.sy, row.sz, row.role,
            row.color_r as f64, row.color_g as f64, row.color_b as f64,
            row.quadrant, row.row, row.col,
            row.content_hash as i64,
        ],
    )?;
    Ok(())
}

/// Set every shelf's colour to green and recompute each shelf's `content_hash`
/// using its actual stored position.
///
/// BUG FIXED: the old implementation passed `(0.0, 0.0, 0.0)` as the position
/// to `fp_shelf`, producing one identical hash for every row.  On the next diff
/// the Rust layer recomputed each shelf's hash from its real (x, y, z) and
/// found a mismatch for every single shelf — marking the entire building as
/// "updated" and triggering a full teardown/recreate in FreeCAD.
///
/// The fix reads (x, y, z) back from the DB for each shelf so the stored hash
/// exactly matches what `diff_all_specs` will recompute.
pub fn make_all_shelves_green(conn: &Connection) -> Result<()> {
    // Read the position of every shelf so we can compute the correct hash.
    let shelves: Vec<(String, f64, f64, f64)> = conn
        .prepare("SELECT label, x, y, z FROM shelves")?
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, f64>(1)?,
                r.get::<_, f64>(2)?,
                r.get::<_, f64>(3)?,
            ))
        })?
        .collect::<Result<_>>()?;

    for (label, x, y, z) in shelves {
        // Hash uses the actual position plus the new green colour (0, 1, 0).
        let hash = fp_shelf(x, y, z, 0.0, 1.0, 0.0) as i64;
        conn.execute(
            "UPDATE shelves
             SET    color_r      = 0.0,
                    color_g      = 1.0,
                    color_b      = 0.0,
                    content_hash = ?1,
                    updated_at   = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE  label        = ?2",
            params![hash, label],
        )?;
    }
    Ok(())
}

pub fn clear_shelf_items(conn: &Connection) -> Result<()> {
    conn.execute("DELETE FROM shelf_items", [])?;
    Ok(())
}
