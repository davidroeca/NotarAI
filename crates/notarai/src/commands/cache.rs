use crate::core::cache;
use clap::Subcommand;
use std::path::{Path, PathBuf};

#[derive(Subcommand)]
pub enum CacheAction {
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
    }
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
