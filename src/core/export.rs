use std::path::Path;

use crate::core::{git, spec_loader};

const RECONCILE_PROMPT_TEMPLATE: &str = include_str!("../../templates/reconcile-prompt.md");
const BOOTSTRAP_PROMPT_TEMPLATE: &str = include_str!("../../templates/bootstrap-prompt.md");

pub struct ExportContext {
    pub spec_path: String,
    pub spec_name: String,
    pub spec_content: String,
    pub base_branch: String,
    pub changed_files: Vec<String>,
    pub diff: String,
    pub binary_changes: Vec<String>,
    pub file_categories: serde_json::Map<String, serde_json::Value>,
}

/// Known binary file extensions (shared with mcp_tools).
const BINARY_EXTENSIONS: &[&str] = &[
    ".png", ".jpg", ".jpeg", ".gif", ".webp", ".ico", ".pptx", ".docx", ".xlsx", ".pdf", ".zip",
    ".tar", ".gz", ".wasm", ".exe", ".dll", ".so", ".dylib",
];

fn is_binary_by_extension(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    BINARY_EXTENSIONS.iter().any(|ext| lower.ends_with(ext))
}

/// Build export context for a single spec.
pub fn build_context(
    spec_path: &str,
    base_branch: &str,
    project_root: &Path,
) -> Result<ExportContext, String> {
    let abs_spec = project_root.join(spec_path);
    let spec_content = std::fs::read_to_string(&abs_spec)
        .map_err(|e| format!("could not read {spec_path}: {e}"))?;
    let spec_value = crate::core::yaml::parse_yaml(&spec_content)?;

    let spec_name = Path::new(spec_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(spec_path)
        .to_string();

    let governed_files = spec_loader::expand_artifact_globs(&spec_value, project_root);

    // Separate binary files.
    let (binary_files, non_binary): (Vec<String>, Vec<String>) = governed_files
        .into_iter()
        .partition(|f| is_binary_by_extension(f));

    // Get changed files from git.
    let all_changed = git::changed_files(base_branch, project_root)?;

    // Filter to only governed files that changed.
    let changed_files: Vec<String> = non_binary
        .iter()
        .filter(|f| all_changed.contains(f))
        .cloned()
        .collect();

    let binary_changes: Vec<String> = binary_files
        .iter()
        .filter(|f| all_changed.contains(f))
        .cloned()
        .collect();

    // Get diff for changed governed files.
    let diff = if changed_files.is_empty() {
        String::new()
    } else {
        git::diff_files(base_branch, &changed_files, &[], project_root)?
    };

    let file_categories =
        spec_loader::build_file_categories(&spec_value, &changed_files, project_root);

    Ok(ExportContext {
        spec_path: spec_path.to_string(),
        spec_name,
        spec_content,
        base_branch: base_branch.to_string(),
        changed_files,
        diff,
        binary_changes,
        file_categories,
    })
}

/// Build export contexts for all specs with affected artifacts.
pub fn build_all_contexts(
    base_branch: &str,
    project_root: &Path,
) -> Result<Vec<ExportContext>, String> {
    let all_changed = git::changed_files(base_branch, project_root)?;
    let specs = spec_loader::collect_specs(project_root)?;

    let mut contexts = Vec::new();
    for spec_path in &specs {
        let spec_rel = spec_path
            .strip_prefix(project_root)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| spec_path.to_string_lossy().to_string());

        let spec_value = spec_loader::load_spec(spec_path)?;

        if spec_loader::is_spec_affected(&spec_value, &all_changed) {
            let ctx = build_context(&spec_rel, base_branch, project_root)?;
            contexts.push(ctx);
        }
    }

    Ok(contexts)
}

/// Render an ExportContext to markdown using the lean reconcile prompt template.
/// The template instructs the agent to read files and run git diff itself.
pub fn render_markdown(ctx: &ExportContext) -> String {
    let changed_list = if ctx.changed_files.is_empty() {
        "No changed files.".to_string()
    } else {
        ctx.changed_files
            .iter()
            .map(|f| format!("- `{f}`"))
            .collect::<Vec<_>>()
            .join("\n")
    };

    RECONCILE_PROMPT_TEMPLATE
        .replace("{{spec_name}}", &ctx.spec_name)
        .replace("{{base_branch}}", &ctx.base_branch)
        .replace("{{changed_files}}", &changed_list)
        // Handle both formatted and unformatted placeholder variants (prettier expands
        // {{spec_content}} inside YAML code fences to { { spec_content } }).
        .replace("{{spec_content}}", &ctx.spec_content)
        .replace("{ { spec_content } }", &ctx.spec_content)
}

/// Output the bootstrap prompt template as-is (no placeholder substitution).
pub fn render_bootstrap() -> &'static str {
    BOOTSTRAP_PROMPT_TEMPLATE
}

/// Render an ExportContext to JSON.
pub fn render_json(ctx: &ExportContext) -> serde_json::Value {
    serde_json::json!({
        "spec_path": ctx.spec_path,
        "spec_name": ctx.spec_name,
        "spec_content": ctx.spec_content,
        "base_branch": ctx.base_branch,
        "changed_files": ctx.changed_files,
        "diff": ctx.diff,
        "binary_changes": ctx.binary_changes,
        "file_categories": ctx.file_categories,
    })
}
