use std::path::{Path, PathBuf};

pub struct McpError {
    pub code: i32,
    pub message: String,
}

pub type McpResult = Result<serde_json::Value, McpError>;

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
        return Ok(
            serde_json::json!({"diff": "", "files": [], "skipped": [], "excluded": exclude_patterns}),
        );
    }

    // Split files into to_diff (cache says changed) and skipped (cache says unchanged).
    let (files_to_diff, skipped): (Vec<String>, Vec<String>) = if bypass_cache {
        (files, vec![])
    } else {
        match crate::core::cache::open_cache_db(project_root) {
            Ok(conn) => files.into_iter().partition(|rel| {
                let abs = project_root.join(rel);
                !matches!(
                    crate::core::cache::check_changed(&conn, rel, &abs),
                    Ok(None)
                )
            }),
            Err(_) => (files, vec![]), // cache unavailable: diff everything
        }
    };

    if files_to_diff.is_empty() {
        return Ok(serde_json::json!({
            "diff": "",
            "files": [],
            "skipped": skipped,
            "excluded": exclude_patterns,
        }));
    }

    // Build :(exclude) pathspecs from caller-supplied patterns.
    // Git resolves these as globs, so patterns like "Cargo.lock" or "*.lock"
    // work without pre-expansion.
    let exclude_args: Vec<String> = exclude_patterns
        .iter()
        .map(|p| format!(":(exclude){p}"))
        .collect();

    let mut args: Vec<&str> = vec!["diff", base_branch, "--"];
    args.extend(files_to_diff.iter().map(String::as_str));
    args.extend(exclude_args.iter().map(String::as_str));

    let output = std::process::Command::new("git")
        .args(&args)
        .current_dir(project_root)
        .output()
        .map_err(|e| McpError {
            code: -32603,
            message: format!("git error: {e}"),
        })?;

    let diff = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(serde_json::json!({
        "diff": diff,
        "files": files_to_diff,
        "skipped": skipped,
        "excluded": exclude_patterns,
    }))
}

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
