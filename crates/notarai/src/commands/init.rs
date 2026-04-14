use std::fs;
use std::path::{Path, PathBuf};

const HOOK_COMMAND: &str = "notarai hook validate";

// Claude-flavor skills (use Claude-specific tool names).
const CLAUDE_RECONCILE_MD: &str = include_str!("../../skills/notarai-reconcile/SKILL.md");
const CLAUDE_BOOTSTRAP_MD: &str = include_str!("../../skills/notarai-bootstrap/SKILL.md");

// Generic-flavor skills (tool-agnostic phrasing for gemini/codex/opencode).
const GENERIC_RECONCILE_MD: &str = include_str!("../../skills-generic/notarai-reconcile/SKILL.md");
const GENERIC_BOOTSTRAP_MD: &str = include_str!("../../skills-generic/notarai-bootstrap/SKILL.md");

const NOTARAI_README_TEMPLATE: &str = include_str!("../../templates/notarai-readme.md");
const SCHEMA_JSON: &str = include_str!("../../notarai.spec.json");
const AGENTS_MD_SECTION: &str = include_str!("../../templates/agents.md");
const RECONCILE_PROMPT_TEMPLATE: &str = include_str!("../../templates/reconcile-prompt.md");
const BOOTSTRAP_PROMPT_TEMPLATE: &str = include_str!("../../templates/bootstrap-prompt.md");

/// The section written to a pointer file (CLAUDE.md, GEMINI.md) when it already
/// has other content but no `@AGENTS.md` import.
const POINTER_SECTION: &str = "## NotarAI\n\nSee @AGENTS.md for NotarAI context.\n";

/// Flavor of skill content to install in an agent's skills directory.
#[derive(Debug, Clone, Copy, PartialEq)]
enum SkillFlavor {
    Claude,
    Generic,
}

impl SkillFlavor {
    fn reconcile(self) -> &'static str {
        match self {
            SkillFlavor::Claude => CLAUDE_RECONCILE_MD,
            SkillFlavor::Generic => GENERIC_RECONCILE_MD,
        }
    }
    fn bootstrap(self) -> &'static str {
        match self {
            SkillFlavor::Claude => CLAUDE_BOOTSTRAP_MD,
            SkillFlavor::Generic => GENERIC_BOOTSTRAP_MD,
        }
    }
}

/// Per-agent install behavior. Adding a new agent = one entry in ADAPTERS.
#[derive(Debug, Clone, Copy)]
struct AgentAdapter {
    name: &'static str,
    /// Optional top-level pointer markdown file (e.g. CLAUDE.md, GEMINI.md)
    /// that will import `@AGENTS.md`. `None` for agents that read AGENTS.md
    /// natively.
    pointer_file: Option<&'static str>,
    /// Optional agent-specific skills directory (e.g. `.claude/skills`).
    skills_dir: Option<&'static str>,
    /// Optional agent-specific hook installer.
    install_hook: Option<fn(&Path) -> Result<(), String>>,
    skill_flavor: SkillFlavor,
}

const ADAPTERS: &[AgentAdapter] = &[
    AgentAdapter {
        name: "claude",
        pointer_file: Some("CLAUDE.md"),
        skills_dir: Some(".claude/skills"),
        install_hook: Some(install_claude_hook),
        skill_flavor: SkillFlavor::Claude,
    },
    AgentAdapter {
        name: "gemini",
        pointer_file: Some("GEMINI.md"),
        skills_dir: Some(".gemini/skills"),
        install_hook: None,
        skill_flavor: SkillFlavor::Generic,
    },
    AgentAdapter {
        name: "codex",
        pointer_file: None,
        skills_dir: Some(".codex/skills"),
        install_hook: None,
        skill_flavor: SkillFlavor::Generic,
    },
    AgentAdapter {
        name: "opencode",
        pointer_file: None,
        skills_dir: Some(".opencode/skills"),
        install_hook: None,
        skill_flavor: SkillFlavor::Generic,
    },
];

fn adapter_by_name(name: &str) -> Option<&'static AgentAdapter> {
    ADAPTERS.iter().find(|a| a.name == name)
}

fn known_agent_names() -> String {
    ADAPTERS
        .iter()
        .map(|a| a.name)
        .collect::<Vec<_>>()
        .join(", ")
}

/// Parse user-supplied agent names into adapters. `all` expands to every
/// known adapter; `none` returns an empty selection. Unknown names return Err.
fn resolve_agents(raw: &[String]) -> Result<Vec<&'static AgentAdapter>, String> {
    let mut out: Vec<&'static AgentAdapter> = Vec::new();

    for item in raw {
        let item = item.trim();
        if item.is_empty() {
            continue;
        }
        match item {
            "all" => {
                for a in ADAPTERS {
                    if !out.iter().any(|x| x.name == a.name) {
                        out.push(a);
                    }
                }
            }
            "none" => {
                // `none` short-circuits: caller should pass only ["none"].
                // If mixed with other tokens, "none" simply contributes nothing.
            }
            "generic" => {
                // Back-compat alias: --agent generic maps to opencode
                // (AGENTS.md + .opencode/skills, no pointer file, no hook).
                let a = adapter_by_name("opencode").expect("opencode adapter");
                if !out.iter().any(|x| x.name == a.name) {
                    out.push(a);
                }
            }
            other => match adapter_by_name(other) {
                Some(a) => {
                    if !out.iter().any(|x| x.name == a.name) {
                        out.push(a);
                    }
                }
                None => {
                    return Err(format!(
                        "unknown agent '{other}'. Known: {} (or 'all', 'none', 'generic').",
                        known_agent_names()
                    ));
                }
            },
        }
    }

    Ok(out)
}

/// Auto-detect installed agents by scanning for their pointer files and
/// skills-dir parents. Returns claude as the sole fallback if nothing is
/// detected.
fn autodetect_agents(project_root: &Path) -> Vec<&'static AgentAdapter> {
    let mut out: Vec<&'static AgentAdapter> = Vec::new();
    for a in ADAPTERS {
        let mut found = false;
        if let Some(p) = a.pointer_file
            && project_root.join(p).exists()
        {
            found = true;
        }
        if let Some(d) = a.skills_dir {
            // Detect the agent root dir (e.g., `.claude` for `.claude/skills`).
            let root = Path::new(d).components().next().map(|c| c.as_os_str());
            if let Some(root) = root
                && project_root.join(root).exists()
            {
                found = true;
            }
        }
        if found {
            out.push(a);
        }
    }
    if out.is_empty() {
        out.push(adapter_by_name("claude").expect("claude adapter"));
    }
    out
}

/// Replace the `## NotarAI` section in `content` with `new_section`.
///
/// The section spans from the `## NotarAI` line up to (but not including) the
/// next `## ` heading, or EOF. If no section is found, `content` is returned
/// unchanged.
pub fn replace_notarai_section(content: &str, new_section: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();

    let start = match lines.iter().position(|&l| l == "## NotarAI") {
        Some(i) => i,
        None => return content.to_string(),
    };

    let end = lines[start + 1..]
        .iter()
        .position(|l| l.starts_with("## "))
        .map(|i| start + 1 + i)
        .unwrap_or(lines.len());

    let before_lines = &lines[..start];
    let after_lines = &lines[end..];

    let mut result = String::new();

    for line in before_lines {
        result.push_str(line);
        result.push('\n');
    }

    result.push_str(new_section);
    if !new_section.ends_with('\n') {
        result.push('\n');
    }

    if !after_lines.is_empty() {
        for line in after_lines {
            result.push_str(line);
            result.push('\n');
        }
    }

    result
}

/// Write or merge a `## NotarAI` section into a markdown file.
fn upsert_notarai_section(path: &Path, section: &str, label: &str) -> Result<(), String> {
    if path.exists() {
        let existing =
            fs::read_to_string(path).map_err(|e| format!("could not read {label}: {e}"))?;
        let has_section = existing.lines().any(|line| line == "## NotarAI");
        let new_content = if has_section {
            replace_notarai_section(&existing, section)
        } else {
            let mut content = existing;
            if !content.is_empty() && !content.ends_with('\n') {
                content.push('\n');
            }
            content.push('\n');
            content.push_str(section);
            content
        };
        fs::write(path, new_content).map_err(|e| format!("could not write {label}: {e}"))?;
        if has_section {
            println!("Updated NotarAI section in {label}");
        } else {
            println!("Added NotarAI section to {label}");
        }
    } else {
        fs::write(path, section).map_err(|e| format!("could not write {label}: {e}"))?;
        println!("Wrote {label}");
    }
    Ok(())
}

fn has_notarai_hook(matchers: &[serde_json::Value]) -> bool {
    matchers.iter().any(|m| {
        m.get("hooks")
            .and_then(|h| h.as_array())
            .is_some_and(|hooks| {
                hooks
                    .iter()
                    .any(|h| h.get("command").and_then(|c| c.as_str()) == Some(HOOK_COMMAND))
            })
    })
}

/// Prompt the user for agent selection via stdin. Accepts a comma-separated
/// list; empty input defaults to claude.
fn prompt_agent_choice() -> Result<Vec<String>, String> {
    eprint!(
        "Which agents? (comma-separated: {}; also 'all' or 'none') [claude]: ",
        known_agent_names()
    );
    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .map_err(|e| format!("could not read stdin: {e}"))?;
    let trimmed = input.trim();
    if trimmed.is_empty() {
        Ok(vec!["claude".to_string()])
    } else {
        Ok(trimmed.split(',').map(|s| s.trim().to_string()).collect())
    }
}

/// Set up NotarAI in the target project directory.
///
/// `agents_raw`: user-supplied agent tokens (post CLI parsing). `None` means
/// no selection was provided; `run` will prompt on TTY or auto-detect on
/// non-TTY.
pub fn run(project_root: Option<&Path>, agents_raw: Option<Vec<String>>) -> i32 {
    let root = match project_root {
        Some(p) => p.to_path_buf(),
        None => std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf()),
    };

    match run_inner(&root, agents_raw) {
        Ok(()) => {
            crate::commands::update::passive_update_hint();
            0
        }
        Err(e) => {
            eprintln!("Error: {e}");
            1
        }
    }
}

fn run_inner(root: &Path, agents_raw: Option<Vec<String>>) -> Result<(), String> {
    // Resolve agent selection.
    use std::io::IsTerminal;
    let adapters: Vec<&AgentAdapter> = match agents_raw {
        Some(raw) => {
            // `none` must be explicit to produce empty selection.
            if raw.iter().any(|t| t.trim() == "none") {
                Vec::new()
            } else {
                resolve_agents(&raw)?
            }
        }
        None if std::io::stdin().is_terminal() => {
            let raw = prompt_agent_choice()?;
            resolve_agents(&raw)?
        }
        None => autodetect_agents(root),
    };

    let notarai_dir = root.join(".notarai");
    if !notarai_dir.exists() {
        fs::create_dir_all(&notarai_dir)
            .map_err(|e| format!("could not create .notarai/ directory: {e}"))?;
    }

    // Always-installed, agent-agnostic artifacts.
    setup_schema(&notarai_dir)?;
    setup_notarai_readme(&notarai_dir)?;
    setup_reconcile_prompt(&notarai_dir)?;
    setup_bootstrap_prompt(&notarai_dir)?;
    setup_agents_md(root)?;
    setup_gitignore(root)?;
    setup_mcp_json(root)?;

    // Per-agent install.
    for adapter in &adapters {
        install_adapter(root, adapter)?;
    }

    if adapters.is_empty() {
        println!("No agent adapters selected; only AGENTS.md + .notarai/ installed.");
    }

    Ok(())
}

fn install_adapter(root: &Path, adapter: &AgentAdapter) -> Result<(), String> {
    println!("Installing adapter: {}", adapter.name);

    if let Some(pointer) = adapter.pointer_file {
        install_pointer_file(root, pointer)?;
    }

    if let Some(skills_dir) = adapter.skills_dir {
        install_skills(root, skills_dir, adapter.skill_flavor)?;
    }

    if let Some(hook) = adapter.install_hook {
        hook(root)?;
    }

    Ok(())
}

/// Create a pointer file (CLAUDE.md, GEMINI.md) that imports `@AGENTS.md`.
///
/// Behavior:
/// - File is a symlink → error (unexpected state; user must resolve).
/// - File absent → write `@AGENTS.md\n`.
/// - File exists and contains `@AGENTS.md` anywhere → leave alone.
/// - File exists with other content → merge a `## NotarAI` section pointing at AGENTS.md.
fn install_pointer_file(root: &Path, name: &str) -> Result<(), String> {
    let path = root.join(name);

    // Reject symlinks outright to avoid clobbering or following into surprises.
    let meta = fs::symlink_metadata(&path);
    if let Ok(m) = &meta
        && m.file_type().is_symlink()
    {
        return Err(format!(
            "{name} is a symlink. Resolve manually (delete or repoint) and re-run."
        ));
    }

    if !path.exists() {
        fs::write(&path, "@AGENTS.md\n").map_err(|e| format!("could not write {name}: {e}"))?;
        println!("Wrote {name} (-> @AGENTS.md)");
        return Ok(());
    }

    let existing = fs::read_to_string(&path).map_err(|e| format!("could not read {name}: {e}"))?;

    if existing.contains("@AGENTS.md") {
        println!("{name} already imports @AGENTS.md");
        return Ok(());
    }

    upsert_notarai_section(&path, POINTER_SECTION, name)?;
    Ok(())
}

/// Write `<skills_dir>/notarai-reconcile/SKILL.md` and
/// `<skills_dir>/notarai-bootstrap/SKILL.md`, always overwriting.
fn install_skills(root: &Path, skills_dir: &str, flavor: SkillFlavor) -> Result<(), String> {
    let base: PathBuf = root.join(skills_dir);

    // Hard error if a non-directory blocks the path.
    if base.exists() && !base.is_dir() {
        return Err(format!(
            "{skills_dir} exists but is not a directory; remove it and re-run"
        ));
    }

    write_skill(&base, "notarai-reconcile", flavor.reconcile())?;
    write_skill(&base, "notarai-bootstrap", flavor.bootstrap())?;
    Ok(())
}

fn write_skill(base: &Path, name: &str, content: &str) -> Result<(), String> {
    let dir = base.join(name);
    fs::create_dir_all(&dir).map_err(|e| format!("could not create {}: {e}", dir.display()))?;
    let dest = dir.join("SKILL.md");
    fs::write(&dest, content).map_err(|e| format!("could not write {}: {e}", dest.display()))?;
    println!("Wrote {}", dest.display());
    Ok(())
}

fn setup_schema(notarai_dir: &Path) -> Result<(), String> {
    let dest = notarai_dir.join("notarai.spec.json");
    fs::write(&dest, SCHEMA_JSON).map_err(|e| format!("could not write notarai.spec.json: {e}"))?;
    println!("Copied schema to .notarai/notarai.spec.json");
    Ok(())
}

fn setup_notarai_readme(notarai_dir: &Path) -> Result<(), String> {
    let version = env!("CARGO_PKG_VERSION");
    let content = NOTARAI_README_TEMPLATE.replace("{{VERSION}}", version);
    let dest = notarai_dir.join("README.md");
    fs::write(&dest, content).map_err(|e| format!("could not write .notarai/README.md: {e}"))?;
    println!("Wrote .notarai/README.md");
    Ok(())
}

fn setup_agents_md(project_dir: &Path) -> Result<(), String> {
    let path = project_dir.join("AGENTS.md");
    upsert_notarai_section(&path, AGENTS_MD_SECTION, "AGENTS.md")
}

fn setup_reconcile_prompt(notarai_dir: &Path) -> Result<(), String> {
    let dest = notarai_dir.join("reconcile-prompt.md");
    fs::write(&dest, RECONCILE_PROMPT_TEMPLATE)
        .map_err(|e| format!("could not write .notarai/reconcile-prompt.md: {e}"))?;
    println!("Wrote .notarai/reconcile-prompt.md");
    Ok(())
}

fn setup_bootstrap_prompt(notarai_dir: &Path) -> Result<(), String> {
    let dest = notarai_dir.join("bootstrap-prompt.md");
    fs::write(&dest, BOOTSTRAP_PROMPT_TEMPLATE)
        .map_err(|e| format!("could not write .notarai/bootstrap-prompt.md: {e}"))?;
    println!("Wrote .notarai/bootstrap-prompt.md");
    Ok(())
}

fn install_claude_hook(root: &Path) -> Result<(), String> {
    let claude_dir = root.join(".claude");
    if !claude_dir.exists() {
        fs::create_dir_all(&claude_dir)
            .map_err(|e| format!("could not create .claude/ directory: {e}"))?;
    }
    let settings_path = claude_dir.join("settings.json");

    let mut settings: serde_json::Value = if settings_path.exists() {
        let content = fs::read_to_string(&settings_path)
            .map_err(|e| format!("could not read .claude/settings.json: {e}"))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("could not parse existing .claude/settings.json: {e}"))?
    } else {
        serde_json::json!({})
    };

    if settings.get("hooks").is_none() {
        settings["hooks"] = serde_json::json!({});
    }
    if settings["hooks"].get("PostToolUse").is_none() {
        settings["hooks"]["PostToolUse"] = serde_json::json!([]);
    }

    let post_tool_use = settings["hooks"]["PostToolUse"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    if has_notarai_hook(&post_tool_use) {
        println!("NotarAI hook already configured in .claude/settings.json");
        return Ok(());
    }

    let hook_entry = serde_json::json!({
        "matcher": "Write|Edit",
        "hooks": [{
            "type": "command",
            "command": HOOK_COMMAND
        }]
    });

    settings["hooks"]["PostToolUse"]
        .as_array_mut()
        .expect("PostToolUse must be an array")
        .push(hook_entry);

    let content = serde_json::to_string_pretty(&settings).expect("JSON serialization") + "\n";
    fs::write(&settings_path, content)
        .map_err(|e| format!("could not write .claude/settings.json: {e}"))?;
    println!("Added NotarAI validation hook to .claude/settings.json");
    Ok(())
}

fn setup_gitignore(project_dir: &Path) -> Result<(), String> {
    let gitignore_path = project_dir.join(".gitignore");
    let cache_entry = ".notarai/.cache/";

    let existing = if gitignore_path.exists() {
        fs::read_to_string(&gitignore_path)
            .map_err(|e| format!("could not read .gitignore: {e}"))?
    } else {
        String::new()
    };

    if existing.lines().any(|line| line == cache_entry) {
        println!(".notarai/.cache/ already in .gitignore");
        return Ok(());
    }

    let mut content = existing;
    if !content.is_empty() && !content.ends_with('\n') {
        content.push('\n');
    }
    content.push_str(cache_entry);
    content.push('\n');

    fs::write(&gitignore_path, content).map_err(|e| format!("could not update .gitignore: {e}"))?;
    println!("Added .notarai/.cache/ to .gitignore");
    Ok(())
}

fn setup_mcp_json(project_root: &Path) -> Result<(), String> {
    let mcp_path = project_root.join(".mcp.json");
    let notarai_entry = serde_json::json!({
        "type": "stdio",
        "command": "notarai",
        "args": ["mcp"]
    });

    if mcp_path.exists() {
        let content =
            fs::read_to_string(&mcp_path).map_err(|e| format!("could not read .mcp.json: {e}"))?;
        let mut json: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| format!("could not parse .mcp.json: {e}"))?;

        if json
            .get("mcpServers")
            .and_then(|s| s.get("notarai"))
            .is_some()
        {
            println!("NotarAI MCP server already configured in .mcp.json");
            return Ok(());
        }

        if json.get("mcpServers").is_none() {
            json["mcpServers"] = serde_json::json!({});
        }
        json["mcpServers"]["notarai"] = notarai_entry;

        let out = serde_json::to_string_pretty(&json).expect("JSON serialization") + "\n";
        fs::write(&mcp_path, out).map_err(|e| format!("could not update .mcp.json: {e}"))?;
        println!("Added notarai MCP server to .mcp.json");
    } else {
        let content = serde_json::to_string_pretty(&serde_json::json!({
            "mcpServers": { "notarai": notarai_entry }
        }))
        .expect("JSON serialization")
            + "\n";
        fs::write(&mcp_path, content).map_err(|e| format!("could not write .mcp.json: {e}"))?;
        println!("Added NotarAI MCP server to .mcp.json");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Find `## NotarAI` at the start of a line and return the section content
    /// up to the next `## ` heading or EOF. Used only in tests.
    fn extract_notarai_section(content: &str) -> String {
        let start = content
            .lines()
            .enumerate()
            .find(|(_, line)| *line == "## NotarAI")
            .map(|(i, _)| i);

        let start = match start {
            Some(s) => s,
            None => return String::new(),
        };

        let lines: Vec<&str> = content.lines().collect();
        let end = lines[start + 1..]
            .iter()
            .enumerate()
            .find(|(_, line)| line.starts_with("## "))
            .map(|(i, _)| start + 1 + i)
            .unwrap_or(lines.len());

        let section: Vec<&str> = lines[start..end].to_vec();
        let result = section.join("\n");
        format!("{}\n", result.trim_end())
    }

    #[test]
    fn extracts_section_from_start_to_eof() {
        let content = "## NotarAI\n\nSome content here.\n";
        let result = extract_notarai_section(content);
        assert_eq!(result, "## NotarAI\n\nSome content here.\n");
    }

    #[test]
    fn extracts_section_between_headings() {
        let content = "## Intro\n\nIntro text.\n\n## NotarAI\n\nNotarAI content.\n\n## Other\n\nOther content.\n";
        let result = extract_notarai_section(content);
        assert_eq!(result, "## NotarAI\n\nNotarAI content.\n");
    }

    #[test]
    fn returns_empty_when_not_found() {
        let content = "## Intro\n\nNo notarai here.\n";
        let result = extract_notarai_section(content);
        assert_eq!(result, "");
    }

    #[test]
    fn replace_section_at_eof() {
        let content = "# Project\n\n## NotarAI\n\nOld content.\n";
        let result = replace_notarai_section(content, "## NotarAI\nnew\n");
        assert_eq!(result, "# Project\n\n## NotarAI\nnew\n");
    }

    #[test]
    fn replace_section_with_following_heading() {
        let content = "# Project\n\n## NotarAI\n\nOld.\n\n## Other\n\nAfter.\n";
        let result = replace_notarai_section(content, "## NotarAI\nnew\n");
        assert_eq!(result, "# Project\n\n## NotarAI\nnew\n## Other\n\nAfter.\n");
    }

    #[test]
    fn replace_returns_unchanged_when_no_section() {
        let content = "## Intro\n\nNo notarai.\n";
        let result = replace_notarai_section(content, "## NotarAI\nnew\n");
        assert_eq!(result, content);
    }

    #[test]
    fn resolve_agents_known_names() {
        let raw = vec!["claude".to_string(), "gemini".to_string()];
        let out = resolve_agents(&raw).unwrap();
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].name, "claude");
        assert_eq!(out[1].name, "gemini");
    }

    #[test]
    fn resolve_agents_all_expands() {
        let raw = vec!["all".to_string()];
        let out = resolve_agents(&raw).unwrap();
        assert_eq!(out.len(), ADAPTERS.len());
    }

    #[test]
    fn resolve_agents_generic_alias_maps_to_opencode() {
        let raw = vec!["generic".to_string()];
        let out = resolve_agents(&raw).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].name, "opencode");
    }

    #[test]
    fn resolve_agents_unknown_errors() {
        let raw = vec!["bogus".to_string()];
        assert!(resolve_agents(&raw).is_err());
    }

    #[test]
    fn resolve_agents_dedupes() {
        let raw = vec!["claude".to_string(), "claude".to_string()];
        let out = resolve_agents(&raw).unwrap();
        assert_eq!(out.len(), 1);
    }
}
