use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Return the canonical path to the SQLite cache database.
///
/// Always `.notarai/.cache/notarai.db` relative to `project_root`.
pub fn db_path(project_root: &Path) -> PathBuf {
    project_root
        .join(".notarai")
        .join(".cache")
        .join("notarai.db")
}

/// Open (or create) the SQLite cache database at `project_root`.
///
/// Creates the `.notarai/.cache/` directory if it does not exist, opens the
/// database, and runs the `CREATE TABLE IF NOT EXISTS` migration. Returns a
/// `Connection` ready for use, or an error string.
///
/// Note: `Connection` is not `Sync`, so callers must open a new connection per
/// command invocation -- never store it in a `OnceLock` or shared state.
pub fn open_cache_db(project_root: &Path) -> Result<Connection, String> {
    let path = db_path(project_root);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("could not create cache directory: {e}"))?;
    }
    let conn = Connection::open(&path).map_err(|e| format!("could not open cache DB: {e}"))?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS file_cache (
            path TEXT PRIMARY KEY,
            blake3_hash TEXT NOT NULL,
            updated_at INTEGER NOT NULL
        );",
    )
    .map_err(|e| format!("could not initialize cache schema: {e}"))?;
    Ok(conn)
}

/// Compute the BLAKE3 hash of a file, returned as a lowercase hex string.
pub fn hash_file(path: &Path) -> Result<String, String> {
    let bytes =
        std::fs::read(path).map_err(|e| format!("could not read {}: {e}", path.display()))?;
    Ok(blake3::hash(&bytes).to_hex().to_string())
}

/// Insert or replace a file's hash record in the cache.
///
/// `rel_path` is the cache key -- always a relative path from the project
/// root (e.g., `"src/main.rs"`). Stores the current Unix timestamp as
/// `updated_at`.
pub fn upsert(conn: &Connection, rel_path: &str, hash: &str) -> Result<(), String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    conn.execute(
        "INSERT OR REPLACE INTO file_cache (path, blake3_hash, updated_at) VALUES (?1, ?2, ?3)",
        params![rel_path, hash, now],
    )
    .map_err(|e| format!("upsert failed: {e}"))?;
    Ok(())
}

/// Check whether a file's content has changed since it was last cached.
///
/// Returns `Ok(None)` if the file's current hash matches the cached hash
/// (unchanged). Returns `Ok(Some(hash))` if the file is new, modified, or
/// absent (absent is treated as changed with an empty hash string). Returns
/// `Err` only on I/O or database failure.
///
/// Cache keys use relative paths (`rel_path`); the absolute path (`abs_path`)
/// is used only to read file content for hashing.
pub fn check_changed(
    conn: &Connection,
    rel_path: &str,
    abs_path: &Path,
) -> Result<Option<String>, String> {
    if !abs_path.exists() {
        // Treat absence as changed
        return Ok(Some(String::new()));
    }
    let current_hash = hash_file(abs_path)?;
    let cached: Option<String> = conn
        .query_row(
            "SELECT blake3_hash FROM file_cache WHERE path = ?1",
            params![rel_path],
            |row| row.get(0),
        )
        .ok();
    match cached {
        Some(h) if h == current_hash => Ok(None),
        _ => Ok(Some(current_hash)),
    }
}

/// Return the number of cached entries and the most recent `updated_at` timestamp.
///
/// The timestamp is `None` when the cache is empty.
pub fn status(conn: &Connection) -> Result<(usize, Option<i64>), String> {
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM file_cache", [], |row| row.get(0))
        .map_err(|e| format!("count query failed: {e}"))?;
    let count = count as usize;
    let newest: Option<i64> = conn
        .query_row("SELECT MAX(updated_at) FROM file_cache", [], |row| {
            row.get(0)
        })
        .map_err(|e| format!("newest query failed: {e}"))?;
    Ok((count, newest))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn hash_is_consistent() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("test.txt");
        std::fs::write(&file, b"hello world").unwrap();
        let h1 = hash_file(&file).unwrap();
        let h2 = hash_file(&file).unwrap();
        assert_eq!(h1, h2);
    }

    #[test]
    fn upsert_then_unchanged() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("test.txt");
        std::fs::write(&file, b"hello").unwrap();
        let conn = open_cache_db(tmp.path()).unwrap();
        let hash = hash_file(&file).unwrap();
        upsert(&conn, file.to_str().unwrap(), &hash).unwrap();
        let result = check_changed(&conn, file.to_str().unwrap(), &file).unwrap();
        assert!(result.is_none(), "expected None for unchanged file");
    }

    #[test]
    fn modified_file_returns_some() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("test.txt");
        std::fs::write(&file, b"hello").unwrap();
        let conn = open_cache_db(tmp.path()).unwrap();
        let hash = hash_file(&file).unwrap();
        upsert(&conn, file.to_str().unwrap(), &hash).unwrap();
        std::fs::write(&file, b"world").unwrap();
        let result = check_changed(&conn, file.to_str().unwrap(), &file).unwrap();
        assert!(result.is_some(), "expected Some for modified file");
    }

    #[test]
    fn missing_file_returns_some() {
        let tmp = TempDir::new().unwrap();
        let conn = open_cache_db(tmp.path()).unwrap();
        let missing = tmp.path().join("nonexistent.txt");
        let result = check_changed(&conn, missing.to_str().unwrap(), &missing).unwrap();
        assert!(result.is_some(), "expected Some for missing file");
    }
}
