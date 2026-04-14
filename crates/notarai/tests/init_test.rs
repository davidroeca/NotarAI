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

// -----------------------------------------------------------------
// Always-installed artifacts (agent-agnostic)
// -----------------------------------------------------------------

#[test]
fn always_installs_agents_md_and_notarai_dir() {
    let tmp = TempDir::new().unwrap();
    notarai()
        .args(["init", "--agents", "none"])
        .current_dir(tmp.path())
        .assert()
        .success();

    assert!(tmp.path().join("AGENTS.md").exists());
    assert!(tmp.path().join(".notarai/notarai.spec.json").exists());
    assert!(tmp.path().join(".notarai/README.md").exists());
    assert!(tmp.path().join(".notarai/reconcile-prompt.md").exists());
    assert!(tmp.path().join(".notarai/bootstrap-prompt.md").exists());
    assert!(tmp.path().join(".mcp.json").exists());

    let gitignore = fs::read_to_string(tmp.path().join(".gitignore")).unwrap();
    assert!(gitignore.contains(".notarai/.cache/"));
}

#[test]
fn agents_md_contains_notarai_section() {
    let tmp = TempDir::new().unwrap();
    notarai()
        .args(["init", "--agents", "none"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let content = fs::read_to_string(tmp.path().join("AGENTS.md")).unwrap();
    assert!(content.contains("## NotarAI"));
    assert!(content.contains("notarai validate"));
    assert!(content.contains("export-context"));
}

#[test]
fn agents_md_section_merge_preserves_user_content() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("AGENTS.md"),
        "# My Project\n\n## Intro\n\nMy prose here.\n",
    )
    .unwrap();

    notarai()
        .args(["init", "--agents", "none"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let content = fs::read_to_string(tmp.path().join("AGENTS.md")).unwrap();
    assert!(content.contains("# My Project"));
    assert!(content.contains("My prose here."));
    assert!(content.contains("## NotarAI"));
}

#[test]
fn none_selection_installs_no_agent_artifacts() {
    let tmp = TempDir::new().unwrap();
    notarai()
        .args(["init", "--agents", "none"])
        .current_dir(tmp.path())
        .assert()
        .success();

    assert!(!tmp.path().join("CLAUDE.md").exists());
    assert!(!tmp.path().join("GEMINI.md").exists());
    assert!(!tmp.path().join(".claude").exists());
    assert!(!tmp.path().join(".gemini").exists());
    assert!(!tmp.path().join(".codex").exists());
    assert!(!tmp.path().join(".opencode").exists());
}

// -----------------------------------------------------------------
// Claude adapter
// -----------------------------------------------------------------

#[test]
fn claude_creates_pointer_and_skills_and_hook() {
    let tmp = TempDir::new().unwrap();
    notarai()
        .args(["init", "--agents", "claude"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Pointer file imports AGENTS.md.
    let claude_md = fs::read_to_string(tmp.path().join("CLAUDE.md")).unwrap();
    assert!(claude_md.contains("@AGENTS.md"));

    // Skills installed.
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

    // Hook configured.
    let settings = read_settings(&tmp);
    let hooks = settings["hooks"]["PostToolUse"].as_array().unwrap();
    assert_eq!(hooks.len(), 1);
    assert_eq!(hooks[0]["hooks"][0]["command"], "notarai hook validate");
}

#[test]
fn claude_pointer_stub_created_when_absent() {
    let tmp = TempDir::new().unwrap();
    notarai()
        .args(["init", "--agents", "claude"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let content = fs::read_to_string(tmp.path().join("CLAUDE.md")).unwrap();
    assert_eq!(content.trim(), "@AGENTS.md");
}

#[test]
fn claude_pointer_existing_with_import_left_alone() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("CLAUDE.md"), "# My Claude\n\n@AGENTS.md\n").unwrap();

    notarai()
        .args(["init", "--agents", "claude"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let content = fs::read_to_string(tmp.path().join("CLAUDE.md")).unwrap();
    assert_eq!(content, "# My Claude\n\n@AGENTS.md\n");
}

#[test]
fn claude_pointer_merges_section_when_existing_has_no_import() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("CLAUDE.md"),
        "# Claude Rules\n\nHand-tuned prose.\n",
    )
    .unwrap();

    notarai()
        .args(["init", "--agents", "claude"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let content = fs::read_to_string(tmp.path().join("CLAUDE.md")).unwrap();
    assert!(content.contains("# Claude Rules"));
    assert!(content.contains("Hand-tuned prose."));
    assert!(content.contains("## NotarAI"));
    assert!(content.contains("@AGENTS.md"));
}

#[test]
fn claude_idempotent_second_run_no_duplicate_hook() {
    let tmp = TempDir::new().unwrap();
    notarai()
        .args(["init", "--agents", "claude"])
        .current_dir(tmp.path())
        .assert()
        .success();
    notarai()
        .args(["init", "--agents", "claude"])
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
fn claude_preserves_other_settings_keys() {
    let tmp = TempDir::new().unwrap();
    let claude_dir = tmp.path().join(".claude");
    fs::create_dir_all(&claude_dir).unwrap();
    fs::write(
        claude_dir.join("settings.json"),
        r#"{"customKey": "preserved"}"#,
    )
    .unwrap();

    notarai()
        .args(["init", "--agents", "claude"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let settings = read_settings(&tmp);
    assert_eq!(settings["customKey"], "preserved");
}

#[test]
fn claude_unparseable_settings_is_hard_error() {
    let tmp = TempDir::new().unwrap();
    let claude_dir = tmp.path().join(".claude");
    fs::create_dir_all(&claude_dir).unwrap();
    fs::write(claude_dir.join("settings.json"), "not json{{{").unwrap();

    notarai()
        .args(["init", "--agents", "claude"])
        .current_dir(tmp.path())
        .assert()
        .failure();
}

#[test]
fn claude_overwrites_skills_on_rerun() {
    let tmp = TempDir::new().unwrap();
    notarai()
        .args(["init", "--agents", "claude"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let skill = tmp.path().join(".claude/skills/notarai-reconcile/SKILL.md");
    fs::write(&skill, "sentinel").unwrap();

    notarai()
        .args(["init", "--agents", "claude"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let content = fs::read_to_string(&skill).unwrap();
    assert_ne!(content, "sentinel");
}

// -----------------------------------------------------------------
// Gemini adapter
// -----------------------------------------------------------------

#[test]
fn gemini_creates_pointer_and_skills_no_hook() {
    let tmp = TempDir::new().unwrap();
    notarai()
        .args(["init", "--agents", "gemini"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let gemini_md = fs::read_to_string(tmp.path().join("GEMINI.md")).unwrap();
    assert!(gemini_md.contains("@AGENTS.md"));

    assert!(
        tmp.path()
            .join(".gemini/skills/notarai-reconcile/SKILL.md")
            .exists()
    );
    assert!(
        tmp.path()
            .join(".gemini/skills/notarai-bootstrap/SKILL.md")
            .exists()
    );

    // No Claude artifacts.
    assert!(!tmp.path().join("CLAUDE.md").exists());
    assert!(!tmp.path().join(".claude").exists());
}

// -----------------------------------------------------------------
// Codex adapter
// -----------------------------------------------------------------

#[test]
fn codex_installs_skills_no_pointer_file() {
    let tmp = TempDir::new().unwrap();
    notarai()
        .args(["init", "--agents", "codex"])
        .current_dir(tmp.path())
        .assert()
        .success();

    assert!(
        tmp.path()
            .join(".codex/skills/notarai-reconcile/SKILL.md")
            .exists()
    );
    assert!(
        tmp.path()
            .join(".codex/skills/notarai-bootstrap/SKILL.md")
            .exists()
    );
    // Codex has no pointer file; it reads AGENTS.md natively.
    assert!(!tmp.path().join("CODEX.md").exists());
}

// -----------------------------------------------------------------
// Opencode adapter
// -----------------------------------------------------------------

#[test]
fn opencode_installs_skills_no_pointer_file() {
    let tmp = TempDir::new().unwrap();
    notarai()
        .args(["init", "--agents", "opencode"])
        .current_dir(tmp.path())
        .assert()
        .success();

    assert!(
        tmp.path()
            .join(".opencode/skills/notarai-reconcile/SKILL.md")
            .exists()
    );
    assert!(
        tmp.path()
            .join(".opencode/skills/notarai-bootstrap/SKILL.md")
            .exists()
    );
    assert!(!tmp.path().join("OPENCODE.md").exists());
    assert!(!tmp.path().join(".claude").exists());
}

// -----------------------------------------------------------------
// Multi-agent + all
// -----------------------------------------------------------------

#[test]
fn multi_agent_side_by_side() {
    let tmp = TempDir::new().unwrap();
    notarai()
        .args(["init", "--agents", "claude,gemini"])
        .current_dir(tmp.path())
        .assert()
        .success();

    assert!(tmp.path().join("CLAUDE.md").exists());
    assert!(tmp.path().join("GEMINI.md").exists());
    assert!(tmp.path().join(".claude/skills").exists());
    assert!(tmp.path().join(".gemini/skills").exists());
}

#[test]
fn all_installs_every_adapter() {
    let tmp = TempDir::new().unwrap();
    notarai()
        .args(["init", "--agents", "all"])
        .current_dir(tmp.path())
        .assert()
        .success();

    assert!(tmp.path().join("CLAUDE.md").exists());
    assert!(tmp.path().join("GEMINI.md").exists());
    assert!(
        tmp.path()
            .join(".claude/skills/notarai-reconcile/SKILL.md")
            .exists()
    );
    assert!(
        tmp.path()
            .join(".gemini/skills/notarai-reconcile/SKILL.md")
            .exists()
    );
    assert!(
        tmp.path()
            .join(".codex/skills/notarai-reconcile/SKILL.md")
            .exists()
    );
    assert!(
        tmp.path()
            .join(".opencode/skills/notarai-reconcile/SKILL.md")
            .exists()
    );
}

// -----------------------------------------------------------------
// Validation errors
// -----------------------------------------------------------------

#[test]
fn unknown_agent_is_hard_error() {
    let tmp = TempDir::new().unwrap();
    notarai()
        .args(["init", "--agents", "bogus"])
        .current_dir(tmp.path())
        .assert()
        .failure();
}

// -----------------------------------------------------------------
// Back-compat: --agent alias
// -----------------------------------------------------------------

#[test]
fn agent_alias_claude_still_works() {
    let tmp = TempDir::new().unwrap();
    notarai()
        .args(["init", "--agent", "claude"])
        .current_dir(tmp.path())
        .assert()
        .success();

    assert!(tmp.path().join("CLAUDE.md").exists());
    assert!(tmp.path().join(".claude/settings.json").exists());
}

#[test]
fn agent_alias_generic_maps_to_opencode() {
    let tmp = TempDir::new().unwrap();
    notarai()
        .args(["init", "--agent", "generic"])
        .current_dir(tmp.path())
        .assert()
        .success();

    assert!(
        tmp.path()
            .join(".opencode/skills/notarai-reconcile/SKILL.md")
            .exists()
    );
    // Should NOT install Claude artifacts.
    assert!(!tmp.path().join(".claude").exists());
    assert!(!tmp.path().join("CLAUDE.md").exists());
}

// -----------------------------------------------------------------
// Auto-detect on non-TTY
// -----------------------------------------------------------------

#[test]
fn autodetect_installs_for_existing_gemini_dir() {
    let tmp = TempDir::new().unwrap();
    fs::create_dir_all(tmp.path().join(".gemini")).unwrap();

    notarai()
        .args(["init"])
        .current_dir(tmp.path())
        .assert()
        .success();

    assert!(
        tmp.path()
            .join(".gemini/skills/notarai-reconcile/SKILL.md")
            .exists()
    );
    // Should not install claude since no claude markers exist.
    assert!(!tmp.path().join(".claude").exists());
}

#[test]
fn autodetect_falls_back_to_claude_when_empty() {
    let tmp = TempDir::new().unwrap();
    notarai()
        .args(["init"])
        .current_dir(tmp.path())
        .assert()
        .success();

    assert!(tmp.path().join(".claude/settings.json").exists());
    assert!(tmp.path().join("CLAUDE.md").exists());
}

// -----------------------------------------------------------------
// MCP json preservation
// -----------------------------------------------------------------

#[test]
fn mcp_json_preserves_other_servers() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join(".mcp.json"),
        r#"{"mcpServers":{"other":{"type":"stdio","command":"other"}}}"#,
    )
    .unwrap();

    notarai()
        .args(["init", "--agents", "none"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let content = fs::read_to_string(tmp.path().join(".mcp.json")).unwrap();
    assert!(content.contains("\"other\""));
    assert!(content.contains("\"notarai\""));
}

#[test]
fn mcp_json_unparseable_is_hard_error() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join(".mcp.json"), "not json{").unwrap();

    notarai()
        .args(["init", "--agents", "none"])
        .current_dir(tmp.path())
        .assert()
        .failure();
}
