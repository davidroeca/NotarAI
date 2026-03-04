use assert_cmd::cargo_bin_cmd;
use predicates::prelude::*;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn notarai() -> assert_cmd::Command {
    cargo_bin_cmd!("notarai")
}

fn setup_git_repo(dir: &Path) {
    for args in [
        vec!["init"],
        vec!["config", "user.email", "test@notarai.dev"],
        vec!["config", "user.name", "NotarAI Test"],
        vec!["config", "commit.gpgsign", "false"],
    ] {
        std::process::Command::new("git")
            .args(&args)
            .current_dir(dir)
            .output()
            .unwrap();
    }
}

fn git_commit_all(dir: &Path, msg: &str) {
    for args in [vec!["add", "."], vec!["commit", "-m", msg]] {
        std::process::Command::new("git")
            .args(&args)
            .current_dir(dir)
            .output()
            .unwrap();
    }
}

// -- notarai state show -------------------------------------------------------

#[test]
fn test_state_show_no_state() {
    let tmp = TempDir::new().unwrap();
    setup_git_repo(tmp.path());
    fs::create_dir_all(tmp.path().join(".notarai")).unwrap();

    notarai()
        .arg("state")
        .arg("show")
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("No reconciliation state found."));
}

// -- notarai state reset -------------------------------------------------------

#[test]
fn test_state_reset_no_state() {
    let tmp = TempDir::new().unwrap();
    setup_git_repo(tmp.path());
    fs::create_dir_all(tmp.path().join(".notarai")).unwrap();

    notarai()
        .arg("state")
        .arg("reset")
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "No reconciliation state to reset.",
        ));
}

#[test]
fn test_state_reset_deletes_file() {
    let tmp = TempDir::new().unwrap();
    setup_git_repo(tmp.path());
    fs::create_dir_all(tmp.path().join(".notarai")).unwrap();

    // Write a dummy state file.
    let state_path = tmp
        .path()
        .join(".notarai")
        .join("reconciliation_state.json");
    fs::write(&state_path, r#"{"schema_version":"1"}"#).unwrap();
    assert!(state_path.exists());

    notarai()
        .arg("state")
        .arg("reset")
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Reconciliation state reset."));

    assert!(!state_path.exists());
}

// -- notarai state snapshot + show --------------------------------------------

#[test]
fn test_state_snapshot_and_show() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    setup_git_repo(root);
    fs::create_dir_all(root.join(".notarai")).unwrap();
    fs::write(root.join("src/main.rs"), "fn main() {}").unwrap_or_else(|_| {
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/main.rs"), "fn main() {}").unwrap();
    });
    git_commit_all(root, "initial");

    // Seed the cache by calling mark_reconciled via MCP, then snapshot.
    let seed_msg = r#"{"jsonrpc":"2.0","id":0,"method":"tools/call","params":{"name":"mark_reconciled","arguments":{"files":["src/main.rs"]}}}"#;
    notarai()
        .arg("mcp")
        .write_stdin(format!("{seed_msg}\n"))
        .current_dir(root)
        .assert()
        .success();

    // Snapshot via CLI.
    notarai()
        .arg("state")
        .arg("snapshot")
        .current_dir(root)
        .assert()
        .success()
        .stdout(predicate::str::contains("Snapshot saved."))
        .stdout(predicate::str::contains("Files:"));

    // State file must exist.
    assert!(root.join(".notarai/reconciliation_state.json").exists());

    // Show should print the summary.
    notarai()
        .arg("state")
        .arg("show")
        .current_dir(root)
        .assert()
        .success()
        .stdout(predicate::str::contains("Last reconciliation:"))
        .stdout(predicate::str::contains("Files:"));
}

// -- notarai state snapshot: JSON is valid ------------------------------------

#[test]
fn test_state_snapshot_produces_valid_json() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    setup_git_repo(root);
    fs::create_dir_all(root.join(".notarai")).unwrap();
    git_commit_all(root, "initial");

    notarai()
        .arg("state")
        .arg("snapshot")
        .current_dir(root)
        .assert()
        .success();

    let content = fs::read_to_string(root.join(".notarai/reconciliation_state.json")).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(parsed["schema_version"], "1");
    assert!(parsed["file_fingerprints"].is_object());
    assert!(parsed["spec_fingerprints"].is_object());
}
