use assert_cmd::cargo_bin_cmd;
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
        .args(["init", "--agent", "claude"])
        .current_dir(tmp.path())
        .assert()
        .success();
    assert!(tmp.path().join(".claude").exists());
}

#[test]
fn creates_settings_with_hook() {
    let tmp = TempDir::new().unwrap();
    notarai()
        .args(["init", "--agent", "claude"])
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
        .args(["init", "--agent", "claude"])
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
        .args(["init", "--agent", "claude"])
        .current_dir(tmp.path())
        .assert()
        .success();
    notarai()
        .args(["init", "--agent", "claude"])
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
        .args(["init", "--agent", "claude"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let claude_md = tmp.path().join("CLAUDE.md");
    assert!(claude_md.exists());
    let content = fs::read_to_string(claude_md).unwrap();
    assert!(content.contains("## NotarAI"));
    assert!(content.contains("/notarai-reconcile"));
    assert!(content.contains("notarai validate"));
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
        .args(["init", "--agent", "claude"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let content = fs::read_to_string(tmp.path().join("CLAUDE.md")).unwrap();
    assert!(content.contains("# My Project"));
    assert!(content.contains("## NotarAI"));
}

#[test]
fn replaces_existing_notarai_section_in_claude_md() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("CLAUDE.md"),
        "## NotarAI\n\nThis is outdated content.\n",
    )
    .unwrap();

    notarai()
        .args(["init", "--agent", "claude"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let content = fs::read_to_string(tmp.path().join("CLAUDE.md")).unwrap();
    // Outdated content is replaced
    assert!(!content.contains("This is outdated content."));
    // New section present
    assert!(content.contains("/notarai-reconcile"));
    assert!(content.contains("notarai validate"));
}

#[test]
fn replaces_notarai_section_on_second_run() {
    let tmp = TempDir::new().unwrap();
    notarai()
        .args(["init", "--agent", "claude"])
        .current_dir(tmp.path())
        .assert()
        .success();

    notarai()
        .args(["init", "--agent", "claude"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let content = fs::read_to_string(tmp.path().join("CLAUDE.md")).unwrap();
    // Section still present, not duplicated
    let count = content.matches("## NotarAI").count();
    assert_eq!(count, 1);
}

#[test]
fn copies_slash_commands() {
    let tmp = TempDir::new().unwrap();
    notarai()
        .args(["init", "--agent", "claude"])
        .current_dir(tmp.path())
        .assert()
        .success();

    assert!(
        tmp.path()
            .join(".claude/skills/notarai-reconcile/SKILL.md")
            .exists()
    );
    assert!(
        tmp.path()
            .join(".claude/skills/notarai-bootstrap/SKILL.md")
            .exists()
    );
}

#[test]
fn always_overwrites_slash_commands_on_rerun() {
    let tmp = TempDir::new().unwrap();
    notarai()
        .args(["init", "--agent", "claude"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let reconcile_path = tmp.path().join(".claude/skills/notarai-reconcile/SKILL.md");
    fs::write(&reconcile_path, "sentinel content").unwrap();

    notarai()
        .args(["init", "--agent", "claude"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let content = fs::read_to_string(&reconcile_path).unwrap();
    // Sentinel was overwritten
    assert_ne!(content, "sentinel content");
}

#[test]
fn copies_schema_to_notarai_dir() {
    let tmp = TempDir::new().unwrap();
    notarai()
        .args(["init", "--agent", "claude"])
        .current_dir(tmp.path())
        .assert()
        .success();

    assert!(tmp.path().join(".notarai/notarai.spec.json").exists());
}

#[test]
fn always_overwrites_schema_on_rerun() {
    let tmp = TempDir::new().unwrap();
    notarai()
        .args(["init", "--agent", "claude"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let schema_path = tmp.path().join(".notarai/notarai.spec.json");
    fs::write(&schema_path, "{}").unwrap();
    notarai()
        .args(["init", "--agent", "claude"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let content = fs::read_to_string(schema_path).unwrap();
    assert_ne!(content, "{}");
}

#[test]
fn writes_notarai_readme() {
    let tmp = TempDir::new().unwrap();
    notarai()
        .args(["init", "--agent", "claude"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let readme = tmp.path().join(".notarai/README.md");
    assert!(readme.exists());
    let content = fs::read_to_string(readme).unwrap();
    assert!(content.contains("# NotarAI"));
    assert!(content.contains("notarai validate"));
}

#[test]
fn gitignore_entry_added() {
    let tmp = TempDir::new().unwrap();
    notarai()
        .args(["init", "--agent", "claude"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let gitignore = fs::read_to_string(tmp.path().join(".gitignore")).unwrap();
    assert!(gitignore.contains(".notarai/.cache/"));
}

#[test]
fn mcp_json_created() {
    let tmp = TempDir::new().unwrap();
    notarai()
        .args(["init", "--agent", "claude"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let mcp = tmp.path().join(".mcp.json");
    assert!(mcp.exists());
    let content = fs::read_to_string(mcp).unwrap();
    assert!(content.contains("notarai"));
    assert!(content.contains("mcp"));
}

// --- Generic mode tests ---

#[test]
fn init_generic_creates_agents_md() {
    let tmp = TempDir::new().unwrap();
    notarai()
        .args(["init", "--agent", "generic"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let agents_md = tmp.path().join("AGENTS.md");
    assert!(agents_md.exists());
    let content = fs::read_to_string(agents_md).unwrap();
    assert!(content.contains("NotarAI"));
    assert!(content.contains("export-context"));
}

#[test]
fn init_generic_creates_reconcile_prompt() {
    let tmp = TempDir::new().unwrap();
    notarai()
        .args(["init", "--agent", "generic"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let prompt = tmp.path().join(".notarai/reconcile-prompt.md");
    assert!(prompt.exists());
    let content = fs::read_to_string(prompt).unwrap();
    assert!(content.contains("spec_content"));
    assert!(content.contains("base_branch"));
}

#[test]
fn init_generic_no_claude_dir() {
    let tmp = TempDir::new().unwrap();
    notarai()
        .args(["init", "--agent", "generic"])
        .current_dir(tmp.path())
        .assert()
        .success();

    assert!(!tmp.path().join(".claude").exists());
    assert!(!tmp.path().join("CLAUDE.md").exists());
}

#[test]
fn init_generic_creates_mcp_and_gitignore() {
    let tmp = TempDir::new().unwrap();
    notarai()
        .args(["init", "--agent", "generic"])
        .current_dir(tmp.path())
        .assert()
        .success();

    assert!(tmp.path().join(".mcp.json").exists());
    let gitignore = fs::read_to_string(tmp.path().join(".gitignore")).unwrap();
    assert!(gitignore.contains(".notarai/.cache/"));
}

#[test]
fn init_generic_creates_schema() {
    let tmp = TempDir::new().unwrap();
    notarai()
        .args(["init", "--agent", "generic"])
        .current_dir(tmp.path())
        .assert()
        .success();

    assert!(tmp.path().join(".notarai/notarai.spec.json").exists());
    assert!(tmp.path().join(".notarai/README.md").exists());
}

#[test]
fn init_claude_backward_compatible() {
    let tmp = TempDir::new().unwrap();
    notarai()
        .args(["init", "--agent", "claude"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // All Claude-specific files should exist.
    assert!(tmp.path().join(".claude/settings.json").exists());
    assert!(
        tmp.path()
            .join(".claude/skills/notarai-reconcile/SKILL.md")
            .exists()
    );
    assert!(tmp.path().join("CLAUDE.md").exists());
    assert!(tmp.path().join(".mcp.json").exists());
    assert!(tmp.path().join(".notarai/notarai.spec.json").exists());

    // Generic-specific files should NOT exist.
    assert!(!tmp.path().join("AGENTS.md").exists());
    assert!(!tmp.path().join(".notarai/reconcile-prompt.md").exists());
}
