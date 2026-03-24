// /home/emporas/repos/freecad/rust/roy/db/src/schema.rs
/// All DDL in one shot — safe to run on every open() call because every
/// statement uses IF NOT EXISTS.  WAL + FK enforcement are connection-local
/// PRAGMAs and must be re-issued on every connection.
pub const DDL: &str = "
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;

-- ── Structural slabs (foundation / inter-floor / roof) ──────────────────────
CREATE TABLE IF NOT EXISTS slabs (
    label        TEXT    PRIMARY KEY,
    store        TEXT    NOT NULL
                         CHECK (store IN ('Kifisos','Piraios','Intersport')),
    x            REAL    NOT NULL,
    y            REAL    NOT NULL,
    z            REAL    NOT NULL,
    length       REAL    NOT NULL,
    width        REAL    NOT NULL,
    thickness    REAL    NOT NULL,
    content_hash INTEGER NOT NULL,
    updated_at   TEXT    NOT NULL
                         DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE INDEX IF NOT EXISTS idx_slabs_store ON slabs (store);

-- ── BIM walls (centre-line driven) ──────────────────────────────────────────
CREATE TABLE IF NOT EXISTS walls (
    label        TEXT    PRIMARY KEY,
    store        TEXT    NOT NULL
                         CHECK (store IN ('Kifisos','Piraios','Intersport')),
    x1           REAL    NOT NULL,
    y1           REAL    NOT NULL,
    x2           REAL    NOT NULL,
    y2           REAL    NOT NULL,
    z            REAL    NOT NULL,
    height       REAL    NOT NULL,
    width        REAL    NOT NULL,
    align        TEXT    NOT NULL CHECK (align IN ('Left','Right','Center')),
    content_hash INTEGER NOT NULL,
    updated_at   TEXT    NOT NULL
                         DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE INDEX IF NOT EXISTS idx_walls_store ON walls (store);

-- ── Shelf gondola units ──────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS shelves (
    label        TEXT    PRIMARY KEY,
    store        TEXT    NOT NULL
                         CHECK (store IN ('Kifisos','Piraios','Intersport')),
    x            REAL    NOT NULL,
    y            REAL    NOT NULL,
    z            REAL    NOT NULL,
    sx           REAL    NOT NULL,   -- size X
    sy           REAL    NOT NULL,   -- size Y (depth)
    sz           REAL    NOT NULL,   -- size Z (height)
    role         TEXT    NOT NULL CHECK (role IN ('Fill','Refill','Wall')),
    color_r      REAL    NOT NULL,
    color_g      REAL    NOT NULL,
    color_b      REAL    NOT NULL,
    quadrant     TEXT    NOT NULL,
    row          INTEGER NOT NULL,
    col          INTEGER NOT NULL,
    content_hash INTEGER NOT NULL,
    updated_at   TEXT    NOT NULL
                         DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE INDEX IF NOT EXISTS idx_shelves_addr
    ON shelves (store, quadrant, row, col);
CREATE INDEX IF NOT EXISTS idx_shelves_store ON shelves (store);

-- ── Inventory slots (one level per shelf unit) ───────────────────────────────
-- Store membership is inherited transitively through the shelf FK.
-- color_r/g/b default to white (1.0) — overridden per-slot via the CLI.
CREATE TABLE IF NOT EXISTS shelf_items (
    coord_label  TEXT    PRIMARY KEY,   -- e.g. \"Kifisos.Kip.2.3.1\"
    shelf_label  TEXT    NOT NULL
                         REFERENCES shelves (label) ON DELETE CASCADE,
    price        REAL    NOT NULL CHECK (price >= 0),
    quantity     INTEGER NOT NULL CHECK (quantity >= 0),
    world_x      REAL    NOT NULL,
    world_y      REAL    NOT NULL,
    world_z      REAL    NOT NULL,
    level        INTEGER NOT NULL CHECK (level >= 1),
    color_r      REAL    NOT NULL DEFAULT 1.0,
    color_g      REAL    NOT NULL DEFAULT 1.0,
    color_b      REAL    NOT NULL DEFAULT 1.0,
    content_hash INTEGER NOT NULL,
    updated_at   TEXT    NOT NULL
                         DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE INDEX IF NOT EXISTS idx_items_shelf
    ON shelf_items (shelf_label);
";

/// Idempotent migrations for databases that pre-date the current schema.
/// Each statement is executed individually; a \"duplicate column\" error is
/// silently ignored so this is safe to run on every open() call.
pub const MIGRATIONS: &[&str] = &[
    "ALTER TABLE shelf_items ADD COLUMN color_r REAL NOT NULL DEFAULT 1.0",
    "ALTER TABLE shelf_items ADD COLUMN color_g REAL NOT NULL DEFAULT 1.0",
    "ALTER TABLE shelf_items ADD COLUMN color_b REAL NOT NULL DEFAULT 1.0",
];
