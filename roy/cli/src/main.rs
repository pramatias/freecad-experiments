// /home/emporas/repos/freecad/rust/roy/cli/src/main.rs
mod init;

use anyhow::{anyhow, Context, Result};
use clap::{ArgAction, Args, Parser, Subcommand};
use db::queries::{
    clear_shelf_items, color_from_flag, ensure_shelf, fp_item, fp_shelf,
    fp_slab, fp_wall,
    make_all_shelves_green, update_item_color, upsert_item, upsert_slab, upsert_wall,
    ItemRow, ShelfRow, SlabRow, WallRow,
};
use directories::ProjectDirs;
use init::init::initialize_logger;
use log::{info, warn, LevelFilter};
use rand::Rng;
use std::path::PathBuf;

use primitives::{make_all_shelves_for_store, make_slabs, make_walls};
use init::init::{initialize_database, delete_database};

// ── Verbosity ─────────────────────────────────────────────────────────────────

#[derive(Args, Debug)]
pub struct Verbosity {
    #[arg(short = 'v', long, action = ArgAction::Count, display_order = 99)]
    pub verbose: u8,
    #[arg(short = 'q', long, action = ArgAction::Count, display_order = 100)]
    pub quiet: u8,
}

impl Verbosity {
    pub fn log_level_filter(&self) -> LevelFilter {
        if self.quiet > 0 {
            LevelFilter::Warn
        } else {
            match self.verbose {
                0 => LevelFilter::Info,
                1 => LevelFilter::Debug,
                _ => LevelFilter::Trace,
            }
        }
    }
}

// ── Top-level CLI ─────────────────────────────────────────────────────────────

#[derive(Parser, Debug)]
#[command(author = "emporas", version = "0.1", about = "Roy shelf-inventory CLI")]
struct Cli {
    #[command(flatten)]
    verbose: Verbosity,
    #[command(subcommand)]
    command: Mode,
}

// ── Subcommands ───────────────────────────────────────────────────────────────

#[derive(Subcommand, Debug)]
enum Mode {
    /// Insert / update a shelf item in the Kifisos store.
    Kifisos(StoreArgs),
    /// Insert / update a shelf item in the Piraios store.
    Piraios(StoreArgs),
    /// Insert / update a shelf item in the Intersport store.
    Intersport(StoreArgs),
    /// Create (or re-initialise) the database with all three buildings.
    Create(DbArgs),
    /// Delete the database file from disk.
    Delete(DbArgs),
    /// Populate the database with random shelves and items.
    Randomize(DbArgs),
    /// Set every shelf's colour to green.
    Clear(DbArgs),
}

// ── Args structs ──────────────────────────────────────────────────────────────

#[derive(Args, Debug)]
pub struct StoreArgs {
    /// Shelf-slot coordinate: QUAD.ROW.COL.LEVEL
    ///
    /// QUAD is the two-character quadrant prefix for the sub-store on this
    /// floor: Kip | Ydr | Ele | Toi.
    ///
    /// Examples:
    ///   Kip.2.3.1   — quadrant Kip, row 2, column 3, level 1
    ///   Toi.5.12.4  — quadrant Toi, row 5, column 12, level 4
    #[arg(
        long = "item",
        value_name = "QUAD.ROW.COL.LEVEL",
        help = "Slot coord, e.g. Kip.2.3.1  (quadrant · row · col · level)"
    )]
    pub item: String,

    /// Set the slot highlight to red.
    #[arg(short = 'r', long = "red")]
    pub red: bool,
    /// Set the slot highlight to blue.
    #[arg(short = 'b', long = "blue")]
    pub blue: bool,
    /// Set the slot highlight to yellow.
    #[arg(short = 'y', long = "yellow")]
    pub yellow: bool,
    /// Set the slot highlight to green.
    #[arg(short = 'g', long = "green")]
    pub green: bool,
    /// Reset the slot highlight to white (no highlight).
    #[arg(short = 'w', long = "white")]
    pub white: bool,

    #[arg(long = "price")]
    pub price: Option<f64>,
    #[arg(long = "quantity")]
    pub quantity: Option<u32>,
    #[arg(long = "db")]
    pub db: Option<String>,
}

#[derive(Args, Debug)]
pub struct DbArgs {
    #[arg(long = "db")]
    pub db: Option<String>,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn resolve_db_path(flag: Option<&str>) -> PathBuf {
    if let Some(p) = flag { return PathBuf::from(p); }
    ProjectDirs::from("com", "example", "roy")
        .map(|pd| pd.data_dir().join("roy.db"))
        .unwrap_or_else(|| PathBuf::from("roy.db"))
}

/// Valid quadrant prefixes (3-character strings, matching `Quad::prefix()`).
const VALID_QUADS: &[&str] = &["Kip", "Ydr", "Ele", "Toi"];

/// Parse a `QUAD.ROW.COL.LEVEL` coordinate string.
///
/// Returns `(quadrant, row, col, level)`.
fn parse_coord(s: &str) -> Result<(String, u32, u32, u32)> {
    let parts: Vec<&str> = s.splitn(4, '.').collect();
    if parts.len() != 4 {
        return Err(anyhow!(
            "Coord must be QUAD.ROW.COL.LEVEL (e.g. \"Kip.2.3.1\"), got: {s}"
        ));
    }
    let quad = parts[0].to_string();
    if !VALID_QUADS.contains(&quad.as_str()) {
        return Err(anyhow!(
            "Unknown quadrant \"{quad}\" — expected one of: {}",
            VALID_QUADS.join(", ")
        ));
    }
    let row   = parts[1].parse::<u32>().map_err(|e| anyhow!("Invalid row in \"{s}\": {e}"))?;
    let col   = parts[2].parse::<u32>().map_err(|e| anyhow!("Invalid col in \"{s}\": {e}"))?;
    let level = parts[3].parse::<u32>().map_err(|e| anyhow!("Invalid level in \"{s}\": {e}"))?;
    if row == 0   { return Err(anyhow!("row must be ≥ 1 (got 0)")); }
    if col == 0   { return Err(anyhow!("col must be ≥ 1 (got 0)")); }
    if level == 0 { return Err(anyhow!("level must be ≥ 1 (got 0)")); }
    Ok((quad, row, col, level))
}

const SHELF_SPACING_X: f64 = 1.2;
const SHELF_SPACING_Y: f64 = 0.8;
const LEVEL_HEIGHT:    f64 = 0.4;

// ── Run functions ─────────────────────────────────────────────────────────────

fn run_store(store_name: &str, args: &StoreArgs) -> Result<()> {
    // ── Colour flag handling ──────────────────────────────────────────────────
    let colour_flags = [args.red, args.blue, args.yellow, args.green, args.white];
    let colour_count = colour_flags.iter().filter(|&&b| b).count();
    if colour_count > 1 {
        warn!("Multiple colour flags set — only the first matching flag will be used.");
    }
    let wants_colour = colour_count > 0;

    // ── Parse coord ───────────────────────────────────────────────────────────
    let (quad, row, col, level) = parse_coord(&args.item)?;

    // Full coord_label matches the format written by QuadBuilder:
    //   "{store}.{quad}.{row}.{col}.{level}"
    let coord_label = format!("{store_name}.{quad}.{row}.{col}.{level}");
    // Shelf label also matches QuadBuilder:
    //   "{store}.{quad}_R{row:02}_C{col:02}"
    let shelf_label = format!("{store_name}.{quad}_R{row:02}_C{col:02}");

    let db_path = resolve_db_path(args.db.as_deref());
    let conn    = db::open(&db_path)
        .with_context(|| format!("Cannot open database at {db_path:?}"))?;

    // ── Path A: colour-only update ────────────────────────────────────────────
    // When any colour flag is given and neither --price nor --quantity is set,
    // we do a targeted single-column UPDATE so the shelf/item position data is
    // left untouched.
    if wants_colour && args.price.is_none() && args.quantity.is_none() {
        let (cr, cg, cb) = if args.white {
            (1.0f32, 1.0, 1.0)
        } else {
            color_from_flag(args.red, args.blue, args.yellow, args.green)
        };

        let found = update_item_color(&conn, &coord_label, cr, cg, cb)
            .with_context(|| format!("Failed to update colour for {coord_label}"))?;

        if found {
            info!(
                "[{store_name}] {coord_label}  colour → ({cr:.2}, {cg:.2}, {cb:.2})"
            );
        } else {
            warn!(
                "[{store_name}] {coord_label} not found in database — \
                 run `create` first or omit colour flags to upsert a new slot."
            );
        }
        return Ok(());
    }

    // ── Path B: full upsert (price / quantity / colour together) ─────────────
    // Used when price or quantity is explicitly provided, optionally with colour.
    let (cr, cg, cb) = if args.white {
        (1.0f32, 1.0, 1.0)
    } else {
        color_from_flag(args.red, args.blue, args.yellow, args.green)
    };

    let world_x = col   as f64 * SHELF_SPACING_X;
    let world_y = row   as f64 * SHELF_SPACING_Y;
    let world_z = level as f64 * LEVEL_HEIGHT;

    let shelf = ShelfRow {
        label:        shelf_label.clone(),
        store:        store_name.into(),
        x:            world_x,
        y:            world_y,
        z:            0.0,
        sx:           0.9,
        sy:           0.4,
        sz:           2.0,
        role:         "Fill".into(),
        color_r:      cr,
        color_g:      cg,
        color_b:      cb,
        quadrant:     quad.clone(),
        row,
        col,
        content_hash: fp_shelf(world_x, world_y, 0.0, cr, cg, cb),
    };

    let price    = args.price.unwrap_or(0.0);
    let quantity = args.quantity.unwrap_or(0);

    let item = ItemRow {
        coord_label:  coord_label.clone(),
        shelf_label:  shelf_label.clone(),
        price,
        quantity,
        world_x,
        world_y,
        world_z,
        level,
        color_r:      cr,
        color_g:      cg,
        color_b:      cb,
        content_hash: fp_item(price, quantity, level, cr, cg, cb),
    };

    ensure_shelf(&conn, &shelf)
        .with_context(|| format!("Failed to ensure shelf {shelf_label}"))?;
    upsert_item(&conn, &item)
        .with_context(|| format!("Failed to upsert item {coord_label}"))?;

    info!(
        "[{store_name}] upserted {coord_label}  \
         price={price:.2}  qty={quantity}  colour=({cr:.2},{cg:.2},{cb:.2})"
    );
    Ok(())
}

fn run_create(args: &DbArgs) -> Result<()> {
    let (conn, db_path) = initialize_database(args.db.as_deref())?;

    let p = primitives::RoyParams::new(
        100_000.0,
         10_300.0,
            300.0,
            400.0,
          4_000.0,
            600.0,
          4_000.0,
              4,
              4,
           true,
    );

    const GAP: f64 = 100_000.0;
    let stores: &[(&str, f64, f64)] = &[
        ("Kifisos",    0.0,                     0.0),
        // ("Piraios",    p.side + GAP,             0.0),
        // ("Intersport", 2.0 * (p.side + GAP),    0.0),
    ];

    let mut slab_count  = 0u32;
    let mut wall_count  = 0u32;
    let mut shelf_count = 0u32;
    let mut item_count  = 0u32;

    let mut all_items: Vec<primitives::ShelfItemSpec> = Vec::new();

    for &(store, ox, oy) in stores {
        for s in make_slabs(&p, store, ox, oy) {
            let row = SlabRow {
                content_hash: fp_slab(s.x, s.y, s.z, s.length, s.width, s.thickness),
                label: s.label, store: s.store,
                x: s.x, y: s.y, z: s.z,
                length: s.length, width: s.width, thickness: s.thickness,
            };
            upsert_slab(&conn, &row)
                .with_context(|| format!("upsert_slab failed for {}", row.label))?;
            slab_count += 1;
        }

        for w in make_walls(&p, store, ox, oy) {
            let row = WallRow {
                content_hash: fp_wall(w.x1, w.y1, w.x2, w.y2, w.z, w.height, w.width, &w.align),
                label: w.label, store: w.store,
                x1: w.x1, y1: w.y1, x2: w.x2, y2: w.y2,
                z: w.z, height: w.height, width: w.width, align: w.align,
            };
            upsert_wall(&conn, &row)
                .with_context(|| format!("upsert_wall failed for {}", row.label))?;
            wall_count += 1;
        }

        let (shelves, items) = make_all_shelves_for_store(&p, store, ox, oy);

        const GREEN: (f32, f32, f32) = (0.0, 1.0, 0.0);
        for s in shelves {
            let (cr, cg, cb) = GREEN;
            let row = ShelfRow {
                content_hash: fp_shelf(s.x, s.y, s.z, cr, cg, cb),
                label:    s.label, store: s.store,
                x: s.x,  y: s.y,  z: s.z,
                sx: s.sx, sy: s.sy, sz: s.sz,
                role:     s.role,
                color_r:  cr, color_g: cg, color_b: cb,
                quadrant: s.quadrant,
                row:      s.row, col: s.col,
            };
            db::queries::upsert_shelf(&conn, &row)
                .with_context(|| format!("upsert_shelf failed for {}", row.label))?;
            shelf_count += 1;
        }

        all_items.extend(items);
    }

    // Items inserted after all shelves exist (FK constraint).
    for item in all_items {
        // Newly created items are white (no highlight).
        let (cr, cg, cb) = item.color;
        let row = ItemRow {
            content_hash: fp_item(item.price, item.quantity, item.level, cr, cg, cb),
            coord_label:  item.coord_label,
            shelf_label:  item.shelf_label,
            price:        item.price,
            quantity:     item.quantity,
            world_x:      item.world_x,
            world_y:      item.world_y,
            world_z:      item.world_z,
            level:        item.level,
            color_r:      cr,
            color_g:      cg,
            color_b:      cb,
        };
        upsert_item(&conn, &row)
            .with_context(|| format!("upsert_item failed for {}", row.coord_label))?;
        item_count += 1;
    }

    info!(
        "Database ready at {db_path:?} — \
         {slab_count} slabs  {wall_count} walls  \
         {shelf_count} shelves  {item_count} items  \
         (3 stores: Kifisos / Piraios / Intersport)."
    );
    Ok(())
}

fn run_delete(args: &DbArgs) -> Result<()> {
    delete_database(args.db.as_deref())?;
    Ok(())
}

fn run_randomize(args: &DbArgs) -> Result<()> {
    let db_path = resolve_db_path(args.db.as_deref());
    let conn    = db::open(&db_path)
        .with_context(|| format!("Cannot open database at {db_path:?}"))?;

    let mut rng = rand::thread_rng();
    let stores  = ["Kifisos", "Piraios", "Intersport"];
    const ROWS:   u32 = 4;
    const COLS:   u32 = 6;
    const LEVELS: u32 = 3;

    let mut shelf_count = 0u32;
    let mut item_count  = 0u32;

    for store in &stores {
        for row in 1..=ROWS {
            for col in 1..=COLS {
                let cr: f32 = rng.gen_range(0.0..=1.0);
                let cg: f32 = rng.gen_range(0.0..=1.0);
                let cb: f32 = rng.gen_range(0.0..=1.0);

                let world_x     = col as f64 * SHELF_SPACING_X;
                let world_y     = row as f64 * SHELF_SPACING_Y;
                let shelf_label = format!("{store}.{row}.{col}");

                let shelf = ShelfRow {
                    label:        shelf_label.clone(),
                    store:        store.to_string(),
                    x:            world_x, y: world_y, z: 0.0,
                    sx:           0.9,     sy: 0.4,    sz: 2.0,
                    role:         "Fill".into(),
                    color_r:      cr, color_g: cg, color_b: cb,
                    quadrant:     store.to_string(),
                    row, col,
                    content_hash: fp_shelf(world_x, world_y, 0.0, cr, cg, cb),
                };

                db::queries::upsert_shelf(&conn, &shelf)
                    .with_context(|| format!("upsert_shelf failed for {shelf_label}"))?;
                shelf_count += 1;

                for level in 1..=LEVELS {
                    let price:    f64 = rng.gen_range(0.5..200.0);
                    let quantity: u32 = rng.gen_range(0..=50);
                    let coord_label   = format!("{store}.{row}.{col}.{level}");

                    // Randomised items start white; use the CLI to highlight them.
                    let (ir, ig, ib) = (1.0f32, 1.0, 1.0);

                    let item = ItemRow {
                        coord_label:  coord_label.clone(),
                        shelf_label:  shelf_label.clone(),
                        price, quantity,
                        world_x, world_y,
                        world_z: level as f64 * LEVEL_HEIGHT,
                        level,
                        color_r: ir, color_g: ig, color_b: ib,
                        content_hash: fp_item(price, quantity, level, ir, ig, ib),
                    };

                    upsert_item(&conn, &item)
                        .with_context(|| format!("upsert_item failed for {coord_label}"))?;
                    item_count += 1;
                }
            }
        }
    }

    info!(
        "Randomized: inserted/updated {shelf_count} shelves and {item_count} items \
         across {} stores.",
        stores.len()
    );
    Ok(())
}

fn run_clear(args: &DbArgs) -> Result<()> {
    let db_path = resolve_db_path(args.db.as_deref());
    let conn    = db::open(&db_path)
        .with_context(|| format!("Cannot open database at {db_path:?}"))?;
    make_all_shelves_green(&conn).context("Failed to set shelves green")?;
    info!("All shelf colours set to green (all stores).");
    Ok(())
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() -> Result<()> {
    let cli = Cli::parse();

    initialize_logger(cli.verbose.log_level_filter())
        .context("Failed to initialize logger")?;

    let result = match &cli.command {
        Mode::Kifisos(args)    => run_store("Kifisos",    args),
        Mode::Piraios(args)    => run_store("Piraios",    args),
        Mode::Intersport(args) => run_store("Intersport", args),
        Mode::Create(args)     => run_create(args),
        Mode::Delete(args)     => run_delete(args),
        Mode::Randomize(args)  => run_randomize(args),
        Mode::Clear(args)      => run_clear(args),
    };

    if let Err(e) = result {
        eprintln!("Error: {e:#}");
        std::process::exit(1);
    }

    Ok(())
}
