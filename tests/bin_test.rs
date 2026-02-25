use assert_cmd::cargo_bin_cmd;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

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

const VALID_SPEC_YAML: &str = "\
schema_version: \"0.4\"
intent: \"Test spec\"
behaviors:
  - name: b1
    given: \"x\"
    then: \"y\"
artifacts:
  code:
    - path: \"src/foo.ts\"
      role: \"test\"
";

#[test]
fn hook_validate_exits_0_for_non_spec_file() {
    notarai()
        .args(["hook", "validate"])
        .write_stdin(r#"{"tool_input":{"file_path":"/tmp/foo.ts"}}"#)
        .assert()
        .success();
}

#[test]
fn hook_validate_exits_0_for_malformed_json() {
    notarai()
        .args(["hook", "validate"])
        .write_stdin("not json!")
        .assert()
        .success();
}

#[test]
fn hook_validate_exits_1_for_invalid_spec() {
    let tmp = TempDir::new().unwrap();
    let spec_dir = tmp.path().join(".notarai");
    fs::create_dir_all(&spec_dir).unwrap();
    let spec_path = spec_dir.join("test.spec.yaml");
    fs::write(&spec_path, "schema_version: \"0.4\"\n").unwrap();

    let input = serde_json::json!({
        "tool_input": { "file_path": spec_path.to_str().unwrap() }
    });
    notarai()
        .args(["hook", "validate"])
        .current_dir(tmp.path())
        .write_stdin(input.to_string())
        .assert()
        .code(1)
        .stderr(predicate::str::contains("Spec validation failed"));
}

#[test]
fn hook_validate_exits_0_for_valid_spec() {
    let tmp = TempDir::new().unwrap();
    let spec_dir = tmp.path().join(".notarai");
    fs::create_dir_all(&spec_dir).unwrap();
    let spec_path = spec_dir.join("test.spec.yaml");
    fs::write(&spec_path, VALID_SPEC_YAML).unwrap();

    let input = serde_json::json!({
        "tool_input": { "file_path": spec_path.to_str().unwrap() }
    });
    notarai()
        .args(["hook", "validate"])
        .current_dir(tmp.path())
        .write_stdin(input.to_string())
        .assert()
        .success();
}

#[test]
fn validate_exits_0_with_warning_for_empty_spec_dir() {
    let tmp = TempDir::new().unwrap();
    let spec_dir = tmp.path().join(".notarai");
    fs::create_dir_all(&spec_dir).unwrap();

    notarai()
        .args(["validate", spec_dir.to_str().unwrap()])
        .assert()
        .success()
        .stderr(predicate::str::contains("no .spec.yaml files"));
}

#[test]
fn validate_warns_when_local_schema_is_stale() {
    let tmp = TempDir::new().unwrap();

    let spec_dir = tmp.path().join(".notarai");
    fs::create_dir_all(&spec_dir).unwrap();
    fs::write(spec_dir.join("test.spec.yaml"), VALID_SPEC_YAML).unwrap();

    let claude_dir = tmp.path().join(".claude");
    fs::create_dir_all(&claude_dir).unwrap();
    fs::write(
        claude_dir.join("notarai.spec.json"),
        r#"{"$id":"https://notarai.dev/schema/0.3/spec.schema.json"}"#,
    )
    .unwrap();

    notarai()
        .arg("validate")
        .current_dir(tmp.path())
        .assert()
        .success()
        .stderr(predicate::str::contains("out of date"));
}
