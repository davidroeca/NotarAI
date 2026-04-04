use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

fn setup_git_repo(dir: &std::path::Path) {
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(dir)
        .output()
        .expect("git init");
    std::process::Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(dir)
        .output()
        .expect("git config email");
    std::process::Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(dir)
        .output()
        .expect("git config name");
}

fn git_commit_all(dir: &std::path::Path, msg: &str) {
    std::process::Command::new("git")
        .args(["add", "-A"])
        .current_dir(dir)
        .output()
        .expect("git add");
    std::process::Command::new("git")
        .args(["commit", "-m", msg, "--allow-empty"])
        .current_dir(dir)
        .output()
        .expect("git commit");
}

const MINIMAL_SPEC: &str = "\
schema_version: '0.7'
intent: 'Test spec'
artifacts:
  code:
    - path: 'src/*.rs'
      role: 'source code'
";

fn notarai() -> assert_cmd::Command {
    cargo_bin_cmd!("notarai")
}

#[test]
fn export_context_exits_2_when_not_initialized() {
    let tmp = TempDir::new().unwrap();
    notarai()
        .args(["export-context", "--all"])
        .current_dir(tmp.path())
        .assert()
        .code(2)
        .stderr(predicate::str::contains(".notarai/"));
}

#[test]
fn export_context_requires_spec_or_all() {
    let tmp = TempDir::new().unwrap();
    setup_git_repo(tmp.path());
    fs::create_dir_all(tmp.path().join(".notarai")).unwrap();

    // Neither --spec nor --all
    notarai()
        .args(["export-context"])
        .current_dir(tmp.path())
        .assert()
        .code(1)
        .stderr(predicate::str::contains("--spec").or(predicate::str::contains("--all")));

    // Both --spec and --all
    notarai()
        .args([
            "export-context",
            "--spec",
            ".notarai/test.spec.yaml",
            "--all",
        ])
        .current_dir(tmp.path())
        .assert()
        .code(1);
}

#[test]
fn export_context_markdown_single_spec() {
    let tmp = TempDir::new().unwrap();
    setup_git_repo(tmp.path());

    // Set up project with a spec and source file
    fs::create_dir_all(tmp.path().join(".notarai")).unwrap();
    fs::create_dir_all(tmp.path().join("src")).unwrap();
    fs::write(tmp.path().join(".notarai/app.spec.yaml"), MINIMAL_SPEC).unwrap();
    fs::write(tmp.path().join("src/main.rs"), "fn main() {}").unwrap();
    git_commit_all(tmp.path(), "initial");

    // Make a change on the same branch
    fs::write(
        tmp.path().join("src/main.rs"),
        "fn main() { println!(\"hi\"); }",
    )
    .unwrap();
    git_commit_all(tmp.path(), "update");

    notarai()
        .args([
            "export-context",
            "--spec",
            ".notarai/app.spec.yaml",
            "--base-branch",
            "HEAD~1",
            "--format",
            "markdown",
        ])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Reconciliation"))
        .stdout(predicate::str::contains("src/main.rs"));
}

#[test]
fn export_context_json_single_spec() {
    let tmp = TempDir::new().unwrap();
    setup_git_repo(tmp.path());

    fs::create_dir_all(tmp.path().join(".notarai")).unwrap();
    fs::create_dir_all(tmp.path().join("src")).unwrap();
    fs::write(tmp.path().join(".notarai/app.spec.yaml"), MINIMAL_SPEC).unwrap();
    fs::write(tmp.path().join("src/main.rs"), "fn main() {}").unwrap();
    git_commit_all(tmp.path(), "initial");

    fs::write(
        tmp.path().join("src/main.rs"),
        "fn main() { println!(\"hi\"); }",
    )
    .unwrap();
    git_commit_all(tmp.path(), "update");

    let output = notarai()
        .args([
            "export-context",
            "--spec",
            ".notarai/app.spec.yaml",
            "--base-branch",
            "HEAD~1",
            "--format",
            "json",
        ])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("valid JSON output");
    assert!(json["spec_name"].as_str().is_some());
    assert!(json["changed_files"].as_array().is_some());
    assert!(json["diff"].as_str().is_some());
}

#[test]
fn export_context_all_specs() {
    let tmp = TempDir::new().unwrap();
    setup_git_repo(tmp.path());

    fs::create_dir_all(tmp.path().join(".notarai")).unwrap();
    fs::create_dir_all(tmp.path().join("src")).unwrap();
    fs::write(tmp.path().join(".notarai/app.spec.yaml"), MINIMAL_SPEC).unwrap();
    fs::write(tmp.path().join("src/main.rs"), "fn main() {}").unwrap();
    git_commit_all(tmp.path(), "initial");

    fs::write(
        tmp.path().join("src/main.rs"),
        "fn main() { println!(\"hi\"); }",
    )
    .unwrap();
    git_commit_all(tmp.path(), "update");

    notarai()
        .args([
            "export-context",
            "--all",
            "--base-branch",
            "HEAD~1",
            "--format",
            "json",
        ])
        .current_dir(tmp.path())
        .assert()
        .success();
}
