// /home/emporas/repos/freecad/rust/roy/db/src/queries.rs
//! Read/write helpers for every entity type.
//!
//! # Change-detection contract
//!
//! Every `get_*` function accepts two optional hints:
//!
//! ```ignore
//! known: Option<&HashMap<String, u64>>   // coord/label → content_hash from last call
//! since: Option<&str>                    // ISO-8601 timestamp from last call
//! ```
//!
//! Pass `None` / `None` (or an empty map / `None`) for a full load.
//! On every subsequent call pass back the `new_fingerprints` map and the
//! `sync_at` string that were returned in `DiffResult` — the queries then
//! fetch **only rows that changed since that timestamp**, making a single-item
//! colour update O(1) DB work instead of a full table scan.
//!
//! Returns a [`Delta<T>`] with three buckets: `added`, `updated`, `removed`.

use rusqlite::{params, Connection, Result, Row};
use std::collections::{HashMap, HashSet};

// ── Delta ─────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct Delta<T> {
    pub added:   Vec<T>,
    pub updated: Vec<T>,
    pub removed: Vec<String>,
}

impl<T> Default for Delta<T> {
    fn default() -> Self {
        Self {
            added:   Vec::new(),
            updated: Vec::new(),
            removed: Vec::new(),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// § 1  Row types
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct SlabRow {
    pub label:        String,
    pub store:        String,
    pub x:            f64,
    pub y:            f64,
    pub z:            f64,
    pub length:       f64,
    pub width:        f64,
    pub thickness:    f64,
    pub content_hash: u64,
}

#[derive(Debug, Clone)]
pub struct WallRow {
    pub label:        String,
    pub store:        String,
    pub x1:           f64,
    pub y1:           f64,
    pub x2:           f64,
    pub y2:           f64,
    pub z:            f64,
    pub height:       f64,
    pub width:        f64,
    pub align:        String,
    pub content_hash: u64,
}

#[derive(Debug, Clone)]
pub struct ShelfRow {
    pub label:        String,
    pub store:        String,
    pub x:            f64,
    pub y:            f64,
    pub z:            f64,
    pub sx:           f64,
    pub sy:           f64,
    pub sz:           f64,
    pub role:         String,
    pub color_r:      f32,
    pub color_g:      f32,
    pub color_b:      f32,
    pub quadrant:     String,
    pub row:          u32,
    pub col:          u32,
    pub content_hash: u64,
}

#[derive(Debug, Clone)]
pub struct ItemRow {
    pub coord_label:  String,
    pub shelf_label:  String,
    pub price:        f64,
    pub quantity:     u32,
    pub world_x:      f64,
    pub world_y:      f64,
    pub world_z:      f64,
    pub level:        u32,
    pub color_r:      f32,
    pub color_g:      f32,
    pub color_b:      f32,
    pub content_hash: u64,
}

// ═══════════════════════════════════════════════════════════════════════════════
// § 2  Row mappers
//  slabs:   label(0) store(1) x(2) y(3) z(4) length(5) width(6) thickness(7) hash(8)
//  walls:   label(0) store(1) x1(2) y1(3) x2(4) y2(5) z(6) height(7) width(8) align(9) hash(10)
//  shelves: label(0) store(1) x(2) y(3) z(4) sx(5) sy(6) sz(7) role(8)
//           cr(9) cg(10) cb(11) quadrant(12) row(13) col(14) hash(15)
//  items:   coord(0) shelf(1) price(2) qty(3) wx(4) wy(5) wz(6) level(7)
//           cr(8) cg(9) cb(10) hash(11)
// ═══════════════════════════════════════════════════════════════════════════════

fn map_slab(r: &Row<'_>) -> rusqlite::Result<SlabRow> {
    Ok(SlabRow {
        label:        r.get(0)?,
        store:        r.get(1)?,
        x:            r.get(2)?,
        y:            r.get(3)?,
        z:            r.get(4)?,
        length:       r.get(5)?,
        width:        r.get(6)?,
        thickness:    r.get(7)?,
        content_hash: r.get::<_, i64>(8)? as u64,
    })
}

fn map_wall(r: &Row<'_>) -> rusqlite::Result<WallRow> {
    Ok(WallRow {
        label:        r.get(0)?,
        store:        r.get(1)?,
        x1:           r.get(2)?,
        y1:           r.get(3)?,
        x2:           r.get(4)?,
        y2:           r.get(5)?,
        z:            r.get(6)?,
        height:       r.get(7)?,
        width:        r.get(8)?,
        align:        r.get(9)?,
        content_hash: r.get::<_, i64>(10)? as u64,
    })
}

fn map_shelf(r: &Row<'_>) -> rusqlite::Result<ShelfRow> {
    Ok(ShelfRow {
        label:        r.get(0)?,
        store:        r.get(1)?,
        x:            r.get(2)?,
        y:            r.get(3)?,
        z:            r.get(4)?,
        sx:           r.get(5)?,
        sy:           r.get(6)?,
        sz:           r.get(7)?,
        role:         r.get(8)?,
        color_r:      r.get::<_, f64>(9)?  as f32,
        color_g:      r.get::<_, f64>(10)? as f32,
        color_b:      r.get::<_, f64>(11)? as f32,
        quadrant:     r.get(12)?,
        row:          r.get::<_, u32>(13)?,
        col:          r.get::<_, u32>(14)?,
        content_hash: r.get::<_, i64>(15)? as u64,
    })
}

fn map_item(r: &Row<'_>) -> rusqlite::Result<ItemRow> {
    Ok(ItemRow {
        coord_label:  r.get(0)?,
        shelf_label:  r.get(1)?,
        price:        r.get(2)?,
        quantity:     r.get::<_, u32>(3)?,
        world_x:      r.get(4)?,
        world_y:      r.get(5)?,
        world_z:      r.get(6)?,
        level:        r.get::<_, u32>(7)?,
        color_r:      r.get::<_, f64>(8)?  as f32,
        color_g:      r.get::<_, f64>(9)?  as f32,
        color_b:      r.get::<_, f64>(10)? as f32,
        content_hash: r.get::<_, i64>(11)? as u64,
    })
}

// ═══════════════════════════════════════════════════════════════════════════════
// § 3  Delta builders
//
//  Two strategies depending on whether `since` is provided:
//
//  Full  (since = None)
//    SELECT all rows → in-memory hash diff against `known`.
//    Deletions are rows present in `known` but absent from the full result set.
//
//  Incremental  (since = Some(ts))
//    Step A — cheap key-only scan  → `all_keys`  (no data columns)
//    Step B — WHERE updated_at >= ts  → `changed_rows`  (only mutated rows)
//    Deletions = keys in `known` that are absent from `all_keys`.
//    Changed rows are classified as added / updated by comparing hashes.
//    Unchanged rows (in `known`, in `all_keys`, not in `changed_rows`) are
//    silently skipped — zero work.
// ═══════════════════════════════════════════════════════════════════════════════

/// Full diff: fetch every row and compare against `known`.
/// Used on first load (since = None) or when `known` is empty.
fn full_diff<T, K, H>(
    all_rows: Vec<T>,
    known:    Option<&HashMap<String, u64>>,
    key_of:   K,
    hash_of:  H,
) -> Delta<T>
where
    K: Fn(&T) -> &str,
    H: Fn(&T) -> u64,
{
    let mut delta   = Delta::default();
    let mut db_keys = HashSet::with_capacity(all_rows.len());

    for row in all_rows {
        let key  = key_of(&row).to_owned();
        let hash = hash_of(&row);
        db_keys.insert(key.clone());
        match known.and_then(|m| m.get(&key)) {
            None                      => delta.added.push(row),
            Some(&old) if old != hash => delta.updated.push(row),
            _                         => {}
        }
    }

    if let Some(m) = known {
        for key in m.keys() {
            if !db_keys.contains(key.as_str()) {
                delta.removed.push(key.clone());
            }
        }
    }

    delta
}

/// Incremental diff: only `changed_rows` (updated_at >= since) are inspected.
/// `all_keys` is the result of a prior cheap key-only scan.
fn incremental_diff<T, K, H>(
    all_keys:     HashSet<String>,
    changed_rows: Vec<T>,
    known:        Option<&HashMap<String, u64>>,
    key_of:       K,
    hash_of:      H,
) -> Delta<T>
where
    K: Fn(&T) -> &str,
    H: Fn(&T) -> u64,
{
    let mut delta = Delta::default();

    for row in changed_rows {
        let key  = key_of(&row).to_owned();
        let hash = hash_of(&row);
        match known.and_then(|m| m.get(&key)) {
            None                      => delta.added.push(row),
            Some(&old) if old != hash => delta.updated.push(row),
            _                         => {} // hash unchanged — already applied
        }
    }

    // Deletions: key was in our fingerprint cache but is no longer in the DB.
    if let Some(m) = known {
        for key in m.keys() {
            if !all_keys.contains(key.as_str()) {
                delta.removed.push(key.clone());
            }
        }
    }

    delta
}

// ═══════════════════════════════════════════════════════════════════════════════
// § 4  DB info
// ═══════════════════════════════════════════════════════════════════════════════

pub fn describe_tables(conn: &Connection) -> Result<Vec<(String, Vec<String>)>> {
    let table_names: Vec<String> = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")?
        .query_map([], |r| r.get(0))?
        .collect::<Result<_>>()?;

    table_names
        .into_iter()
        .map(|tbl| {
            let cols: Vec<String> = conn
                .prepare(&format!("PRAGMA table_info('{tbl}')"))?
                .query_map([], |r| r.get::<_, String>(1))?
                .collect::<Result<_>>()?;
            Ok((tbl, cols))
        })
        .collect()
}

// ═══════════════════════════════════════════════════════════════════════════════
// § 5  Get functions  (read + change-detect)
// ═══════════════════════════════════════════════════════════════════════════════

pub fn get_slabs(
    conn:  &Connection,
    known: Option<&HashMap<String, u64>>,
    store: Option<&str>,
    since: Option<&str>,
) -> Result<Delta<SlabRow>> {
    // ── Incremental path ──────────────────────────────────────────────────────
    if let Some(ts) = since {
        // Step A: cheap key-only scan (all existing labels, no data columns).
        let all_keys: HashSet<String> = match store {
            None => conn
                .prepare("SELECT label FROM slabs")?
                .query_map([], |r| r.get(0))?
                .collect::<Result<_>>()?,
            Some(s) => conn
                .prepare("SELECT label FROM slabs WHERE store = ?1")?
                .query_map(params![s], |r| r.get(0))?
                .collect::<Result<_>>()?,
        };

        // Step B: full row fetch only for rows touched since last sync.
        let changed: Vec<SlabRow> = match store {
            None => conn
                .prepare(
                    "SELECT label, store, x, y, z, length, width, thickness, content_hash
                     FROM   slabs
                     WHERE  updated_at >= ?1
                     ORDER  BY store, label",
                )?
                .query_map(params![ts], map_slab)?
                .collect::<Result<_>>()?,
            Some(s) => conn
                .prepare(
                    "SELECT label, store, x, y, z, length, width, thickness, content_hash
                     FROM   slabs
                     WHERE  store = ?1 AND updated_at >= ?2
                     ORDER  BY label",
                )?
                .query_map(params![s, ts], map_slab)?
                .collect::<Result<_>>()?,
        };

        return Ok(incremental_diff(all_keys, changed, known, |r| &r.label, |r| r.content_hash));
    }

    // ── Full path (first load or forced rebuild) ───────────────────────────────
    let all_rows: Vec<SlabRow> = match store {
        None => conn
            .prepare(
                "SELECT label, store, x, y, z, length, width, thickness, content_hash
                 FROM   slabs
                 ORDER  BY store, label",
            )?
            .query_map([], map_slab)?
            .collect::<Result<_>>()?,
        Some(s) => conn
            .prepare(
                "SELECT label, store, x, y, z, length, width, thickness, content_hash
                 FROM   slabs
                 WHERE  store = ?1
                 ORDER  BY label",
            )?
            .query_map(params![s], map_slab)?
            .collect::<Result<_>>()?,
    };

    Ok(full_diff(all_rows, known, |r| &r.label, |r| r.content_hash))
}

pub fn get_walls(
    conn:  &Connection,
    known: Option<&HashMap<String, u64>>,
    store: Option<&str>,
    since: Option<&str>,
) -> Result<Delta<WallRow>> {
    // ── Incremental path ──────────────────────────────────────────────────────
    if let Some(ts) = since {
        let all_keys: HashSet<String> = match store {
            None => conn
                .prepare("SELECT label FROM walls")?
                .query_map([], |r| r.get(0))?
                .collect::<Result<_>>()?,
            Some(s) => conn
                .prepare("SELECT label FROM walls WHERE store = ?1")?
                .query_map(params![s], |r| r.get(0))?
                .collect::<Result<_>>()?,
        };

        let changed: Vec<WallRow> = match store {
            None => conn
                .prepare(
                    "SELECT label, store, x1, y1, x2, y2, z, height, width, align, content_hash
                     FROM   walls
                     WHERE  updated_at >= ?1
                     ORDER  BY store, label",
                )?
                .query_map(params![ts], map_wall)?
                .collect::<Result<_>>()?,
            Some(s) => conn
                .prepare(
                    "SELECT label, store, x1, y1, x2, y2, z, height, width, align, content_hash
                     FROM   walls
                     WHERE  store = ?1 AND updated_at >= ?2
                     ORDER  BY label",
                )?
                .query_map(params![s, ts], map_wall)?
                .collect::<Result<_>>()?,
        };

        return Ok(incremental_diff(all_keys, changed, known, |r| &r.label, |r| r.content_hash));
    }

    // ── Full path ─────────────────────────────────────────────────────────────
    let all_rows: Vec<WallRow> = match store {
        None => conn
            .prepare(
                "SELECT label, store, x1, y1, x2, y2, z, height, width, align, content_hash
                 FROM   walls
                 ORDER  BY store, label",
            )?
            .query_map([], map_wall)?
            .collect::<Result<_>>()?,
        Some(s) => conn
            .prepare(
                "SELECT label, store, x1, y1, x2, y2, z, height, width, align, content_hash
                 FROM   walls
                 WHERE  store = ?1
                 ORDER  BY label",
            )?
            .query_map(params![s], map_wall)?
            .collect::<Result<_>>()?,
    };

    Ok(full_diff(all_rows, known, |r| &r.label, |r| r.content_hash))
}

pub fn get_shelves(
    conn:     &Connection,
    known:    Option<&HashMap<String, u64>>,
    store:    Option<&str>,
    quadrant: Option<&str>,
    since:    Option<&str>,
) -> Result<Delta<ShelfRow>> {
    const COLS: &str =
        "label, store, x, y, z, sx, sy, sz, role,
         color_r, color_g, color_b, quadrant, row, col, content_hash";

    // ── Incremental path ──────────────────────────────────────────────────────
    if let Some(ts) = since {
        let all_keys: HashSet<String> = match (store, quadrant) {
            (None, None) => conn
                .prepare("SELECT label FROM shelves")?
                .query_map([], |r| r.get(0))?
                .collect::<Result<_>>()?,
            (Some(s), None) => conn
                .prepare("SELECT label FROM shelves WHERE store = ?1")?
                .query_map(params![s], |r| r.get(0))?
                .collect::<Result<_>>()?,
            (None, Some(q)) => conn
                .prepare("SELECT label FROM shelves WHERE quadrant = ?1")?
                .query_map(params![q], |r| r.get(0))?
                .collect::<Result<_>>()?,
            (Some(s), Some(q)) => conn
                .prepare("SELECT label FROM shelves WHERE store = ?1 AND quadrant = ?2")?
                .query_map(params![s, q], |r| r.get(0))?
                .collect::<Result<_>>()?,
        };

        let changed: Vec<ShelfRow> = match (store, quadrant) {
            (None, None) => conn
                .prepare(&format!(
                    "SELECT {COLS} FROM shelves
                     WHERE  updated_at >= ?1
                     ORDER  BY store, quadrant, row, col"
                ))?
                .query_map(params![ts], map_shelf)?
                .collect::<Result<_>>()?,
            (Some(s), None) => conn
                .prepare(&format!(
                    "SELECT {COLS} FROM shelves
                     WHERE  store = ?1 AND updated_at >= ?2
                     ORDER  BY quadrant, row, col"
                ))?
                .query_map(params![s, ts], map_shelf)?
                .collect::<Result<_>>()?,
            (None, Some(q)) => conn
                .prepare(&format!(
                    "SELECT {COLS} FROM shelves
                     WHERE  quadrant = ?1 AND updated_at >= ?2
                     ORDER  BY store, row, col"
                ))?
                .query_map(params![q, ts], map_shelf)?
                .collect::<Result<_>>()?,
            (Some(s), Some(q)) => conn
                .prepare(&format!(
                    "SELECT {COLS} FROM shelves
                     WHERE  store = ?1 AND quadrant = ?2 AND updated_at >= ?3
                     ORDER  BY row, col"
                ))?
                .query_map(params![s, q, ts], map_shelf)?
                .collect::<Result<_>>()?,
        };

        return Ok(incremental_diff(all_keys, changed, known, |r| &r.label, |r| r.content_hash));
    }

    // ── Full path ─────────────────────────────────────────────────────────────
    let all_rows: Vec<ShelfRow> = match (store, quadrant) {
        (None, None) => conn
            .prepare(&format!(
                "SELECT {COLS} FROM shelves ORDER BY store, quadrant, row, col"
            ))?
            .query_map([], map_shelf)?
            .collect::<Result<_>>()?,
        (Some(s), None) => conn
            .prepare(&format!(
                "SELECT {COLS} FROM shelves WHERE store = ?1 ORDER BY quadrant, row, col"
            ))?
            .query_map(params![s], map_shelf)?
            .collect::<Result<_>>()?,
        (None, Some(q)) => conn
            .prepare(&format!(
                "SELECT {COLS} FROM shelves WHERE quadrant = ?1 ORDER BY store, row, col"
            ))?
            .query_map(params![q], map_shelf)?
            .collect::<Result<_>>()?,
        (Some(s), Some(q)) => conn
            .prepare(&format!(
                "SELECT {COLS} FROM shelves
                 WHERE  store = ?1 AND quadrant = ?2
                 ORDER  BY row, col"
            ))?
            .query_map(params![s, q], map_shelf)?
            .collect::<Result<_>>()?,
    };

    Ok(full_diff(all_rows, known, |r| &r.label, |r| r.content_hash))
}

pub fn get_items(
    conn:        &Connection,
    known:       Option<&HashMap<String, u64>>,
    shelf_label: Option<&str>,
    since:       Option<&str>,
) -> Result<Delta<ItemRow>> {
    const COLS: &str =
        "coord_label, shelf_label, price, quantity,
         world_x, world_y, world_z, level,
         color_r, color_g, color_b, content_hash";

    // ── Incremental path ──────────────────────────────────────────────────────
    if let Some(ts) = since {
        // Step A: cheap key-only scan — tells us what still exists (for deletion
        // detection) without paying for any data columns.
        let all_keys: HashSet<String> = match shelf_label {
            None => conn
                .prepare("SELECT coord_label FROM shelf_items")?
                .query_map([], |r| r.get(0))?
                .collect::<Result<_>>()?,
            Some(sl) => conn
                .prepare("SELECT coord_label FROM shelf_items WHERE shelf_label = ?1")?
                .query_map(params![sl], |r| r.get(0))?
                .collect::<Result<_>>()?,
        };

        // Step B: full row fetch only for rows actually touched since last sync.
        let changed: Vec<ItemRow> = match shelf_label {
            None => conn
                .prepare(&format!(
                    "SELECT {COLS} FROM shelf_items
                     WHERE  updated_at >= ?1
                     ORDER  BY shelf_label, level"
                ))?
                .query_map(params![ts], map_item)?
                .collect::<Result<_>>()?,
            Some(sl) => conn
                .prepare(&format!(
                    "SELECT {COLS} FROM shelf_items
                     WHERE  shelf_label = ?1 AND updated_at >= ?2
                     ORDER  BY level"
                ))?
                .query_map(params![sl, ts], map_item)?
                .collect::<Result<_>>()?,
        };

        return Ok(incremental_diff(
            all_keys, changed, known,
            |r| &r.coord_label, |r| r.content_hash,
        ));
    }

    // ── Full path (first load or forced rebuild) ───────────────────────────────
    let all_rows: Vec<ItemRow> = match shelf_label {
        None => conn
            .prepare(&format!(
                "SELECT {COLS} FROM shelf_items ORDER BY shelf_label, level"
            ))?
            .query_map([], map_item)?
            .collect::<Result<_>>()?,
        Some(sl) => conn
            .prepare(&format!(
                "SELECT {COLS} FROM shelf_items WHERE shelf_label = ?1 ORDER BY level"
            ))?
            .query_map(params![sl], map_item)?
            .collect::<Result<_>>()?,
    };

    Ok(full_diff(all_rows, known, |r| &r.coord_label, |r| r.content_hash))
}

// ═══════════════════════════════════════════════════════════════════════════════
// § 6  Upsert functions
// ═══════════════════════════════════════════════════════════════════════════════

pub fn upsert_slab(conn: &Connection, row: &SlabRow) -> Result<()> {
    conn.execute(
        "INSERT INTO slabs (label, store, x, y, z, length, width, thickness, content_hash)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
         ON CONFLICT (label) DO UPDATE SET
             store        = excluded.store,
             x            = excluded.x,
             y            = excluded.y,
             z            = excluded.z,
             length       = excluded.length,
             width        = excluded.width,
             thickness    = excluded.thickness,
             content_hash = excluded.content_hash,
             updated_at   = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
        params![
            row.label, row.store,
            row.x, row.y, row.z,
            row.length, row.width, row.thickness,
            row.content_hash as i64,
        ],
    )?;
    Ok(())
}

pub fn upsert_wall(conn: &Connection, row: &WallRow) -> Result<()> {
    conn.execute(
        "INSERT INTO walls (label, store, x1, y1, x2, y2, z, height, width, align, content_hash)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
         ON CONFLICT (label) DO UPDATE SET
             store        = excluded.store,
             x1           = excluded.x1,
             y1           = excluded.y1,
             x2           = excluded.x2,
             y2           = excluded.y2,
             z            = excluded.z,
             height       = excluded.height,
             width        = excluded.width,
             align        = excluded.align,
             content_hash = excluded.content_hash,
             updated_at   = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
        params![
            row.label, row.store,
            row.x1, row.y1, row.x2, row.y2,
            row.z, row.height, row.width, row.align,
            row.content_hash as i64,
        ],
    )?;
    Ok(())
}

pub fn upsert_shelf(conn: &Connection, row: &ShelfRow) -> Result<()> {
    conn.execute(
        "INSERT INTO shelves
             (label, store, x, y, z, sx, sy, sz, role,
              color_r, color_g, color_b, quadrant, row, col, content_hash)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)
         ON CONFLICT (label) DO UPDATE SET
             store        = excluded.store,
             x            = excluded.x,
             y            = excluded.y,
             z            = excluded.z,
             sx           = excluded.sx,
             sy           = excluded.sy,
             sz           = excluded.sz,
             role         = excluded.role,
             color_r      = excluded.color_r,
             color_g      = excluded.color_g,
             color_b      = excluded.color_b,
             quadrant     = excluded.quadrant,
             row          = excluded.row,
             col          = excluded.col,
             content_hash = excluded.content_hash,
             updated_at   = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
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

pub fn upsert_item(conn: &Connection, row: &ItemRow) -> Result<()> {
    conn.execute(
        "INSERT INTO shelf_items
             (coord_label, shelf_label, price, quantity,
              world_x, world_y, world_z, level,
              color_r, color_g, color_b, content_hash)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)
         ON CONFLICT (coord_label) DO UPDATE SET
             shelf_label  = excluded.shelf_label,
             price        = excluded.price,
             quantity     = excluded.quantity,
             world_x      = excluded.world_x,
             world_y      = excluded.world_y,
             world_z      = excluded.world_z,
             level        = excluded.level,
             color_r      = excluded.color_r,
             color_g      = excluded.color_g,
             color_b      = excluded.color_b,
             content_hash = excluded.content_hash,
             updated_at   = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
        params![
            row.coord_label, row.shelf_label,
            row.price, row.quantity,
            row.world_x, row.world_y, row.world_z,
            row.level,
            row.color_r as f64, row.color_g as f64, row.color_b as f64,
            row.content_hash as i64,
        ],
    )?;
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// § 7  Targeted colour update
// ═══════════════════════════════════════════════════════════════════════════════

/// Change only the highlight colour of a single inventory slot.
///
/// Reads back `price`, `quantity`, and `level` to recompute the content hash,
/// ensuring the diff engine detects the change on the next `build_roy()`.
///
/// Returns `Ok(true)` if the slot was found and updated, `Ok(false)` if the
/// `coord_label` does not exist in the database.
pub fn update_item_color(
    conn:        &Connection,
    coord_label: &str,
    cr: f32, cg: f32, cb: f32,
) -> Result<bool> {
    let result = conn.query_row(
        "SELECT price, quantity, level FROM shelf_items WHERE coord_label = ?1",
        params![coord_label],
        |r| Ok((r.get::<_, f64>(0)?, r.get::<_, u32>(1)?, r.get::<_, u32>(2)?)),
    );

    match result {
        Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(false),
        Err(e) => return Err(e),
        Ok((price, quantity, level)) => {
            let hash = fp_item(price, quantity, level, cr, cg, cb);
            conn.execute(
                "UPDATE shelf_items
                 SET    color_r      = ?1,
                        color_g      = ?2,
                        color_b      = ?3,
                        content_hash = ?4,
                        updated_at   = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                 WHERE  coord_label  = ?5",
                params![cr as f64, cg as f64, cb as f64, hash as i64, coord_label],
            )?;
        }
    }
    Ok(true)
}

pub fn delete_slab(conn: &Connection, label: &str) -> Result<()> {
    conn.execute("DELETE FROM slabs       WHERE label       = ?1", params![label])?;
    Ok(())
}
pub fn delete_wall(conn: &Connection, label: &str) -> Result<()> {
    conn.execute("DELETE FROM walls       WHERE label       = ?1", params![label])?;
    Ok(())
}
pub fn delete_shelf(conn: &Connection, label: &str) -> Result<()> {
    conn.execute("DELETE FROM shelves     WHERE label       = ?1", params![label])?;
    Ok(())
}
pub fn delete_item(conn: &Connection, coord: &str) -> Result<()> {
    conn.execute("DELETE FROM shelf_items WHERE coord_label = ?1", params![coord])?;
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// § 8  Content-hash helpers
// ═══════════════════════════════════════════════════════════════════════════════

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

fn make_hasher() -> DefaultHasher { DefaultHasher::new() }

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

pub fn fp_slab(x: f64, y: f64, z: f64, length: f64, width: f64, thickness: f64) -> u64 {
    let mut h = make_hasher();
    x.to_bits().hash(&mut h);
    y.to_bits().hash(&mut h);
    z.to_bits().hash(&mut h);
    length.to_bits().hash(&mut h);
    width.to_bits().hash(&mut h);
    thickness.to_bits().hash(&mut h);
    h.finish()
}

pub fn fp_wall(
    x1: f64, y1: f64, x2: f64, y2: f64,
    z: f64, height: f64, width: f64, align: &str,
) -> u64 {
    let mut h = make_hasher();
    x1.to_bits().hash(&mut h);
    y1.to_bits().hash(&mut h);
    x2.to_bits().hash(&mut h);
    y2.to_bits().hash(&mut h);
    z.to_bits().hash(&mut h);
    height.to_bits().hash(&mut h);
    width.to_bits().hash(&mut h);
    align.hash(&mut h);
    h.finish()
}

// ═══════════════════════════════════════════════════════════════════════════════
// § 9  Colour helpers
// ═══════════════════════════════════════════════════════════════════════════════

pub fn color_from_flag(red: bool, blue: bool, yellow: bool, green: bool) -> (f32, f32, f32) {
    if red    { return (1.0, 0.0, 0.0); }
    if blue   { return (0.0, 0.0, 1.0); }
    if yellow { return (1.0, 1.0, 0.0); }
    if green  { return (0.0, 1.0, 0.0); }
    (1.0, 1.0, 1.0)
}

// ═══════════════════════════════════════════════════════════════════════════════
// § 10  Store-level write helpers
// ═══════════════════════════════════════════════════════════════════════════════

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
/// BUG FIXED: the old implementation used `fp_shelf(0.0, 0.0, 0.0, ...)` for
/// every row, producing a single identical hash regardless of position.  On the
/// next diff the Rust layer recomputed the correct per-shelf hash, found a
/// mismatch for every row, and marked the entire building as "updated" —
/// causing FreeCAD to tear down and recreate every object.
///
/// The fix reads (x, y, z) back from the DB so each shelf gets the hash that
/// `diff_all_specs` will recompute from the same stored values.
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
