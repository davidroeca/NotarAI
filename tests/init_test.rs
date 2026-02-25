use assert_cmd::cargo_bin_cmd;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

fn notarai() -> assert_cmd::Command {
    cargo_bin_cmd!("notarai")
}

fn read_settings(tmp: &TempDir) -> serde_json::Value {
    let content = fs::read_to_string(tmp.path().join(".claude/settings.json")).unwrap();
    serde_json::from_str(&content).unwrap()
}

#[test]
fn creates_claude_dir_when_missing() {
    let tmp = TempDir::new().unwrap();
    notarai()
        .arg("init")
        .current_dir(tmp.path())
        .assert()
        .success();
    assert!(tmp.path().join(".claude").exists());
}

#[test]
fn creates_settings_with_hook() {
    let tmp = TempDir::new().unwrap();
    notarai()
        .arg("init")
        .current_dir(tmp.path())
        .assert()
        .success();

    let settings = read_settings(&tmp);
    let hooks = settings["hooks"]["PostToolUse"].as_array().unwrap();
    assert_eq!(hooks.len(), 1);
    assert_eq!(hooks[0]["matcher"], "Write|Edit");
    assert_eq!(hooks[0]["hooks"][0]["command"], "notarai hook validate");
}

#[test]
fn preserves_existing_settings_keys() {
    let tmp = TempDir::new().unwrap();
    let claude_dir = tmp.path().join(".claude");
    fs::create_dir_all(&claude_dir).unwrap();
    fs::write(
        claude_dir.join("settings.json"),
        r#"{"customKey": "preserved"}"#,
    )
    .unwrap();

    notarai()
        .arg("init")
        .current_dir(tmp.path())
        .assert()
        .success();

    let settings = read_settings(&tmp);
    assert_eq!(settings["customKey"], "preserved");
    assert_eq!(
        settings["hooks"]["PostToolUse"].as_array().unwrap().len(),
        1
    );
}

#[test]
fn idempotent_second_run_no_duplicate_hook() {
    let tmp = TempDir::new().unwrap();
    notarai()
        .arg("init")
        .current_dir(tmp.path())
        .assert()
        .success();
    notarai()
        .arg("init")
        .current_dir(tmp.path())
        .assert()
        .success();

    let settings = read_settings(&tmp);
    assert_eq!(
        settings["hooks"]["PostToolUse"].as_array().unwrap().len(),
        1
    );
}

#[test]
fn creates_claude_md_when_missing() {
    let tmp = TempDir::new().unwrap();
    notarai()
        .arg("init")
        .current_dir(tmp.path())
        .assert()
        .success();

    let claude_md = tmp.path().join("CLAUDE.md");
    assert!(claude_md.exists());
    let content = fs::read_to_string(claude_md).unwrap();
    assert!(content.contains("## NotarAI"));
}

#[test]
fn appends_to_existing_claude_md() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("CLAUDE.md"),
        "# My Project\n\nExisting content.\n",
    )
    .unwrap();

    notarai()
        .arg("init")
        .current_dir(tmp.path())
        .assert()
        .success();

    let content = fs::read_to_string(tmp.path().join("CLAUDE.md")).unwrap();
    assert!(content.contains("# My Project"));
    assert!(content.contains("## NotarAI"));
}

#[test]
fn skips_claude_md_when_notarai_header_present_and_matches() {
    let tmp = TempDir::new().unwrap();
    notarai()
        .arg("init")
        .current_dir(tmp.path())
        .assert()
        .success();

    let original = fs::read_to_string(tmp.path().join("CLAUDE.md")).unwrap();
    notarai()
        .arg("init")
        .current_dir(tmp.path())
        .assert()
        .success();

    let after = fs::read_to_string(tmp.path().join("CLAUDE.md")).unwrap();
    assert_eq!(original, after);
}

#[test]
fn copies_slash_commands() {
    let tmp = TempDir::new().unwrap();
    notarai()
        .arg("init")
        .current_dir(tmp.path())
        .assert()
        .success();

    assert!(tmp
        .path()
        .join(".claude/commands/notarai-reconcile.md")
        .exists());
    assert!(tmp
        .path()
        .join(".claude/commands/notarai-bootstrap.md")
        .exists());
}

#[test]
fn copies_schema() {
    let tmp = TempDir::new().unwrap();
    notarai()
        .arg("init")
        .current_dir(tmp.path())
        .assert()
        .success();

    assert!(tmp.path().join(".claude/notarai.spec.json").exists());
}

#[test]
fn does_not_overwrite_slash_commands_on_rerun() {
    let tmp = TempDir::new().unwrap();
    notarai()
        .arg("init")
        .current_dir(tmp.path())
        .assert()
        .success();

    let reconcile_path = tmp.path().join(".claude/commands/notarai-reconcile.md");
    fs::write(&reconcile_path, "sentinel content").unwrap();

    notarai()
        .arg("init")
        .current_dir(tmp.path())
        .assert()
        .success();

    let content = fs::read_to_string(reconcile_path).unwrap();
    assert_eq!(content, "sentinel content");
}

#[test]
fn warns_on_stderr_when_notarai_section_has_drifted() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("CLAUDE.md"),
        "## NotarAI\n\nThis is outdated content that differs from the bundled template.\n",
    )
    .unwrap();

    notarai()
        .arg("init")
        .current_dir(tmp.path())
        .assert()
        .success()
        .stderr(predicate::str::contains("drifted"));
}

#[test]
fn always_overwrites_schema_on_rerun() {
    let tmp = TempDir::new().unwrap();
    notarai()
        .arg("init")
        .current_dir(tmp.path())
        .assert()
        .success();

    let schema_path = tmp.path().join(".claude/notarai.spec.json");
    fs::write(&schema_path, "{}").unwrap();
    notarai()
        .arg("init")
        .current_dir(tmp.path())
        .assert()
        .success();

    let content = fs::read_to_string(schema_path).unwrap();
    assert_ne!(content, "{}");
}
