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
        .success()
        .stdout(predicate::str::contains("Entries: 0"));
}

#[test]
fn update_hashes_given_files() {
    let tmp = TempDir::new().unwrap();
    let file = tmp.path().join("test.txt");
    fs::write(&file, "hello").unwrap();

    notarai()
        .args(["cache", "update", file.to_str().unwrap()])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Updated 1"));

    notarai()
        .args(["cache", "status"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Entries: 1"));
}

#[test]
fn changed_prints_new_file() {
    let tmp = TempDir::new().unwrap();
    let file = tmp.path().join("test.txt");
    fs::write(&file, "hello").unwrap();

    notarai()
        .args(["cache", "changed", file.to_str().unwrap()])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(file.to_str().unwrap()));
}

#[test]
fn changed_prints_nothing_for_cached_unchanged() {
    let tmp = TempDir::new().unwrap();
    let file = tmp.path().join("test.txt");
    fs::write(&file, "hello").unwrap();

    notarai()
        .args(["cache", "update", file.to_str().unwrap()])
        .current_dir(tmp.path())
        .assert()
        .success();

    notarai()
        .args(["cache", "changed", file.to_str().unwrap()])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout("");
}

#[test]
fn changed_prints_path_after_modification() {
    let tmp = TempDir::new().unwrap();
    let file = tmp.path().join("test.txt");
    fs::write(&file, "hello").unwrap();

    notarai()
        .args(["cache", "update", file.to_str().unwrap()])
        .current_dir(tmp.path())
        .assert()
        .success();

    fs::write(&file, "world").unwrap();

    notarai()
        .args(["cache", "changed", file.to_str().unwrap()])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(file.to_str().unwrap()));
}

#[test]
fn clear_removes_db() {
    let tmp = TempDir::new().unwrap();
    let file = tmp.path().join("test.txt");
    fs::write(&file, "hello").unwrap();

    notarai()
        .args(["cache", "update", file.to_str().unwrap()])
        .current_dir(tmp.path())
        .assert()
        .success();

    let db = tmp.path().join(".notarai/.cache/notarai.db");
    assert!(db.exists());

    notarai()
        .args(["cache", "clear"])
        .current_dir(tmp.path())
        .assert()
        .success();

    assert!(!db.exists());
}

#[test]
fn update_reads_from_stdin_when_no_args() {
    let tmp = TempDir::new().unwrap();
    let file = tmp.path().join("test.txt");
    fs::write(&file, "hello").unwrap();

    notarai()
        .args(["cache", "update"])
        .current_dir(tmp.path())
        .write_stdin(format!("{}\n", file.to_str().unwrap()))
        .assert()
        .success()
        .stdout(predicate::str::contains("Updated 1"));
}
