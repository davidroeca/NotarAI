use assert_cmd::cargo_bin_cmd;
use predicates::prelude::*;

fn notarai() -> assert_cmd::Command {
    cargo_bin_cmd!("notarai")
}

#[test]
fn help_flag_exits_0_and_prints_usage() {
    notarai()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("notarai"));
}

#[test]
fn unknown_command_exits_2() {
    // clap exits with code 2 for parse errors
    notarai()
        .arg("nonexistent")
        .assert()
        .code(2)
        .stderr(predicate::str::contains("notarai"));
}

#[test]
fn validate_exits_0_for_repo_notarai_dir() {
    notarai()
        .arg("validate")
        .assert()
        .success()
        .stdout(predicate::str::contains("PASS"));
}

#[test]
fn validate_exits_1_for_missing_path() {
    notarai()
        .args(["validate", "/nonexistent/path"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("path not found"));
}

#[test]
fn no_command_exits_1() {
    notarai()
        .assert()
        .code(1)
        .stdout(predicate::str::contains("notarai"));
}
