use crate::core::cache;
use clap::Subcommand;
use std::io::BufRead;
use std::path::{Path, PathBuf};

#[derive(Subcommand)]
pub enum CacheAction {
    /// Hash files and update cache
    Update {
        /// Files to hash and cache (reads from stdin if empty)
        paths: Vec<String>,
    },
    /// Print paths that have changed since last cache update
    Changed {
        /// Files to check (reads from stdin if empty)
        paths: Vec<String>,
    },
    /// Delete the cache database
    Clear,
    /// Show cache status
    Status,
}

pub fn run(action: CacheAction) -> i32 {
    let root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    match action {
        CacheAction::Status => run_status(&root),
        CacheAction::Clear => run_clear(&root),
        CacheAction::Update { paths } => run_update(&root, paths),
        CacheAction::Changed { paths } => run_changed(&root, paths),
    }
}

fn collect_paths(args: Vec<String>) -> Vec<String> {
    if args.is_empty() {
        let stdin = std::io::stdin();
        stdin
            .lock()
            .lines()
            .map_while(Result::ok)
            .filter(|l| !l.trim().is_empty())
            .collect()
    } else {
        args
    }
}

fn resolve_path(p: &str, cwd: &Path) -> (String, PathBuf) {
    let abs = if Path::new(p).is_absolute() {
        PathBuf::from(p)
    } else {
        cwd.join(p)
    };
    let canonical = abs.canonicalize().unwrap_or(abs);
    (canonical.to_string_lossy().into_owned(), canonical)
}

fn run_status(root: &Path) -> i32 {
    let db_path = cache::db_path(root);
    if !db_path.exists() {
        println!("Cache DB: {} (not initialized)", db_path.display());
        println!("Entries: 0");
        return 0;
    }
    let conn = match cache::open_cache_db(root) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: {e}");
            return 1;
        }
    };
    match cache::status(&conn) {
        Ok((count, newest)) => {
            println!("Cache DB: {}", db_path.display());
            println!("Entries: {count}");
            if let Some(ts) = newest {
                println!("Newest: {ts}");
            }
            0
        }
        Err(e) => {
            eprintln!("Error: {e}");
            1
        }
    }
}

fn run_clear(root: &Path) -> i32 {
    let db_path = cache::db_path(root);
    if db_path.exists() {
        if let Err(e) = std::fs::remove_file(&db_path) {
            eprintln!("Error: could not delete cache DB: {e}");
            return 1;
        }
        println!("Cache cleared");
    } else {
        println!("Cache not initialized");
    }
    0
}

fn run_update(root: &Path, paths: Vec<String>) -> i32 {
    let paths = collect_paths(paths);
    let conn = match cache::open_cache_db(root) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: {e}");
            return 1;
        }
    };
    let resolved: Vec<(String, PathBuf)> = paths.iter().map(|p| resolve_path(p, root)).collect();
    let pairs: Vec<(&str, &Path)> = resolved
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_path()))
        .collect();
    match cache::update_batch(&conn, &pairs) {
        Ok(count) => {
            println!("Updated {count} file(s)");
            0
        }
        Err(e) => {
            eprintln!("Error: {e}");
            1
        }
    }
}

fn run_changed(root: &Path, paths: Vec<String>) -> i32 {
    let paths = collect_paths(paths);
    let conn = match cache::open_cache_db(root) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: {e}");
            return 1;
        }
    };
    for p in &paths {
        let (key, abs) = resolve_path(p, root);
        match cache::check_changed(&conn, &key, &abs) {
            Ok(Some(_)) => println!("{p}"),
            Ok(None) => {}
            Err(e) => eprintln!("Error checking {p}: {e}"),
        }
    }
    0
}
