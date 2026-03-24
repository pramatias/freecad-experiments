// /home/emporas/repos/freecad/rust/roy/cli/src/init/init.rs
use anyhow::{Context, Result};
use directories::ProjectDirs;
use env_logger::Builder;
use log::LevelFilter;
use std::fs::{create_dir_all, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::sync::Mutex;

// use db::schema::DDL;

/// Initialise the global logger, writing to
/// `$XDG_DATA_HOME/roy/logs/app.log` (with a 5 MiB truncation guard).
pub fn initialize_logger(log_level: LevelFilter) -> Result<()> {
    const SIZE_LIMIT: u64 = 5 * 1024 * 1024;

    let log_dir: PathBuf = ProjectDirs::from("com", "example", "roy")
        .map(|pd| pd.data_dir().join("logs"))
        .unwrap_or_else(|| PathBuf::from("logs"));

    create_dir_all(&log_dir).context("Failed to create log directory")?;

    let log_file_path = log_dir.join("app.log");

    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .read(true)
        .open(&log_file_path)
        .with_context(|| format!("Failed to open log file {:?}", log_file_path))?;

    let file = Mutex::new(file);
    let log_file_path_for_closure = log_file_path.clone();

    let mut builder = Builder::new();
    builder.filter(None, log_level);

    builder.format(move |buf, record| {
        use console::style;

        let ts    = buf.timestamp();
        let level = record.level();
        let msg   = record.args();

        let color = match level {
            log::Level::Error => console::Color::Red,
            log::Level::Warn  => console::Color::Yellow,
            log::Level::Info  => console::Color::Green,
            log::Level::Debug => console::Color::Blue,
            log::Level::Trace => console::Color::Cyan,
        };
        let styled_level = style(level).fg(color);

        writeln!(buf, "[{:<5}] {} - {}", styled_level, ts, msg)?;

        let log_entry = format!("{} - {} - {}\n", ts, level, msg);

        if let Ok(mut f) = file.lock() {
            match f.metadata() {
                Ok(meta) if meta.len() >= SIZE_LIMIT => {
                    if let Err(e) = f.set_len(0) {
                        eprintln!("Failed to truncate log file {:?}: {:?}", log_file_path_for_closure, e);
                    } else if let Err(e) = f.seek(SeekFrom::Start(0)) {
                        eprintln!("Failed to seek log file {:?}: {:?}", log_file_path_for_closure, e);
                    } else {
                        let _ = f.write_all(b"--- Log truncated due to size limit ---\n");
                    }
                }
                Err(e) => {
                    eprintln!("Failed to stat log file {:?}: {:?}", log_file_path_for_closure, e);
                }
                _ => {}
            }

            if let Err(e) = f.write_all(log_entry.as_bytes()) {
                eprintln!("Failed to write to log file {:?}: {:?}", log_file_path_for_closure, e);
            }
        } else {
            eprintln!("Failed to acquire lock for log file {:?}", log_file_path_for_closure);
        }

        Ok(())
    });

    builder
        .try_init()
        .context("Failed to initialize global logger")?;

    Ok(())
}

/// Resolve the canonical XDG database path for roy.
///
/// `$XDG_DATA_HOME/roy/roy.db`  (Linux default: `~/.local/share/roy/roy.db`)
/// Falls back to `./roy.db` if XDG directories are unavailable.
pub fn xdg_db_path() -> PathBuf {
    ProjectDirs::from("com", "example", "roy")
        .map(|pd| pd.data_dir().join("roy.db"))
        .unwrap_or_else(|| PathBuf::from("roy.db"))
}

/// Create the XDG data directory and open (or re-initialise) the SQLite
/// database, running all DDL statements via `db::open`.
///
/// Returns the `Connection` so the caller can continue populating tables,
/// and the resolved path for log messages.
pub fn initialize_database(override_path: Option<&str>) -> Result<(rusqlite::Connection, PathBuf)> {
    let db_path = override_path
        .map(PathBuf::from)
        .unwrap_or_else(xdg_db_path);

    // Ensure the parent directory exists (e.g. ~/.local/share/roy/).
    if let Some(parent) = db_path.parent() {
        create_dir_all(parent)
            .with_context(|| format!("Failed to create DB directory {:?}", parent))?;
    }

    let conn = db::open(&db_path)
        .with_context(|| format!("Failed to open/create database at {:?}", db_path))?;

    Ok((conn, db_path))
}

/// Remove the database file from disk.
/// Returns `Ok(false)` (with a warning) if the file doesn't exist.
pub fn delete_database(override_path: Option<&str>) -> Result<bool> {
    let db_path = override_path
        .map(PathBuf::from)
        .unwrap_or_else(xdg_db_path);

    if !db_path.exists() {
        log::warn!("Database not found at {:?} — nothing to delete.", db_path);
        return Ok(false);
    }

    std::fs::remove_file(&db_path)
        .with_context(|| format!("Failed to delete database at {:?}", db_path))?;

    log::info!("Deleted database at {:?}", db_path);
    Ok(true)
}
