use assert_cmd::cargo_bin_cmd;
use predicates::prelude::*;

fn notarai() -> assert_cmd::Command {
    cargo_bin_cmd!("notarai")
}

#[test]
fn update_check_runs_without_crash() {
    // Network may be unavailable, so accept exit 0 or 1
    let output = notarai().args(["update", "--check"]).output().unwrap();
    assert!(
        output.status.code() == Some(0) || output.status.code() == Some(1),
        "Expected exit 0 or 1, got {:?}",
        output.status.code()
    );
}

#[test]
fn update_help_shows_usage() {
    notarai()
        .args(["update", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Check for and install updates"));
}
