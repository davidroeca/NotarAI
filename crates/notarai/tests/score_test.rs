use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
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

fn git_add_commit(dir: &std::path::Path) {
    std::process::Command::new("git")
        .args(["add", "-A"])
        .current_dir(dir)
        .output()
        .expect("git add");
    std::process::Command::new("git")
        .args(["commit", "-m", "initial", "--no-gpg-sign"])
        .current_dir(dir)
        .output()
        .expect("git commit");
}

const SPEC_COVERED: &str = "\
schema_version: '0.8'
intent: 'Test spec'
tier: registered
artifacts:
  code:
    - path: 'src/lib.rs'
      role: 'source code'
";

fn init_project(dir: &std::path::Path) {
    std::fs::create_dir_all(dir.join(".notarai")).unwrap();
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(dir.join("src/lib.rs"), "pub fn x() {}\n").unwrap();
    std::fs::write(dir.join(".notarai/app.spec.yaml"), SPEC_COVERED).unwrap();
}

#[test]
fn score_exits_2_when_not_initialized() {
    let tmp = TempDir::new().unwrap();
    cargo_bin_cmd!("notarai")
        .args(["score"])
        .current_dir(tmp.path())
        .assert()
        .code(2)
        .stderr(predicate::str::contains(".notarai/"));
}

#[test]
fn score_empty_project_no_specs() {
    let tmp = TempDir::new().unwrap();
    std::fs::create_dir_all(tmp.path().join(".notarai")).unwrap();

    cargo_bin_cmd!("notarai")
        .args(["score"])
        .current_dir(tmp.path())
        .assert()
        .code(0)
        .stdout(predicate::str::contains("No specs found"));
}

#[test]
fn score_json_format() {
    let tmp = TempDir::new().unwrap();
    setup_git_repo(tmp.path());
    init_project(tmp.path());
    git_add_commit(tmp.path());

    let output = cargo_bin_cmd!("notarai")
        .args(["score", "--format", "json"])
        .current_dir(tmp.path())
        .env("NOTARAI_TODAY", "2026-04-13")
        .output()
        .expect("run score");

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("valid json");
    assert!(parsed.get("specs").is_some());
    assert!(parsed.get("overall").is_some());
    assert!(parsed["overall"].get("score").is_some());
    assert!(parsed["overall"].get("status").is_some());
}

#[test]
fn score_human_format_shows_status() {
    let tmp = TempDir::new().unwrap();
    setup_git_repo(tmp.path());
    init_project(tmp.path());
    git_add_commit(tmp.path());

    cargo_bin_cmd!("notarai")
        .args(["score"])
        .current_dir(tmp.path())
        .env("NOTARAI_TODAY", "2026-04-13")
        .assert()
        .code(0)
        .stdout(predicate::str::contains("Spec"))
        .stdout(predicate::str::contains("Overall:"));
}

#[test]
fn score_single_spec_filter() {
    let tmp = TempDir::new().unwrap();
    setup_git_repo(tmp.path());
    init_project(tmp.path());
    // Add a second spec so we can verify filtering.
    std::fs::write(
        tmp.path().join(".notarai/other.spec.yaml"),
        "schema_version: '0.8'\nintent: 'Other'\ntier: registered\nartifacts:\n  code:\n    - path: 'src/lib.rs'\n      role: 'src'\n",
    )
    .unwrap();
    git_add_commit(tmp.path());

    let output = cargo_bin_cmd!("notarai")
        .args([
            "score",
            "--format",
            "json",
            "--spec",
            ".notarai/app.spec.yaml",
        ])
        .current_dir(tmp.path())
        .env("NOTARAI_TODAY", "2026-04-13")
        .output()
        .expect("run score");

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("valid json");
    let specs = parsed["specs"].as_array().expect("specs array");
    assert_eq!(specs.len(), 1);
    assert_eq!(specs[0]["spec_path"], ".notarai/app.spec.yaml");
}
