use std::fs;
use std::path::Path;

const HOOK_COMMAND: &str = "notarai hook validate";

const RECONCILE_MD: &str = include_str!("../../commands/notarai-reconcile.md");
const BOOTSTRAP_MD: &str = include_str!("../../commands/notarai-bootstrap.md");
const CLAUDE_CONTEXT_MD: &str = include_str!("../../templates/claude-context.md");
const SCHEMA_JSON: &str = include_str!("../../notarai.spec.json");

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

pub fn extract_notarai_section(content: &str) -> String {
    // Find "## NotarAI" at the start of a line
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

    // Find the next ## heading after the start
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

pub fn run(project_root: Option<&Path>) -> i32 {
    let root = match project_root {
        Some(p) => p.to_path_buf(),
        None => std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf()),
    };

    let claude_dir = root.join(".claude");
    let settings_path = claude_dir.join("settings.json");

    if !claude_dir.exists()
        && let Err(e) = fs::create_dir_all(&claude_dir)
    {
        eprintln!("Error: could not create .claude/ directory: {e}");
        return 1;
    }

    let mut settings: serde_json::Value = if settings_path.exists() {
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
            .unwrap()
            .push(hook_entry);

        let content = serde_json::to_string_pretty(&settings).unwrap() + "\n";
        if let Err(e) = fs::write(&settings_path, content) {
            eprintln!("Error: could not write .claude/settings.json: {e}");
            return 1;
        }
        println!("Added NotarAI validation hook to .claude/settings.json");
    }

    setup_command("notarai-reconcile", RECONCILE_MD, &claude_dir);
    setup_command("notarai-bootstrap", BOOTSTRAP_MD, &claude_dir);
    setup_schema(&claude_dir);
    setup_claude_context(&root);

    0
}

fn setup_claude_context(project_dir: &Path) {
    let claude_md_path = project_dir.join("CLAUDE.md");

    if claude_md_path.exists() {
        let existing = match fs::read_to_string(&claude_md_path) {
            Ok(c) => c,
            Err(_) => return,
        };

        let has_section = existing.lines().any(|line| line == "## NotarAI");
        if has_section {
            let existing_section = extract_notarai_section(&existing);
            let template_section = extract_notarai_section(CLAUDE_CONTEXT_MD);
            if existing_section != template_section {
                eprintln!(
                    "Warning: the ## NotarAI section in CLAUDE.md has drifted from the bundled template. \
                     Review manually, or delete the section and re-run `notarai init` to replace it."
                );
            } else {
                println!("NotarAI context already present in CLAUDE.md");
            }
            return;
        }

        // Append to existing CLAUDE.md
        let mut content = existing;
        content.push_str("\n\n");
        content.push_str(CLAUDE_CONTEXT_MD);
        if let Err(e) = fs::write(&claude_md_path, content) {
            eprintln!("Warning: could not update CLAUDE.md: {e}");
            return;
        }
    } else if let Err(e) = fs::write(&claude_md_path, CLAUDE_CONTEXT_MD) {
        eprintln!("Warning: could not create CLAUDE.md: {e}");
        return;
    }

    println!("Added NotarAI context to CLAUDE.md");
}

fn setup_command(name: &str, content: &str, claude_dir: &Path) {
    let commands_dir = claude_dir.join("commands");
    let dest_path = commands_dir.join(format!("{name}.md"));

    if dest_path.exists() {
        println!("{name} command already exists at .claude/commands/{name}.md");
        return;
    }

    if let Err(e) = fs::create_dir_all(&commands_dir) {
        eprintln!("Warning: could not create .claude/commands/ directory: {e}");
        return;
    }

    if let Err(e) = fs::write(&dest_path, content) {
        eprintln!("Warning: could not write {name}.md: {e}");
        return;
    }

    println!("Added /{name} command to .claude/commands/{name}.md");
}

fn setup_schema(claude_dir: &Path) {
    let dest_path = claude_dir.join("notarai.spec.json");

    if let Err(e) = fs::write(&dest_path, SCHEMA_JSON) {
        eprintln!("Warning: could not write notarai.spec.json: {e}");
        return;
    }

    println!("Copied schema to .claude/notarai.spec.json");
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
}
