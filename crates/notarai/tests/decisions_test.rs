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

const MINIMAL_SPEC: &str = "\
schema_version: '0.8'
intent: 'Test spec'
tier: registered
artifacts:
  code:
    - path: 'src/*.rs'
      role: 'source code'
";

fn write_decision_log(dir: &std::path::Path, content: &str) {
    std::fs::write(dir.join(".notarai/decision-log.json"), content).unwrap();
}

#[test]
fn decisions_exits_2_when_not_initialized() {
    let tmp = TempDir::new().unwrap();
    cargo_bin_cmd!("notarai")
        .args(["decisions", "list"])
        .current_dir(tmp.path())
        .assert()
        .code(2)
        .stderr(predicate::str::contains(".notarai/"));
}

#[test]
fn decisions_list_empty_log() {
    let tmp = TempDir::new().unwrap();
    std::fs::create_dir_all(tmp.path().join(".notarai")).unwrap();

    cargo_bin_cmd!("notarai")
        .args(["decisions", "list"])
        .current_dir(tmp.path())
        .assert()
        .code(0)
        .stdout(predicate::str::contains("No decisions"));
}

#[test]
fn decisions_list_shows_proposals() {
    let tmp = TempDir::new().unwrap();
    std::fs::create_dir_all(tmp.path().join(".notarai")).unwrap();

    write_decision_log(
        tmp.path(),
        r#"{"proposals":[{
            "spec_path":".notarai/app.spec.yaml",
            "date":"2026-04-13",
            "choice":"Update retry behavior",
            "rationale":"Code changed retry logic",
            "origin":"reconciliation",
            "status":"proposed",
            "related_files":["src/http.rs"]
        }]}"#,
    );

    cargo_bin_cmd!("notarai")
        .args(["decisions", "list"])
        .current_dir(tmp.path())
        .assert()
        .code(0)
        .stdout(predicate::str::contains("Update retry behavior"))
        .stdout(predicate::str::contains("proposed"))
        .stdout(predicate::str::contains("1 decision(s)"));
}

#[test]
fn decisions_list_filters_by_status() {
    let tmp = TempDir::new().unwrap();
    std::fs::create_dir_all(tmp.path().join(".notarai")).unwrap();

    write_decision_log(
        tmp.path(),
        r#"{"proposals":[
            {"spec_path":".notarai/a.spec.yaml","date":"2026-04-13","choice":"Choice A","rationale":"R","origin":"human","status":"proposed","related_files":[]},
            {"spec_path":".notarai/b.spec.yaml","date":"2026-04-13","choice":"Choice B","rationale":"R","origin":"human","status":"rejected","related_files":[]}
        ]}"#,
    );

    // Filter proposed: only Choice A.
    cargo_bin_cmd!("notarai")
        .args(["decisions", "list", "--status", "proposed"])
        .current_dir(tmp.path())
        .assert()
        .code(0)
        .stdout(predicate::str::contains("Choice A"))
        .stdout(predicate::str::contains("1 decision(s)"));

    // Filter rejected: only Choice B.
    cargo_bin_cmd!("notarai")
        .args(["decisions", "list", "--status", "rejected"])
        .current_dir(tmp.path())
        .assert()
        .code(0)
        .stdout(predicate::str::contains("Choice B"))
        .stdout(predicate::str::contains("1 decision(s)"));
}

#[test]
fn decisions_accept_appends_to_spec() {
    let tmp = TempDir::new().unwrap();
    setup_git_repo(tmp.path());

    std::fs::create_dir_all(tmp.path().join(".notarai")).unwrap();
    std::fs::create_dir_all(tmp.path().join("src")).unwrap();
    std::fs::write(tmp.path().join("src/main.rs"), "fn main() {}").unwrap();
    std::fs::write(tmp.path().join(".notarai/app.spec.yaml"), MINIMAL_SPEC).unwrap();

    write_decision_log(
        tmp.path(),
        r#"{"proposals":[{
            "spec_path":".notarai/app.spec.yaml",
            "date":"2026-04-13",
            "choice":"Add retry behavior",
            "rationale":"Code now retries on failure",
            "origin":"reconciliation",
            "status":"proposed",
            "related_files":["src/main.rs"]
        }]}"#,
    );

    cargo_bin_cmd!("notarai")
        .args(["decisions", "accept", ".notarai/app.spec.yaml", "0"])
        .current_dir(tmp.path())
        .assert()
        .code(0)
        .stdout(predicate::str::contains("Accepted"));

    // Verify the spec now contains a decisions section.
    let spec_content = std::fs::read_to_string(tmp.path().join(".notarai/app.spec.yaml")).unwrap();
    assert!(
        spec_content.contains("decisions:"),
        "spec should contain decisions section"
    );
    assert!(
        spec_content.contains("Add retry behavior"),
        "spec should contain the accepted choice"
    );

    // Verify the log is updated (proposal removed).
    let log_content =
        std::fs::read_to_string(tmp.path().join(".notarai/decision-log.json")).unwrap();
    let log: serde_json::Value = serde_json::from_str(&log_content).unwrap();
    assert_eq!(
        log["proposals"].as_array().unwrap().len(),
        0,
        "accepted proposal should be removed from log"
    );

    // Verify the spec still validates.
    cargo_bin_cmd!("notarai")
        .args(["validate", ".notarai/app.spec.yaml"])
        .current_dir(tmp.path())
        .assert()
        .code(0);
}

#[test]
fn decisions_reject_marks_in_log() {
    let tmp = TempDir::new().unwrap();
    std::fs::create_dir_all(tmp.path().join(".notarai")).unwrap();

    write_decision_log(
        tmp.path(),
        r#"{"proposals":[{
            "spec_path":".notarai/app.spec.yaml",
            "date":"2026-04-13",
            "choice":"Remove legacy endpoint",
            "rationale":"Endpoint still in use",
            "origin":"reconciliation",
            "status":"proposed",
            "related_files":[]
        }]}"#,
    );

    cargo_bin_cmd!("notarai")
        .args([
            "decisions",
            "reject",
            ".notarai/app.spec.yaml",
            "0",
            "--reason",
            "Still needed",
        ])
        .current_dir(tmp.path())
        .assert()
        .code(0)
        .stdout(predicate::str::contains("Rejected"));

    // Verify the log shows rejected status and reason.
    let log_content =
        std::fs::read_to_string(tmp.path().join(".notarai/decision-log.json")).unwrap();
    let log: serde_json::Value = serde_json::from_str(&log_content).unwrap();
    let proposal = &log["proposals"][0];
    assert_eq!(proposal["status"], "rejected");
    assert_eq!(proposal["reject_reason"], "Still needed");
}

#[test]
fn decisions_accept_wrong_spec_errors() {
    let tmp = TempDir::new().unwrap();
    std::fs::create_dir_all(tmp.path().join(".notarai")).unwrap();

    write_decision_log(
        tmp.path(),
        r#"{"proposals":[{
            "spec_path":".notarai/app.spec.yaml",
            "date":"2026-04-13",
            "choice":"Something",
            "rationale":"Reason",
            "origin":"human",
            "status":"proposed",
            "related_files":[]
        }]}"#,
    );

    // Try to accept with wrong spec path.
    cargo_bin_cmd!("notarai")
        .args(["decisions", "accept", ".notarai/other.spec.yaml", "0"])
        .current_dir(tmp.path())
        .assert()
        .code(1)
        .stderr(predicate::str::contains("belongs to"));
}

#[test]
fn decisions_accept_out_of_range_errors() {
    let tmp = TempDir::new().unwrap();
    std::fs::create_dir_all(tmp.path().join(".notarai")).unwrap();

    write_decision_log(tmp.path(), r#"{"proposals":[]}"#);

    cargo_bin_cmd!("notarai")
        .args(["decisions", "accept", ".notarai/app.spec.yaml", "0"])
        .current_dir(tmp.path())
        .assert()
        .code(1)
        .stderr(predicate::str::contains("out of range"));
}
