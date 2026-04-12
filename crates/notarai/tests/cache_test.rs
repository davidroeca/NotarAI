use assert_cmd::cargo_bin_cmd;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

fn notarai() -> assert_cmd::Command {
    cargo_bin_cmd!("notarai")
}

#[test]
fn status_exits_0_on_empty_cache() {
    let tmp = TempDir::new().unwrap();
    notarai()
        .args(["cache", "status"])
        .current_dir(tmp.path())
        .assert()
        .success();
}

#[test]
fn status_prints_entries_0_when_no_db() {
    let tmp = TempDir::new().unwrap();
    notarai()
        .args(["cache", "status"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Entries: 0"));
}

#[test]
fn clear_exits_0_when_no_db() {
    let tmp = TempDir::new().unwrap();
    notarai()
        .args(["cache", "clear"])
        .current_dir(tmp.path())
        .assert()
        .success();
}

#[test]
fn clear_prints_not_initialized_when_no_db() {
    let tmp = TempDir::new().unwrap();
    notarai()
        .args(["cache", "clear"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("not initialized"));
}

#[test]
fn clear_removes_db_when_it_exists() {
    let tmp = TempDir::new().unwrap();
    let db_dir = tmp.path().join(".notarai/.cache");
    fs::create_dir_all(&db_dir).unwrap();
    let db = db_dir.join("notarai.db");
    // Write a placeholder file to simulate an existing cache DB
    fs::write(&db, b"placeholder").unwrap();

    assert!(db.exists());

    notarai()
        .args(["cache", "clear"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("cleared"));

    assert!(!db.exists());
}
