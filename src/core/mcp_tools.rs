use std::path::{Path, PathBuf};

/// A JSON-RPC error returned by an MCP tool.
pub struct McpError {
    pub code: i32,
    pub message: String,
}

/// Shorthand result type for MCP tool functions.
pub type McpResult = Result<serde_json::Value, McpError>;

/// List specs whose governed files overlap with files changed since `base_branch`.
///
/// Runs `git diff <base_branch> --name-only`, then cross-references each
/// `.notarai/*.spec.yaml` artifact glob against the changed file list. Returns
/// a JSON object with `changed_files` (all changed paths) and `affected_specs`
/// (specs with at least one matching artifact, including their `behaviors`,
/// `constraints`, and `invariants`).
pub fn list_affected_specs(base_branch: &str, project_root: &Path) -> McpResult {
    let output = std::process::Command::new("git")
        .args(["diff", base_branch, "--name-only"])
        .current_dir(project_root)
        .output()
        .map_err(|e| McpError {
            code: -32603,
            message: format!("git error: {e}"),
        })?;

    if !output.status.success() {
        return Err(McpError {
            code: -32603,
            message: String::from_utf8_lossy(&output.stderr).to_string(),
        });
    }

    let changed: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect();

    let specs = collect_specs(project_root)?;

    let mut affected = Vec::new();
    for spec_path in &specs {
        let spec_rel = spec_path
            .strip_prefix(project_root)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| spec_path.to_string_lossy().to_string());

        let content = std::fs::read_to_string(spec_path).map_err(|e| McpError {
            code: -32603,
            message: format!("read error for {spec_rel}: {e}"),
        })?;

        let spec_value = crate::core::yaml::parse_yaml(&content).map_err(|e| McpError {
            code: -32603,
            message: e,
        })?;

        if is_spec_affected(&spec_value, &changed) {
            let behaviors = spec_value
                .get("behaviors")
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            let constraints = spec_value
                .get("constraints")
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            let invariants = spec_value
                .get("invariants")
                .cloned()
                .unwrap_or(serde_json::Value::Null);

            affected.push(serde_json::json!({
                "spec_path": spec_rel,
                "behaviors": behaviors,
                "constraints": constraints,
                "invariants": invariants,
            }));
        }
    }

    Ok(serde_json::json!({
        "changed_files": changed,
        "affected_specs": affected,
    }))
}

/// Return a filtered `git diff` for the files governed by a spec.
///
/// Reads `spec_path`, expands its artifact globs, then:
/// - Splits governed files into spec files (`.notarai/**/*.spec.yaml`) and
///   non-spec artifacts.
/// - Unless `bypass_cache` is true, filters out files whose hash matches the
///   cache (these are listed in `"skipped"`). A cold or absent cache is treated
///   as "include everything".
/// - Returns full content (not a diff) for any changed spec files in
///   `"spec_changes"`. When `spec_changes` is non-empty, also includes
///   `"system_spec"` with the full content of the spec containing `subsystems`.
/// - Runs `git diff <base_branch>` on the remaining non-spec artifacts,
///   applying `exclude_patterns` as `:(exclude)` pathspecs.
///
/// The returned JSON has keys: `diff`, `files`, `skipped`, `excluded`,
/// `spec_changes`, `system_spec`.
pub fn get_spec_diff(
    spec_path: &str,
    base_branch: &str,
    exclude_patterns: &[String],
    bypass_cache: bool,
    project_root: &Path,
) -> McpResult {
    let abs_spec = project_root.join(spec_path);
    let content = std::fs::read_to_string(&abs_spec).map_err(|e| McpError {
        code: -32603,
        message: format!("read error: {e}"),
    })?;
    let spec_value = crate::core::yaml::parse_yaml(&content).map_err(|e| McpError {
        code: -32603,
        message: e,
    })?;

    let files = expand_artifact_globs(&spec_value, project_root);

    if files.is_empty() {
        return Ok(serde_json::json!({
            "diff": "",
            "files": [],
            "skipped": [],
            "excluded": exclude_patterns,
            "spec_changes": [],
            "system_spec": null,
        }));
    }

    // Partition governed files: .notarai/**/*.spec.yaml vs. everything else.
    let (spec_files, artifact_files): (Vec<String>, Vec<String>) =
        files.into_iter().partition(|f| is_spec_file(f));

    // Apply cache filtering to both groups independently.
    let (spec_to_show, artifact_to_diff, artifact_skipped) = if bypass_cache {
        (spec_files, artifact_files, vec![])
    } else {
        match crate::core::cache::open_cache_db(project_root) {
            Ok(conn) => {
                let (s_show, _s_skip): (Vec<String>, Vec<String>) =
                    spec_files.into_iter().partition(|rel| {
                        let abs = project_root.join(rel);
                        !matches!(
                            crate::core::cache::check_changed(&conn, rel, &abs),
                            Ok(None)
                        )
                    });
                let (a_diff, a_skip): (Vec<String>, Vec<String>) =
                    artifact_files.into_iter().partition(|rel| {
                        let abs = project_root.join(rel);
                        !matches!(
                            crate::core::cache::check_changed(&conn, rel, &abs),
                            Ok(None)
                        )
                    });
                (s_show, a_diff, a_skip)
            }
            Err(_) => (spec_files, artifact_files, vec![]), // cache unavailable: include everything
        }
    };

    // Read full content of each changed spec file.
    let mut spec_changes = Vec::new();
    for spec_rel in &spec_to_show {
        let abs = project_root.join(spec_rel);
        let spec_content = std::fs::read_to_string(&abs).map_err(|e| McpError {
            code: -32603,
            message: format!("read error for {spec_rel}: {e}"),
        })?;
        spec_changes.push(serde_json::json!({
            "path": spec_rel,
            "content": spec_content,
        }));
    }

    // Locate and include the system spec when any spec files changed.
    let system_spec = if spec_changes.is_empty() {
        serde_json::Value::Null
    } else {
        find_system_spec(project_root, &spec_to_show)?
    };

    // Build :(exclude) pathspecs from caller-supplied patterns.
    // Git resolves these as globs, so patterns like "Cargo.lock" or "*.lock"
    // work without pre-expansion.
    let diff = if artifact_to_diff.is_empty() {
        String::new()
    } else {
        let exclude_args: Vec<String> = exclude_patterns
            .iter()
            .map(|p| format!(":(exclude){p}"))
            .collect();

        let mut args: Vec<&str> = vec!["diff", base_branch, "--"];
        args.extend(artifact_to_diff.iter().map(String::as_str));
        args.extend(exclude_args.iter().map(String::as_str));

        let output = std::process::Command::new("git")
            .args(&args)
            .current_dir(project_root)
            .output()
            .map_err(|e| McpError {
                code: -32603,
                message: format!("git error: {e}"),
            })?;

        String::from_utf8_lossy(&output.stdout).to_string()
    };

    Ok(serde_json::json!({
        "diff": diff,
        "files": artifact_to_diff,
        "skipped": artifact_skipped,
        "excluded": exclude_patterns,
        "spec_changes": spec_changes,
        "system_spec": system_spec,
    }))
}

/// Delete the cache database file, if it exists.
///
/// Returns `{"cleared": true}` when the file was deleted, `{"cleared": false}`
/// when it did not exist.
pub fn clear_cache(project_root: &Path) -> McpResult {
    let path = crate::core::cache::db_path(project_root);
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| McpError {
            code: -32603,
            message: format!("could not delete cache: {e}"),
        })?;
        Ok(serde_json::json!({"cleared": true}))
    } else {
        Ok(serde_json::json!({"cleared": false}))
    }
}

/// Return artifact files governed by a spec that have changed since last cached.
///
/// Expands the spec's artifact globs (optionally filtered to `artifact_type`),
/// then checks each file against the hash cache. Files with a hash mismatch
/// (or absent from the cache) are returned in `{"changed_artifacts": [...]}`.
///
/// Unlike `get_spec_diff`, this does not run `git diff` -- it compares against
/// the local cache state, which is updated by `mark_reconciled`.
pub fn get_changed_artifacts(
    spec_path: &str,
    artifact_type: Option<&str>,
    project_root: &Path,
) -> McpResult {
    let abs_spec = project_root.join(spec_path);
    let content = std::fs::read_to_string(&abs_spec).map_err(|e| McpError {
        code: -32603,
        message: format!("read error: {e}"),
    })?;
    let spec_value = crate::core::yaml::parse_yaml(&content).map_err(|e| McpError {
        code: -32603,
        message: e,
    })?;

    let files = match artifact_type {
        Some(art_type) => expand_artifact_type_globs(&spec_value, project_root, art_type),
        None => expand_artifact_globs(&spec_value, project_root),
    };

    let conn = crate::core::cache::open_cache_db(project_root).map_err(|e| McpError {
        code: -32603,
        message: e,
    })?;

    let mut changed = Vec::new();
    for rel_file in &files {
        let abs_file = project_root.join(rel_file);
        match crate::core::cache::check_changed(&conn, rel_file, &abs_file) {
            Ok(Some(_)) => changed.push(rel_file.clone()),
            Ok(None) => {}
            Err(e) => eprintln!("Warning: {e}"),
        }
    }

    Ok(serde_json::json!({"changed_artifacts": changed}))
}

/// Record that the given files have been reconciled by hashing and caching them.
///
/// For each path in `files` that exists on disk, computes its BLAKE3 hash and
/// upserts it into the cache. Files that do not exist are silently skipped.
/// Returns `{"updated": N}` with the count of successfully cached files.
///
/// This is the correct way to seed or update the MCP cache -- not the CLI
/// `cache update` subcommand, which uses absolute paths as keys instead of
/// relative paths.
pub fn mark_reconciled(files: &[String], project_root: &Path) -> McpResult {
    let conn = crate::core::cache::open_cache_db(project_root).map_err(|e| McpError {
        code: -32603,
        message: e,
    })?;

    let mut count = 0;
    for f in files {
        let abs = project_root.join(f);
        if abs.exists() {
            match crate::core::cache::hash_file(&abs) {
                Ok(hash) => {
                    crate::core::cache::upsert(&conn, f, &hash).map_err(|e| McpError {
                        code: -32603,
                        message: e,
                    })?;
                    count += 1;
                }
                Err(e) => eprintln!("Warning: {e}"),
            }
        }
    }

    Ok(serde_json::json!({"updated": count}))
}

fn is_spec_file(path: &str) -> bool {
    path.starts_with(".notarai/") && path.ends_with(".spec.yaml")
}

/// Locate the system spec (the one with a `subsystems` key) in `.notarai/`.
///
/// If the system spec is already in `spec_changes_paths` (i.e., it changed),
/// returns `{path}` only to avoid duplicating its content. Otherwise returns
/// `{path, content}` with the full file. Returns `null` if no system spec is found.
fn find_system_spec(
    project_root: &Path,
    spec_changes_paths: &[String],
) -> Result<serde_json::Value, McpError> {
    let notarai_dir = project_root.join(".notarai");
    if !notarai_dir.exists() {
        return Ok(serde_json::Value::Null);
    }

    let mut system_spec_rel: Option<String> = None;

    // Scan .notarai/ (non-recursive) for a spec with a `subsystems` key.
    if let Ok(entries) = std::fs::read_dir(&notarai_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if !name.ends_with(".spec.yaml") {
                continue;
            }
            if let Ok(content) = std::fs::read_to_string(&path)
                && let Ok(value) = crate::core::yaml::parse_yaml(&content)
                && value.get("subsystems").is_some()
                && let Ok(rel) = path.strip_prefix(project_root)
            {
                system_spec_rel = Some(rel.to_string_lossy().to_string());
                break;
            }
        }
    }

    // Fallback: check for .notarai/system.spec.yaml by name.
    if system_spec_rel.is_none() {
        let candidate = notarai_dir.join("system.spec.yaml");
        if candidate.exists()
            && let Ok(rel) = candidate.strip_prefix(project_root)
        {
            system_spec_rel = Some(rel.to_string_lossy().to_string());
        }
    }

    let Some(sys_path) = system_spec_rel else {
        return Ok(serde_json::Value::Null);
    };

    // If the system spec itself changed, it's already in spec_changes -- return
    // just the path reference to avoid duplicating the content.
    if spec_changes_paths.contains(&sys_path) {
        return Ok(serde_json::json!({"path": sys_path}));
    }

    // Otherwise read its full content.
    let abs_sys = project_root.join(&sys_path);
    let content = std::fs::read_to_string(&abs_sys).map_err(|e| McpError {
        code: -32603,
        message: format!("read error for system spec {sys_path}: {e}"),
    })?;

    Ok(serde_json::json!({
        "path": sys_path,
        "content": content,
    }))
}

fn collect_specs(project_root: &Path) -> Result<Vec<PathBuf>, McpError> {
    use walkdir::WalkDir;
    let mut specs = Vec::new();
    let notarai_dir = project_root.join(".notarai");
    if !notarai_dir.exists() {
        return Ok(specs);
    }
    for entry in WalkDir::new(&notarai_dir) {
        let entry = entry.map_err(|e| McpError {
            code: -32603,
            message: format!("{e}"),
        })?;
        if entry.file_type().is_file() {
            let name = entry.file_name().to_string_lossy();
            if name.ends_with(".spec.yaml") {
                specs.push(entry.into_path());
            }
        }
    }
    Ok(specs)
}

fn is_spec_affected(spec: &serde_json::Value, changed: &[String]) -> bool {
    let Some(artifacts) = spec.get("artifacts") else {
        return false;
    };
    let Some(obj) = artifacts.as_object() else {
        return false;
    };
    for (_key, refs) in obj {
        let Some(arr) = refs.as_array() else {
            continue;
        };
        for item in arr {
            let Some(pattern_str) = item.get("path").and_then(|p| p.as_str()) else {
                continue;
            };
            if let Ok(pattern) = glob::Pattern::new(pattern_str) {
                for changed_file in changed {
                    if pattern.matches(changed_file) {
                        return true;
                    }
                }
            }
        }
    }
    false
}

fn expand_artifact_globs(spec: &serde_json::Value, project_root: &Path) -> Vec<String> {
    let mut files = Vec::new();
    let Some(artifacts) = spec.get("artifacts").and_then(|a| a.as_object()) else {
        return files;
    };
    for (_key, refs) in artifacts {
        let Some(arr) = refs.as_array() else {
            continue;
        };
        for item in arr {
            if let Some(pattern_str) = item.get("path").and_then(|p| p.as_str()) {
                files.extend(expand_glob(pattern_str, project_root));
            }
        }
    }
    files
}

fn expand_artifact_type_globs(
    spec: &serde_json::Value,
    project_root: &Path,
    art_type: &str,
) -> Vec<String> {
    let mut files = Vec::new();
    let Some(refs) = spec
        .get("artifacts")
        .and_then(|a| a.get(art_type))
        .and_then(|r| r.as_array())
    else {
        return files;
    };
    for item in refs {
        if let Some(pattern_str) = item.get("path").and_then(|p| p.as_str()) {
            files.extend(expand_glob(pattern_str, project_root));
        }
    }
    files
}

fn expand_glob(pattern_str: &str, project_root: &Path) -> Vec<String> {
    let abs_pattern = project_root.join(pattern_str);
    let abs_pattern_str = abs_pattern.to_string_lossy();
    let mut result = Vec::new();
    if let Ok(paths) = glob::glob(&abs_pattern_str) {
        for path in paths.filter_map(|p| p.ok()) {
            if let Ok(rel) = path.strip_prefix(project_root) {
                result.push(rel.to_string_lossy().to_string());
            }
        }
    }
    result
}
