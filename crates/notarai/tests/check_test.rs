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

const MINIMAL_SPEC: &str = "\
schema_version: '0.7'
intent: 'Test spec'
artifacts:
  code:
    - path: 'src/*.rs'
      role: 'source code'
";

const SYSTEM_SPEC: &str = "\
schema_version: '0.7'
intent: 'System spec'
artifacts:
  configs:
    - path: 'config.toml'
      role: 'config'
subsystems:
  - $ref: '.notarai/app.spec.yaml'
exclude:
  - 'vendor/**'
  - 'build/**'
";

#[test]
fn check_exits_2_when_not_initialized() {
    let tmp = TempDir::new().unwrap();
    cargo_bin_cmd!("notarai")
        .arg("check")
        .current_dir(tmp.path())
        .assert()
        .code(2)
        .stderr(predicate::str::contains(".notarai/"));
}

#[test]
fn check_exits_0_when_clean() {
    let tmp = TempDir::new().unwrap();
    setup_git_repo(tmp.path());

    // Create .notarai with a spec that governs existing files.
    std::fs::create_dir_all(tmp.path().join(".notarai")).unwrap();
    std::fs::create_dir_all(tmp.path().join("src")).unwrap();
    std::fs::write(tmp.path().join("src/main.rs"), "fn main() {}").unwrap();
    std::fs::write(tmp.path().join(".notarai/app.spec.yaml"), MINIMAL_SPEC).unwrap();

    // System spec excludes nothing and governs the only other file.
    let system_spec = "\
schema_version: '0.7'
intent: 'System'
artifacts:
  configs:
    - path: '.gitignore'
      role: 'git ignore'
subsystems:
  - $ref: '.notarai/app.spec.yaml'
exclude:
  - '.eslint*'
";
    std::fs::write(tmp.path().join(".notarai/system.spec.yaml"), system_spec).unwrap();
    std::fs::write(tmp.path().join(".gitignore"), "").unwrap();

    git_commit_all(tmp.path(), "initial");

    cargo_bin_cmd!("notarai")
        .arg("check")
        .current_dir(tmp.path())
        .assert()
        .code(0)
        .stdout(predicate::str::contains("All checks passed"));
}

#[test]
fn check_detects_coverage_gap() {
    let tmp = TempDir::new().unwrap();
    setup_git_repo(tmp.path());

    std::fs::create_dir_all(tmp.path().join(".notarai")).unwrap();
    std::fs::create_dir_all(tmp.path().join("src")).unwrap();
    std::fs::write(tmp.path().join("src/main.rs"), "fn main() {}").unwrap();
    std::fs::write(tmp.path().join(".notarai/app.spec.yaml"), MINIMAL_SPEC).unwrap();

    // Create an ungoverned file.
    std::fs::write(tmp.path().join("orphan.txt"), "ungoverned").unwrap();

    git_commit_all(tmp.path(), "initial");

    cargo_bin_cmd!("notarai")
        .arg("check")
        .current_dir(tmp.path())
        .assert()
        .code(0) // Warnings only, not errors.
        .stdout(predicate::str::contains("orphan.txt"));
}

#[test]
fn check_detects_orphaned_glob() {
    let tmp = TempDir::new().unwrap();
    setup_git_repo(tmp.path());

    std::fs::create_dir_all(tmp.path().join(".notarai")).unwrap();

    // Spec references a glob that matches nothing.
    let spec = "\
schema_version: '0.7'
intent: 'Test spec'
artifacts:
  code:
    - path: 'nonexistent/**/*.rs'
      role: 'phantom code'
";
    std::fs::write(tmp.path().join(".notarai/phantom.spec.yaml"), spec).unwrap();
    git_commit_all(tmp.path(), "initial");

    // Orphaned globs are now error-severity, so exit code is 1.
    cargo_bin_cmd!("notarai")
        .arg("check")
        .current_dir(tmp.path())
        .assert()
        .code(1)
        .stdout(predicate::str::contains("nonexistent/**/*.rs"));
}

#[test]
fn check_detects_overlapping_coverage() {
    let tmp = TempDir::new().unwrap();
    setup_git_repo(tmp.path());

    std::fs::create_dir_all(tmp.path().join(".notarai")).unwrap();
    std::fs::create_dir_all(tmp.path().join("src")).unwrap();
    std::fs::write(tmp.path().join("src/main.rs"), "fn main() {}").unwrap();

    // Two specs governing the same file.
    let spec_a = "\
schema_version: '0.7'
intent: 'Spec A'
artifacts:
  code:
    - path: 'src/*.rs'
      role: 'source'
";
    let spec_b = "\
schema_version: '0.7'
intent: 'Spec B'
artifacts:
  code:
    - path: 'src/main.rs'
      role: 'entry point'
";
    std::fs::write(tmp.path().join(".notarai/a.spec.yaml"), spec_a).unwrap();
    std::fs::write(tmp.path().join(".notarai/b.spec.yaml"), spec_b).unwrap();
    git_commit_all(tmp.path(), "initial");

    cargo_bin_cmd!("notarai")
        .arg("check")
        .current_dir(tmp.path())
        .assert()
        .code(0)
        .stdout(predicate::str::contains("src/main.rs"))
        .stdout(predicate::str::contains("Overlapping Coverage"));
}

#[test]
fn check_json_output_is_valid() {
    let tmp = TempDir::new().unwrap();
    setup_git_repo(tmp.path());

    std::fs::create_dir_all(tmp.path().join(".notarai")).unwrap();
    std::fs::create_dir_all(tmp.path().join("src")).unwrap();
    std::fs::write(tmp.path().join("src/main.rs"), "fn main() {}").unwrap();
    std::fs::write(tmp.path().join(".notarai/app.spec.yaml"), MINIMAL_SPEC).unwrap();
    std::fs::write(tmp.path().join("ungoverned.txt"), "test").unwrap();
    git_commit_all(tmp.path(), "initial");

    let output = cargo_bin_cmd!("notarai")
        .args(["check", "--format", "json"])
        .current_dir(tmp.path())
        .output()
        .expect("check command");

    // Coverage gap is a warning, exit 0.
    assert!(output.status.success());

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("valid JSON output");
    assert!(json.get("findings").unwrap().is_array());
    assert!(json.get("summary").unwrap().is_object());
    assert!(json["summary"]["warnings"].is_number());
    assert!(json["summary"]["errors"].is_number());

    // Verify finding structure.
    let findings = json["findings"].as_array().unwrap();
    assert!(!findings.is_empty());
    let first = &findings[0];
    assert!(first.get("type").is_some());
    assert!(first.get("severity").is_some());
    assert!(first.get("message").is_some());
}

#[test]
fn check_detects_circular_ref_cycle() {
    let tmp = TempDir::new().unwrap();
    setup_git_repo(tmp.path());

    std::fs::create_dir_all(tmp.path().join(".notarai")).unwrap();

    // a -> b -> a cycle via $ref. Use ./ form (relative to spec parent dir),
    // matching the convention used in the real system.spec.yaml.
    let spec_a = "\
schema_version: '0.7'
intent: 'Spec A'
artifacts:
  code:
    - path: 'src/*.rs'
      role: 'source'
subsystems:
  - $ref: './b.spec.yaml'
";
    let spec_b = "\
schema_version: '0.7'
intent: 'Spec B'
artifacts:
  code:
    - path: 'src/*.rs'
      role: 'source'
dependencies:
  - $ref: './a.spec.yaml'
";
    std::fs::create_dir_all(tmp.path().join("src")).unwrap();
    std::fs::write(tmp.path().join("src/main.rs"), "fn main() {}").unwrap();
    std::fs::write(tmp.path().join(".notarai/a.spec.yaml"), spec_a).unwrap();
    std::fs::write(tmp.path().join(".notarai/b.spec.yaml"), spec_b).unwrap();
    git_commit_all(tmp.path(), "initial");

    let output = cargo_bin_cmd!("notarai")
        .args(["check", "--format", "json"])
        .current_dir(tmp.path())
        .output()
        .expect("check command");

    // Cycle is error-severity, so exit code is 1.
    assert_eq!(output.status.code(), Some(1));

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let findings = json["findings"].as_array().unwrap();
    let cycles: Vec<&serde_json::Value> = findings
        .iter()
        .filter(|f| f["type"] == "circular_ref")
        .collect();
    assert_eq!(cycles.len(), 1, "expected exactly one circular_ref finding");
    assert_eq!(cycles[0]["severity"], "error");
    let msg = cycles[0]["message"].as_str().unwrap();
    assert!(msg.contains("a.spec.yaml") && msg.contains("b.spec.yaml"));
}

#[test]
fn check_detects_behavior_missing_given() {
    let tmp = TempDir::new().unwrap();
    setup_git_repo(tmp.path());

    std::fs::create_dir_all(tmp.path().join(".notarai")).unwrap();
    std::fs::create_dir_all(tmp.path().join("src")).unwrap();
    std::fs::write(tmp.path().join("src/main.rs"), "fn main() {}").unwrap();

    let spec = "\
schema_version: '0.7'
intent: 'Test spec'
behaviors:
  - name: incomplete_behavior
    then: 'something happens'
artifacts:
  code:
    - path: 'src/*.rs'
      role: 'source'
";
    std::fs::write(tmp.path().join(".notarai/app.spec.yaml"), spec).unwrap();
    git_commit_all(tmp.path(), "initial");

    let output = cargo_bin_cmd!("notarai")
        .args(["check", "--format", "json"])
        .current_dir(tmp.path())
        .output()
        .expect("check command");

    // Warning-severity by default, exit 0.
    assert_eq!(output.status.code(), Some(0));

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let findings = json["findings"].as_array().unwrap();
    let incomplete: Vec<&serde_json::Value> = findings
        .iter()
        .filter(|f| f["type"] == "behavior_incomplete")
        .collect();
    assert_eq!(incomplete.len(), 1);
    assert_eq!(incomplete[0]["severity"], "warning");
    let msg = incomplete[0]["message"].as_str().unwrap();
    assert!(msg.contains("incomplete_behavior") && msg.contains("given"));
}

#[test]
fn check_strict_promotes_warnings_to_errors() {
    let tmp = TempDir::new().unwrap();
    setup_git_repo(tmp.path());

    std::fs::create_dir_all(tmp.path().join(".notarai")).unwrap();
    std::fs::create_dir_all(tmp.path().join("src")).unwrap();
    std::fs::write(tmp.path().join("src/main.rs"), "fn main() {}").unwrap();
    std::fs::write(tmp.path().join(".notarai/app.spec.yaml"), MINIMAL_SPEC).unwrap();
    // Coverage gap: file not in any spec, no exclude patterns.
    std::fs::write(tmp.path().join("orphan.txt"), "ungoverned").unwrap();
    git_commit_all(tmp.path(), "initial");

    // Without --strict: warning severity, exit 0.
    cargo_bin_cmd!("notarai")
        .arg("check")
        .current_dir(tmp.path())
        .assert()
        .code(0);

    // With --strict: warning is promoted to error, exit 1.
    let output = cargo_bin_cmd!("notarai")
        .args(["check", "--strict", "--format", "json"])
        .current_dir(tmp.path())
        .output()
        .expect("check --strict");
    assert_eq!(output.status.code(), Some(1));
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let findings = json["findings"].as_array().unwrap();
    let coverage_gaps: Vec<&serde_json::Value> = findings
        .iter()
        .filter(|f| f["type"] == "coverage_gap")
        .collect();
    assert!(!coverage_gaps.is_empty());
    for g in &coverage_gaps {
        assert_eq!(g["severity"], "error");
    }
    assert!(json["summary"]["errors"].as_u64().unwrap() >= 1);
    assert_eq!(json["summary"]["warnings"].as_u64().unwrap(), 0);
}

#[test]
fn check_respects_exclude_patterns() {
    let tmp = TempDir::new().unwrap();
    setup_git_repo(tmp.path());

    std::fs::create_dir_all(tmp.path().join(".notarai")).unwrap();
    std::fs::create_dir_all(tmp.path().join("vendor")).unwrap();
    std::fs::write(tmp.path().join("vendor/lib.js"), "external").unwrap();
    std::fs::write(tmp.path().join(".notarai/system.spec.yaml"), SYSTEM_SPEC).unwrap();
    std::fs::write(tmp.path().join(".notarai/app.spec.yaml"), MINIMAL_SPEC).unwrap();
    std::fs::write(tmp.path().join("config.toml"), "").unwrap();
    std::fs::create_dir_all(tmp.path().join("src")).unwrap();
    std::fs::write(tmp.path().join("src/main.rs"), "fn main() {}").unwrap();
    git_commit_all(tmp.path(), "initial");

    let output = cargo_bin_cmd!("notarai")
        .args(["check", "--format", "json"])
        .current_dir(tmp.path())
        .output()
        .expect("check command");

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let findings = json["findings"].as_array().unwrap();

    // vendor/lib.js should NOT appear as a coverage gap (excluded by system spec).
    let coverage_gaps: Vec<&serde_json::Value> = findings
        .iter()
        .filter(|f| f["type"] == "coverage_gap")
        .collect();
    let gap_files: Vec<&str> = coverage_gaps
        .iter()
        .filter_map(|f| f["file_path"].as_str())
        .collect();
    assert!(
        !gap_files.contains(&"vendor/lib.js"),
        "vendor/lib.js should be excluded by system spec exclude patterns, found in: {gap_files:?}"
    );
}
