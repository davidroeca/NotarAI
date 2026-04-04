use std::path::{Path, PathBuf};

/// Recursively find all `.spec.yaml` files under `.notarai/`.
pub fn collect_specs(project_root: &Path) -> Result<Vec<PathBuf>, String> {
    use walkdir::WalkDir;
    let mut specs = Vec::new();
    let notarai_dir = project_root.join(".notarai");
    if !notarai_dir.exists() {
        return Ok(specs);
    }
    for entry in WalkDir::new(&notarai_dir) {
        let entry = entry.map_err(|e| format!("{e}"))?;
        if entry.file_type().is_file() {
            let name = entry.file_name().to_string_lossy();
            if name.ends_with(".spec.yaml") {
                specs.push(entry.into_path());
            }
        }
    }
    Ok(specs)
}

/// Read and parse a spec YAML file into a JSON Value.
pub fn load_spec(spec_path: &Path) -> Result<serde_json::Value, String> {
    let content = std::fs::read_to_string(spec_path)
        .map_err(|e| format!("read error for {}: {e}", spec_path.display()))?;
    crate::core::yaml::parse_yaml(&content)
}

/// Check whether a relative path looks like a spec file.
pub fn is_spec_file(path: &str) -> bool {
    path.starts_with(".notarai/") && path.ends_with(".spec.yaml")
}

/// Check whether any of a spec's artifact globs match any of the changed files.
pub fn is_spec_affected(spec: &serde_json::Value, changed: &[String]) -> bool {
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

/// Expand all artifact glob patterns in a spec to concrete file paths.
pub fn expand_artifact_globs(spec: &serde_json::Value, project_root: &Path) -> Vec<String> {
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

/// Expand artifact globs for a specific artifact type only.
pub fn expand_artifact_type_globs(
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

/// Expand a single glob pattern relative to the project root.
///
/// Returns relative paths from the project root.
pub fn expand_glob(pattern_str: &str, project_root: &Path) -> Vec<String> {
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

/// Build a map from file path to artifact category name based on a spec's artifact globs.
///
/// Each file in `files` is matched against every category's glob patterns. The first
/// matching category wins. Files that match no category are omitted.
pub fn build_file_categories(
    spec: &serde_json::Value,
    files: &[String],
    project_root: &Path,
) -> serde_json::Map<String, serde_json::Value> {
    use std::collections::HashSet;

    let mut map = serde_json::Map::new();
    let Some(artifacts) = spec.get("artifacts").and_then(|a| a.as_object()) else {
        return map;
    };

    // Pre-expand all category globs once into HashSets for O(1) lookup.
    let category_files: Vec<(String, HashSet<String>)> = artifacts
        .iter()
        .map(|(cat, refs)| {
            let expanded: HashSet<String> = refs
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|item| item.get("path").and_then(|p| p.as_str()))
                        .flat_map(|pattern| expand_glob(pattern, project_root))
                        .collect()
                })
                .unwrap_or_default();
            (cat.clone(), expanded)
        })
        .collect();

    for file in files {
        for (cat, cat_files) in &category_files {
            if cat_files.contains(file) {
                map.insert(file.clone(), serde_json::Value::String(cat.clone()));
                break;
            }
        }
    }

    map
}

/// Locate the system spec (the one with a `subsystems` key) in `.notarai/`.
///
/// If the system spec is already in `spec_changes_paths` (i.e., it changed),
/// returns `{path}` only to avoid duplicating its content. Otherwise returns
/// `{path, content}` with the full file. Returns `null` if no system spec is found.
pub fn find_system_spec(
    project_root: &Path,
    spec_changes_paths: &[String],
) -> Result<serde_json::Value, String> {
    let notarai_dir = project_root.join(".notarai");
    if !notarai_dir.exists() {
        return Ok(serde_json::Value::Null);
    }

    let mut system_spec_rel: Option<String> = None;

    // Fast path: check for .notarai/system.spec.yaml by convention name first.
    let candidate = notarai_dir.join("system.spec.yaml");
    if candidate.exists()
        && let Ok(content) = std::fs::read_to_string(&candidate)
        && let Ok(value) = crate::core::yaml::parse_yaml(&content)
        && value.get("subsystems").is_some()
        && let Ok(rel) = candidate.strip_prefix(project_root)
    {
        system_spec_rel = Some(rel.to_string_lossy().to_string());
    }

    // Fallback: scan .notarai/ (non-recursive) for a spec with a `subsystems` key.
    if system_spec_rel.is_none()
        && let Ok(entries) = std::fs::read_dir(&notarai_dir)
    {
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
    let content = std::fs::read_to_string(&abs_sys)
        .map_err(|e| format!("read error for system spec {sys_path}: {e}"))?;

    Ok(serde_json::json!({
        "path": sys_path,
        "content": content,
    }))
}

/// Extract the `exclude` glob patterns from the system spec.
///
/// Returns an empty vec if no system spec exists or it has no `exclude` field.
pub fn get_exclude_patterns(project_root: &Path) -> Result<Vec<String>, String> {
    let notarai_dir = project_root.join(".notarai");
    if !notarai_dir.exists() {
        return Ok(vec![]);
    }

    // Fast path: check system.spec.yaml by convention.
    let candidate = notarai_dir.join("system.spec.yaml");
    let spec_value = if candidate.exists() {
        let content =
            std::fs::read_to_string(&candidate).map_err(|e| format!("read error: {e}"))?;
        let value = crate::core::yaml::parse_yaml(&content)?;
        if value.get("subsystems").is_some() || value.get("exclude").is_some() {
            Some(value)
        } else {
            None
        }
    } else {
        None
    };

    // Fallback: scan for system spec.
    let spec_value = match spec_value {
        Some(v) => v,
        None => {
            let entries =
                std::fs::read_dir(&notarai_dir).map_err(|e| format!("read dir error: {e}"))?;
            let mut found = None;
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
                {
                    found = Some(value);
                    break;
                }
            }
            match found {
                Some(v) => v,
                None => return Ok(vec![]),
            }
        }
    };

    let patterns = spec_value
        .get("exclude")
        .and_then(|e| e.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    Ok(patterns)
}
