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

fn write_spec(dir: &std::path::Path, name: &str, content: &str) {
    std::fs::create_dir_all(dir.join(".notarai")).unwrap();
    std::fs::write(dir.join(format!(".notarai/{name}")), content).unwrap();
}

#[test]
fn lint_exits_2_when_not_initialized() {
    let tmp = TempDir::new().unwrap();
    cargo_bin_cmd!("notarai")
        .arg("lint")
        .current_dir(tmp.path())
        .assert()
        .code(2)
        .stderr(predicate::str::contains(".notarai/"));
}

#[test]
fn lint_exits_0_when_clean() {
    let tmp = TempDir::new().unwrap();
    setup_git_repo(tmp.path());

    let spec = "\
schema_version: '0.8'
intent: 'Clean spec'
behaviors:
  - name: test_behavior
    given: 'some input'
    then: 'some output'
artifacts:
  code:
    - path: 'src/*.rs'
      role: 'source'
";
    std::fs::create_dir_all(tmp.path().join("src")).unwrap();
    std::fs::write(tmp.path().join("src/main.rs"), "fn main() {}").unwrap();
    write_spec(tmp.path(), "app.spec.yaml", spec);
    git_commit_all(tmp.path(), "initial");

    cargo_bin_cmd!("notarai")
        .arg("lint")
        .current_dir(tmp.path())
        .assert()
        .code(0)
        .stdout(predicate::str::contains("All lint checks passed"));
}

#[test]
fn lint_l001_tier1_no_behaviors() {
    let tmp = TempDir::new().unwrap();
    setup_git_repo(tmp.path());

    let spec = "\
schema_version: '0.8'
intent: 'No behaviors'
artifacts:
  code:
    - path: 'src/*.rs'
      role: 'source'
";
    std::fs::create_dir_all(tmp.path().join("src")).unwrap();
    std::fs::write(tmp.path().join("src/main.rs"), "fn main() {}").unwrap();
    write_spec(tmp.path(), "app.spec.yaml", spec);
    git_commit_all(tmp.path(), "initial");

    cargo_bin_cmd!("notarai")
        .args(["lint", "--format", "json"])
        .current_dir(tmp.path())
        .assert()
        .code(1)
        .stdout(predicate::str::contains("L001"));
}

#[test]
fn lint_l001_skips_registered_tier() {
    let tmp = TempDir::new().unwrap();
    setup_git_repo(tmp.path());

    let spec = "\
schema_version: '0.8'
intent: 'Registered spec'
tier: registered
artifacts:
  code:
    - path: 'src/*.rs'
      role: 'source'
";
    std::fs::create_dir_all(tmp.path().join("src")).unwrap();
    std::fs::write(tmp.path().join("src/main.rs"), "fn main() {}").unwrap();
    write_spec(tmp.path(), "app.spec.yaml", spec);
    git_commit_all(tmp.path(), "initial");

    // Registered tier should not trigger L001.
    cargo_bin_cmd!("notarai")
        .args(["lint", "--format", "json"])
        .current_dir(tmp.path())
        .assert()
        .code(0);
}

#[test]
fn lint_l002_missing_given() {
    let tmp = TempDir::new().unwrap();
    setup_git_repo(tmp.path());

    let spec = "\
schema_version: '0.8'
intent: 'Missing given'
behaviors:
  - name: no_given
    then: 'something'
artifacts:
  code:
    - path: 'src/*.rs'
      role: 'source'
";
    std::fs::create_dir_all(tmp.path().join("src")).unwrap();
    std::fs::write(tmp.path().join("src/main.rs"), "fn main() {}").unwrap();
    write_spec(tmp.path(), "app.spec.yaml", spec);
    git_commit_all(tmp.path(), "initial");

    let output = cargo_bin_cmd!("notarai")
        .args(["lint", "--format", "json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let findings = json["findings"].as_array().unwrap();
    let l002: Vec<_> = findings.iter().filter(|f| f["rule_id"] == "L002").collect();
    assert_eq!(l002.len(), 1);
    assert_eq!(l002[0]["severity"], "warning");
}

#[test]
fn lint_l003_missing_then() {
    let tmp = TempDir::new().unwrap();
    setup_git_repo(tmp.path());

    let spec = "\
schema_version: '0.8'
intent: 'Missing then'
behaviors:
  - name: no_then
    given: 'something'
artifacts:
  code:
    - path: 'src/*.rs'
      role: 'source'
";
    std::fs::create_dir_all(tmp.path().join("src")).unwrap();
    std::fs::write(tmp.path().join("src/main.rs"), "fn main() {}").unwrap();
    write_spec(tmp.path(), "app.spec.yaml", spec);
    git_commit_all(tmp.path(), "initial");

    let output = cargo_bin_cmd!("notarai")
        .args(["lint", "--format", "json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let findings = json["findings"].as_array().unwrap();
    let l003: Vec<_> = findings.iter().filter(|f| f["rule_id"] == "L003").collect();
    assert_eq!(l003.len(), 1);
    assert_eq!(l003[0]["severity"], "warning");
}

#[test]
fn lint_l004_ref_missing() {
    let tmp = TempDir::new().unwrap();
    setup_git_repo(tmp.path());

    let spec = "\
schema_version: '0.8'
intent: 'Bad ref'
tier: registered
artifacts:
  code:
    - path: 'src/*.rs'
      role: 'source'
subsystems:
  - $ref: './nonexistent.spec.yaml'
";
    std::fs::create_dir_all(tmp.path().join("src")).unwrap();
    std::fs::write(tmp.path().join("src/main.rs"), "fn main() {}").unwrap();
    write_spec(tmp.path(), "app.spec.yaml", spec);
    git_commit_all(tmp.path(), "initial");

    cargo_bin_cmd!("notarai")
        .args(["lint", "--format", "json"])
        .current_dir(tmp.path())
        .assert()
        .code(1)
        .stdout(predicate::str::contains("L004"));
}

#[test]
fn lint_l005_circular_ref() {
    let tmp = TempDir::new().unwrap();
    setup_git_repo(tmp.path());

    let spec_a = "\
schema_version: '0.8'
intent: 'A'
tier: registered
artifacts:
  code:
    - path: 'src/*.rs'
      role: 'source'
subsystems:
  - $ref: './b.spec.yaml'
";
    let spec_b = "\
schema_version: '0.8'
intent: 'B'
tier: registered
artifacts:
  code:
    - path: 'src/*.rs'
      role: 'source'
dependencies:
  - $ref: './a.spec.yaml'
";
    std::fs::create_dir_all(tmp.path().join("src")).unwrap();
    std::fs::write(tmp.path().join("src/main.rs"), "fn main() {}").unwrap();
    write_spec(tmp.path(), "a.spec.yaml", spec_a);
    write_spec(tmp.path(), "b.spec.yaml", spec_b);
    git_commit_all(tmp.path(), "initial");

    let output = cargo_bin_cmd!("notarai")
        .args(["lint", "--format", "json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let findings = json["findings"].as_array().unwrap();
    let l005: Vec<_> = findings.iter().filter(|f| f["rule_id"] == "L005").collect();
    assert_eq!(l005.len(), 1);
    assert_eq!(l005[0]["severity"], "warning");
}

#[test]
fn lint_l006_stale_decision() {
    let tmp = TempDir::new().unwrap();
    setup_git_repo(tmp.path());

    // Decision from 2020 with no rationale.
    let spec = "\
schema_version: '0.8'
intent: 'Stale decision'
tier: registered
decisions:
  - date: '2020-01-01'
    choice: 'Old choice with no rationale'
artifacts:
  code:
    - path: 'src/*.rs'
      role: 'source'
";
    std::fs::create_dir_all(tmp.path().join("src")).unwrap();
    std::fs::write(tmp.path().join("src/main.rs"), "fn main() {}").unwrap();
    write_spec(tmp.path(), "app.spec.yaml", spec);
    git_commit_all(tmp.path(), "initial");

    let output = cargo_bin_cmd!("notarai")
        .args(["lint", "--format", "json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let findings = json["findings"].as_array().unwrap();
    let l006: Vec<_> = findings.iter().filter(|f| f["rule_id"] == "L006").collect();
    assert_eq!(l006.len(), 1);
    assert_eq!(l006[0]["severity"], "warning");
}

#[test]
fn lint_l007_open_questions() {
    let tmp = TempDir::new().unwrap();
    setup_git_repo(tmp.path());

    let spec = "\
schema_version: '0.8'
intent: 'Has questions'
tier: registered
open_questions:
  - 'Should we use X or Y?'
artifacts:
  code:
    - path: 'src/*.rs'
      role: 'source'
";
    std::fs::create_dir_all(tmp.path().join("src")).unwrap();
    std::fs::write(tmp.path().join("src/main.rs"), "fn main() {}").unwrap();
    write_spec(tmp.path(), "app.spec.yaml", spec);
    git_commit_all(tmp.path(), "initial");

    let output = cargo_bin_cmd!("notarai")
        .args(["lint", "--format", "json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    // L007 is info severity, should not cause exit 1.
    assert!(output.status.success());

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let findings = json["findings"].as_array().unwrap();
    let l007: Vec<_> = findings.iter().filter(|f| f["rule_id"] == "L007").collect();
    assert_eq!(l007.len(), 1);
    assert_eq!(l007[0]["severity"], "info");
}

#[test]
fn lint_l008_broad_glob() {
    let tmp = TempDir::new().unwrap();
    setup_git_repo(tmp.path());

    let spec = "\
schema_version: '0.8'
intent: 'Broad glob'
tier: registered
artifacts:
  code:
    - path: '**/*'
      role: 'everything'
";
    write_spec(tmp.path(), "app.spec.yaml", spec);
    git_commit_all(tmp.path(), "initial");

    let output = cargo_bin_cmd!("notarai")
        .args(["lint", "--format", "json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let findings = json["findings"].as_array().unwrap();
    let l008: Vec<_> = findings.iter().filter(|f| f["rule_id"] == "L008").collect();
    assert_eq!(l008.len(), 1);
    assert_eq!(l008[0]["severity"], "warning");
}

#[test]
fn lint_l009_schema_mismatch() {
    let tmp = TempDir::new().unwrap();
    setup_git_repo(tmp.path());

    let spec = "\
schema_version: '0.5'
intent: 'Old schema'
tier: registered
artifacts:
  code:
    - path: 'src/*.rs'
      role: 'source'
";
    std::fs::create_dir_all(tmp.path().join("src")).unwrap();
    std::fs::write(tmp.path().join("src/main.rs"), "fn main() {}").unwrap();
    write_spec(tmp.path(), "app.spec.yaml", spec);
    git_commit_all(tmp.path(), "initial");

    cargo_bin_cmd!("notarai")
        .args(["lint", "--format", "json"])
        .current_dir(tmp.path())
        .assert()
        .code(1)
        .stdout(predicate::str::contains("L009"));
}

#[test]
fn lint_l010_duplicate_behaviors() {
    let tmp = TempDir::new().unwrap();
    setup_git_repo(tmp.path());

    let spec = "\
schema_version: '0.8'
intent: 'Duplicate behaviors'
behaviors:
  - name: duplicated
    given: 'input A'
    then: 'output A'
  - name: duplicated
    given: 'input B'
    then: 'output B'
artifacts:
  code:
    - path: 'src/*.rs'
      role: 'source'
";
    std::fs::create_dir_all(tmp.path().join("src")).unwrap();
    std::fs::write(tmp.path().join("src/main.rs"), "fn main() {}").unwrap();
    write_spec(tmp.path(), "app.spec.yaml", spec);
    git_commit_all(tmp.path(), "initial");

    let output = cargo_bin_cmd!("notarai")
        .args(["lint", "--format", "json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let findings = json["findings"].as_array().unwrap();
    let l010: Vec<_> = findings.iter().filter(|f| f["rule_id"] == "L010").collect();
    assert_eq!(l010.len(), 1);
    assert_eq!(l010[0]["severity"], "warning");
    assert!(l010[0]["message"].as_str().unwrap().contains("duplicated"));
}

#[test]
fn lint_config_disables_rule() {
    let tmp = TempDir::new().unwrap();
    setup_git_repo(tmp.path());

    // Spec that would trigger L001.
    let spec = "\
schema_version: '0.8'
intent: 'No behaviors'
artifacts:
  code:
    - path: 'src/*.rs'
      role: 'source'
";
    std::fs::create_dir_all(tmp.path().join("src")).unwrap();
    std::fs::write(tmp.path().join("src/main.rs"), "fn main() {}").unwrap();
    write_spec(tmp.path(), "app.spec.yaml", spec);

    // Disable L001 via config.
    let config = "\
rules:
  L001:
    enabled: false
";
    std::fs::write(tmp.path().join(".notarai/lint.yaml"), config).unwrap();
    git_commit_all(tmp.path(), "initial");

    cargo_bin_cmd!("notarai")
        .args(["lint", "--format", "json"])
        .current_dir(tmp.path())
        .assert()
        .code(0);
}

#[test]
fn lint_config_overrides_severity() {
    let tmp = TempDir::new().unwrap();
    setup_git_repo(tmp.path());

    // Spec that triggers L001 (default: error).
    let spec = "\
schema_version: '0.8'
intent: 'No behaviors'
artifacts:
  code:
    - path: 'src/*.rs'
      role: 'source'
";
    std::fs::create_dir_all(tmp.path().join("src")).unwrap();
    std::fs::write(tmp.path().join("src/main.rs"), "fn main() {}").unwrap();
    write_spec(tmp.path(), "app.spec.yaml", spec);

    // Downgrade L001 to info.
    let config = "\
rules:
  L001:
    severity: info
";
    std::fs::write(tmp.path().join(".notarai/lint.yaml"), config).unwrap();
    git_commit_all(tmp.path(), "initial");

    let output = cargo_bin_cmd!("notarai")
        .args(["lint", "--format", "json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    // Info severity should not cause exit 1.
    assert!(output.status.success());

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let findings = json["findings"].as_array().unwrap();
    let l001: Vec<_> = findings.iter().filter(|f| f["rule_id"] == "L001").collect();
    assert_eq!(l001.len(), 1);
    assert_eq!(l001[0]["severity"], "info");
}

#[test]
fn lint_json_output_structure() {
    let tmp = TempDir::new().unwrap();
    setup_git_repo(tmp.path());

    let spec = "\
schema_version: '0.8'
intent: 'No behaviors'
artifacts:
  code:
    - path: 'src/*.rs'
      role: 'source'
";
    std::fs::create_dir_all(tmp.path().join("src")).unwrap();
    std::fs::write(tmp.path().join("src/main.rs"), "fn main() {}").unwrap();
    write_spec(tmp.path(), "app.spec.yaml", spec);
    git_commit_all(tmp.path(), "initial");

    let output = cargo_bin_cmd!("notarai")
        .args(["lint", "--format", "json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();

    // Verify top-level structure.
    assert!(json.get("findings").unwrap().is_array());
    assert!(json.get("summary").unwrap().is_object());
    assert!(json["summary"]["errors"].is_number());
    assert!(json["summary"]["warnings"].is_number());
    assert!(json["summary"]["infos"].is_number());

    // Verify finding structure.
    let findings = json["findings"].as_array().unwrap();
    assert!(!findings.is_empty());
    let first = &findings[0];
    assert!(first.get("rule_id").is_some());
    assert!(first.get("severity").is_some());
    assert!(first.get("spec_path").is_some());
    assert!(first.get("message").is_some());
}

#[test]
fn lint_l011_cross_cutting_in_subsystems() {
    let tmp = TempDir::new().unwrap();
    setup_git_repo(tmp.path());

    // Cross-cutting spec wrongly referenced via subsystems instead of applies.
    let system_spec = "\
schema_version: '0.8'
intent: 'System'
artifacts:
  code:
    - path: 'src/*.rs'
      role: 'source'
subsystems:
  - $ref: './style.spec.yaml'
behaviors:
  - name: b
    given: g
    then: t
";
    let cross_cutting = "\
schema_version: '0.8'
cross_cutting: true
intent: 'Style'
behaviors:
  - name: american_english
    given: 'british spelling appears'
    then: 'reconciliation flags it'
";
    std::fs::create_dir_all(tmp.path().join("src")).unwrap();
    std::fs::write(tmp.path().join("src/main.rs"), "fn main() {}").unwrap();
    write_spec(tmp.path(), "system.spec.yaml", system_spec);
    write_spec(tmp.path(), "style.spec.yaml", cross_cutting);
    git_commit_all(tmp.path(), "initial");

    let output = cargo_bin_cmd!("notarai")
        .args(["lint", "--format", "json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let findings = json["findings"].as_array().unwrap();
    let l011: Vec<_> = findings.iter().filter(|f| f["rule_id"] == "L011").collect();
    assert_eq!(
        l011.len(),
        1,
        "expected one L011 finding, got: {findings:?}"
    );
    assert_eq!(l011[0]["severity"], "error");
    assert!(
        l011[0]["message"]
            .as_str()
            .unwrap()
            .contains(".notarai/style.spec.yaml")
    );
}

#[test]
fn lint_l011_silent_when_cross_cutting_in_applies() {
    let tmp = TempDir::new().unwrap();
    setup_git_repo(tmp.path());

    // Correct placement: cross-cutting spec referenced via applies.
    let system_spec = "\
schema_version: '0.8'
intent: 'System'
artifacts:
  code:
    - path: 'src/*.rs'
      role: 'source'
applies:
  - $ref: './style.spec.yaml'
behaviors:
  - name: b
    given: g
    then: t
";
    let cross_cutting = "\
schema_version: '0.8'
cross_cutting: true
intent: 'Style'
behaviors:
  - name: american_english
    given: 'british spelling appears'
    then: 'reconciliation flags it'
";
    std::fs::create_dir_all(tmp.path().join("src")).unwrap();
    std::fs::write(tmp.path().join("src/main.rs"), "fn main() {}").unwrap();
    write_spec(tmp.path(), "system.spec.yaml", system_spec);
    write_spec(tmp.path(), "style.spec.yaml", cross_cutting);
    git_commit_all(tmp.path(), "initial");

    let output = cargo_bin_cmd!("notarai")
        .args(["lint", "--format", "json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let findings = json["findings"].as_array().unwrap();
    let l011: Vec<_> = findings.iter().filter(|f| f["rule_id"] == "L011").collect();
    assert_eq!(
        l011.len(),
        0,
        "L011 should not fire for applies: {findings:?}"
    );
}
