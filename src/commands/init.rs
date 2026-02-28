use std::fs;
use std::path::Path;

const HOOK_COMMAND: &str = "notarai hook validate";

const RECONCILE_MD: &str = include_str!("../../commands/notarai-reconcile.md");
const BOOTSTRAP_MD: &str = include_str!("../../commands/notarai-bootstrap.md");
const NOTARAI_README_TEMPLATE: &str = include_str!("../../templates/notarai-readme.md");
const SCHEMA_JSON: &str = include_str!("../../notarai.spec.json");

/// The section written to / replaced in CLAUDE.md. Exactly 3 lines plus trailing newline.
const NOTARAI_SECTION: &str = "## NotarAI\n@.notarai/README.md\n@.notarai/notarai.spec.json\n";

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

/// Find `## NotarAI` at the start of a line and return the section content
/// up to the next `## ` heading or EOF. Used only in tests.
#[cfg(test)]
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

/// Set up NotarAI in the target project directory.
pub fn run(project_root: Option<&Path>) -> i32 {
    let root = match project_root {
        Some(p) => p.to_path_buf(),
        None => std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf()),
    };

    let claude_dir = root.join(".claude");
    let notarai_dir = root.join(".notarai");

    if !claude_dir.exists()
        && let Err(e) = fs::create_dir_all(&claude_dir)
    {
        eprintln!("Error: could not create .claude/ directory: {e}");
        return 1;
    }

    let mut settings: serde_json::Value = {
        let settings_path = claude_dir.join("settings.json");
        if settings_path.exists() {
            match fs::read_to_string(&settings_path) {
                Ok(content) => match serde_json::from_str(&content) {
                    Ok(v) => v,
                    Err(_) => {
                        eprintln!("Error: could not parse existing .claude/settings.json");
                        return 1;
                    }
                },
                Err(_) => {
                    eprintln!("Error: could not read .claude/settings.json");
                    return 1;
                }
            }
        } else {
            serde_json::json!({})
        }
    };

    // Ensure hooks.PostToolUse exists
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
    } else {
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

        let settings_path = claude_dir.join("settings.json");
        let content = serde_json::to_string_pretty(&settings).expect("JSON serialization") + "\n";
        if let Err(e) = fs::write(&settings_path, content) {
            eprintln!("Error: could not write .claude/settings.json: {e}");
            return 1;
        }
        println!("Added NotarAI validation hook to .claude/settings.json");
    }

    // Create .notarai/ if it doesn't exist
    if !notarai_dir.exists()
        && let Err(e) = fs::create_dir_all(&notarai_dir)
    {
        eprintln!("Error: could not create .notarai/ directory: {e}");
        return 1;
    }

    setup_schema(&notarai_dir);
    setup_notarai_readme(&notarai_dir);
    setup_command("notarai-reconcile", RECONCILE_MD, &claude_dir);
    setup_command("notarai-bootstrap", BOOTSTRAP_MD, &claude_dir);
    setup_claude_context(&root);
    setup_gitignore(&root);
    setup_mcp_json(&root);

    0
}

fn setup_schema(notarai_dir: &Path) {
    let dest_path = notarai_dir.join("notarai.spec.json");

    if let Err(e) = fs::write(&dest_path, SCHEMA_JSON) {
        eprintln!("Warning: could not write notarai.spec.json: {e}");
        return;
    }

    println!("Copied schema to .notarai/notarai.spec.json");
}

fn setup_notarai_readme(notarai_dir: &Path) {
    let version = env!("CARGO_PKG_VERSION");
    let content = NOTARAI_README_TEMPLATE.replace("{{VERSION}}", version);
    let dest_path = notarai_dir.join("README.md");

    if let Err(e) = fs::write(&dest_path, content) {
        eprintln!("Warning: could not write .notarai/README.md: {e}");
        return;
    }

    println!("Wrote .notarai/README.md");
}

fn setup_claude_context(project_dir: &Path) {
    let claude_md_path = project_dir.join("CLAUDE.md");

    if claude_md_path.exists() {
        let existing = match fs::read_to_string(&claude_md_path) {
            Ok(c) => c,
            Err(_) => return,
        };

        let has_section = existing.lines().any(|line| line == "## NotarAI");

        let new_content = if has_section {
            replace_notarai_section(&existing, NOTARAI_SECTION)
        } else {
            let mut content = existing;
            if !content.is_empty() && !content.ends_with('\n') {
                content.push('\n');
            }
            content.push('\n');
            content.push_str(NOTARAI_SECTION);
            content
        };

        if let Err(e) = fs::write(&claude_md_path, new_content) {
            eprintln!("Warning: could not update CLAUDE.md: {e}");
            return;
        }
        if has_section {
            println!("Updated NotarAI section in CLAUDE.md");
        } else {
            println!("Added NotarAI context to CLAUDE.md");
        }
    } else {
        if let Err(e) = fs::write(&claude_md_path, NOTARAI_SECTION) {
            eprintln!("Warning: could not create CLAUDE.md: {e}");
            return;
        }
        println!("Added NotarAI context to CLAUDE.md");
    }
}

fn setup_command(name: &str, content: &str, claude_dir: &Path) {
    let commands_dir = claude_dir.join("commands");

    if let Err(e) = fs::create_dir_all(&commands_dir) {
        eprintln!("Warning: could not create .claude/commands/ directory: {e}");
        return;
    }

    let dest_path = commands_dir.join(format!("{name}.md"));

    if let Err(e) = fs::write(&dest_path, content) {
        eprintln!("Warning: could not write {name}.md: {e}");
        return;
    }

    println!("Updated .claude/commands/{name}.md");
}

fn setup_gitignore(project_dir: &Path) {
    let gitignore_path = project_dir.join(".gitignore");
    let cache_entry = ".notarai/.cache/";

    let existing = if gitignore_path.exists() {
        match fs::read_to_string(&gitignore_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Warning: could not read .gitignore: {e}");
                return;
            }
        }
    } else {
        String::new()
    };

    if existing.lines().any(|line| line == cache_entry) {
        println!(".notarai/.cache/ already in .gitignore");
        return;
    }

    let mut content = existing;
    if !content.is_empty() && !content.ends_with('\n') {
        content.push('\n');
    }
    content.push_str(cache_entry);
    content.push('\n');

    if let Err(e) = fs::write(&gitignore_path, content) {
        eprintln!("Warning: could not update .gitignore: {e}");
        return;
    }

    println!("Added .notarai/.cache/ to .gitignore");
}

fn setup_mcp_json(project_root: &Path) {
    let mcp_path = project_root.join(".mcp.json");

    let notarai_entry = serde_json::json!({
        "type": "stdio",
        "command": "notarai",
        "args": ["mcp"]
    });

    if mcp_path.exists() {
        let content = match fs::read_to_string(&mcp_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Warning: could not read .mcp.json: {e}");
                return;
            }
        };

        let mut json: serde_json::Value =
            serde_json::from_str(&content).unwrap_or(serde_json::json!({}));

        if json
            .get("mcpServers")
            .and_then(|s| s.get("notarai"))
            .is_some()
        {
            println!("NotarAI MCP server already configured in .mcp.json");
            return;
        }

        if json.get("mcpServers").is_none() {
            json["mcpServers"] = serde_json::json!({});
        }
        json["mcpServers"]["notarai"] = notarai_entry;

        let out = serde_json::to_string_pretty(&json).expect("JSON serialization") + "\n";
        if let Err(e) = fs::write(&mcp_path, out) {
            eprintln!("Warning: could not update .mcp.json: {e}");
            return;
        }
        println!("Added notarai MCP server to .mcp.json");
    } else {
        let content = serde_json::to_string_pretty(&serde_json::json!({
            "mcpServers": {
                "notarai": notarai_entry
            }
        }))
        .expect("JSON serialization")
            + "\n";

        if let Err(e) = fs::write(&mcp_path, content) {
            eprintln!("Warning: could not write .mcp.json: {e}");
            return;
        }
        println!("Added NotarAI MCP server to .mcp.json");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
