use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::core::spec_loader;

#[derive(Debug, Clone, PartialEq)]
pub enum CheckType {
    CoverageGap,
    OrphanedGlob,
    ChangedSinceReconciliation,
    OverlappingCoverage,
    CircularRef,
    BehaviorIncomplete,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Severity {
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub struct CheckFinding {
    pub check_type: CheckType,
    pub severity: Severity,
    pub spec_path: Option<String>,
    pub file_path: Option<String>,
    pub glob_pattern: Option<String>,
    pub message: String,
}

pub struct CheckResult {
    pub findings: Vec<CheckFinding>,
}

/// Run all deterministic drift checks. Never modifies files or the cache.
pub fn run_all_checks(project_root: &Path) -> Result<CheckResult, String> {
    let specs = spec_loader::collect_specs(project_root)?;
    let exclude_patterns = spec_loader::get_exclude_patterns(project_root)?;

    // Load all specs: (relative_path, parsed_value)
    let mut loaded_specs = Vec::new();
    for spec_path in &specs {
        let spec_rel = spec_path
            .strip_prefix(project_root)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| spec_path.to_string_lossy().to_string());
        let spec_value = spec_loader::load_spec(spec_path)?;
        loaded_specs.push((spec_rel, spec_value));
    }

    // Expand all artifact globs once, per spec.
    let spec_files: Vec<(String, Vec<String>)> = loaded_specs
        .iter()
        .map(|(rel, val)| {
            let files = spec_loader::expand_artifact_globs(val, project_root);
            (rel.clone(), files)
        })
        .collect();

    // Collect all governed files across all specs.
    let all_governed: HashSet<String> = spec_files
        .iter()
        .flat_map(|(_, files)| files.iter().cloned())
        .collect();

    let mut findings = Vec::new();

    findings.extend(check_coverage_gaps(
        project_root,
        &exclude_patterns,
        &all_governed,
    )?);
    findings.extend(check_orphaned_globs(&loaded_specs, project_root));
    findings.extend(check_changed_since(project_root, &all_governed)?);
    findings.extend(check_overlapping_coverage(&spec_files));
    findings.extend(check_circular_refs(&loaded_specs));
    findings.extend(check_behavior_completeness(&loaded_specs));

    Ok(CheckResult { findings })
}

/// Find tracked files not governed by any spec and not excluded.
fn check_coverage_gaps(
    project_root: &Path,
    exclude_patterns: &[String],
    all_governed: &HashSet<String>,
) -> Result<Vec<CheckFinding>, String> {
    let tracked = crate::core::git::tracked_files(project_root)?;

    // Compile exclude patterns once.
    let exclude_matchers: Vec<glob::Pattern> = exclude_patterns
        .iter()
        .filter_map(|p| glob::Pattern::new(p).ok())
        .collect();

    let mut findings = Vec::new();
    for file in &tracked {
        // Skip spec files, schema, cache, and other .notarai/ internal files.
        if file.starts_with(".notarai/") {
            continue;
        }
        // Skip .git/ (should not appear from ls-files, but just in case).
        if file.starts_with(".git/") {
            continue;
        }
        if all_governed.contains(file) {
            continue;
        }
        if exclude_matchers.iter().any(|p| p.matches(file)) {
            continue;
        }
        findings.push(CheckFinding {
            check_type: CheckType::CoverageGap,
            severity: Severity::Warning,
            spec_path: None,
            file_path: Some(file.clone()),
            glob_pattern: None,
            message: format!("File not governed by any spec: {file}"),
        });
    }

    Ok(findings)
}

/// Find artifact globs that match zero files.
fn check_orphaned_globs(
    loaded_specs: &[(String, serde_json::Value)],
    project_root: &Path,
) -> Vec<CheckFinding> {
    let mut findings = Vec::new();

    for (spec_rel, spec_value) in loaded_specs {
        let Some(artifacts) = spec_value.get("artifacts").and_then(|a| a.as_object()) else {
            continue;
        };
        for (_category, refs) in artifacts {
            let Some(arr) = refs.as_array() else {
                continue;
            };
            for item in arr {
                let Some(pattern_str) = item.get("path").and_then(|p| p.as_str()) else {
                    continue;
                };
                let expanded = spec_loader::expand_glob(pattern_str, project_root);
                if expanded.is_empty() {
                    findings.push(CheckFinding {
                        check_type: CheckType::OrphanedGlob,
                        severity: Severity::Error,
                        spec_path: Some(spec_rel.clone()),
                        file_path: None,
                        glob_pattern: Some(pattern_str.to_string()),
                        message: format!(
                            "Artifact glob matches no files: {pattern_str} (in {spec_rel})"
                        ),
                    });
                }
            }
        }
    }

    findings
}

/// Find governed files that have changed since last reconciliation.
fn check_changed_since(
    project_root: &Path,
    all_governed: &HashSet<String>,
) -> Result<Vec<CheckFinding>, String> {
    // Only check if the cache DB actually exists on disk. open_cache_db
    // creates the DB if absent, which would make all files look "changed."
    let db_path = crate::core::cache::db_path(project_root);
    if !db_path.exists() {
        return Ok(vec![]);
    }
    let conn = match crate::core::cache::open_cache_db(project_root) {
        Ok(c) => c,
        Err(_) => return Ok(vec![]),
    };

    let pairs: Vec<(String, std::path::PathBuf)> = all_governed
        .iter()
        .map(|rel| {
            let abs = project_root.join(rel);
            (rel.clone(), abs)
        })
        .collect();

    let (changed, _) =
        crate::core::cache::check_changed_batch(&conn, &pairs).unwrap_or((vec![], vec![]));

    Ok(changed
        .into_iter()
        .map(|file| CheckFinding {
            check_type: CheckType::ChangedSinceReconciliation,
            severity: Severity::Warning,
            spec_path: None,
            file_path: Some(file.clone()),
            glob_pattern: None,
            message: format!("File changed since last reconciliation: {file}"),
        })
        .collect())
}

/// Find files governed by multiple specs.
fn check_overlapping_coverage(spec_files: &[(String, Vec<String>)]) -> Vec<CheckFinding> {
    let mut file_to_specs: HashMap<&str, Vec<&str>> = HashMap::new();

    for (spec_rel, files) in spec_files {
        for file in files {
            file_to_specs
                .entry(file.as_str())
                .or_default()
                .push(spec_rel.as_str());
        }
    }

    let mut findings: Vec<CheckFinding> = file_to_specs
        .into_iter()
        .filter(|(_, specs)| specs.len() > 1)
        .map(|(file, specs)| {
            let spec_list = specs.join(", ");
            CheckFinding {
                check_type: CheckType::OverlappingCoverage,
                severity: Severity::Warning,
                spec_path: None,
                file_path: Some(file.to_string()),
                glob_pattern: None,
                message: format!("File governed by multiple specs: {file} ({spec_list})"),
            }
        })
        .collect();

    // Sort for deterministic output.
    findings.sort_by(|a, b| a.file_path.cmp(&b.file_path));
    findings
}

/// Detect cycles in `$ref` chains across `subsystems`, `applies`, and `dependencies`.
///
/// Refs are resolved relative to the containing spec file's directory when they start
/// with `./` or `../`, and relative to the project root otherwise. Only refs that match
/// another loaded spec participate in the graph; unresolved refs are silently ignored
/// (they would be caught by a separate check if added later).
fn check_circular_refs(loaded_specs: &[(String, serde_json::Value)]) -> Vec<CheckFinding> {
    // Build a path -> adjacency list map keyed by the normalized relative spec path.
    let known: HashSet<&str> = loaded_specs.iter().map(|(p, _)| p.as_str()).collect();
    let mut graph: HashMap<String, Vec<String>> = HashMap::new();

    for (spec_rel, spec_value) in loaded_specs {
        let parent_dir = Path::new(spec_rel)
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_default();

        let mut edges = Vec::new();
        for field in ["subsystems", "applies", "dependencies"] {
            let Some(arr) = spec_value.get(field).and_then(|v| v.as_array()) else {
                continue;
            };
            for item in arr {
                let Some(ref_str) = item.get("$ref").and_then(|r| r.as_str()) else {
                    continue;
                };
                let resolved = resolve_ref_path(&parent_dir, ref_str);
                if known.contains(resolved.as_str()) {
                    edges.push(resolved);
                }
            }
        }
        graph.insert(spec_rel.clone(), edges);
    }

    // DFS with a colored visit set: 0 = unvisited, 1 = on stack, 2 = done.
    let mut color: HashMap<&str, u8> = loaded_specs.iter().map(|(p, _)| (p.as_str(), 0)).collect();
    let mut reported: HashSet<Vec<String>> = HashSet::new();
    let mut findings = Vec::new();

    // Visit in deterministic order.
    let mut roots: Vec<&str> = loaded_specs.iter().map(|(p, _)| p.as_str()).collect();
    roots.sort();

    for root in roots {
        if color.get(root).copied().unwrap_or(2) != 0 {
            continue;
        }
        let mut stack: Vec<String> = vec![root.to_string()];
        dfs_find_cycles(&graph, &mut color, &mut stack, &mut reported, &mut findings);
    }

    findings
}

/// Resolve a `$ref` string relative to the containing spec's parent directory.
///
/// If `ref_str` starts with `./` or `../`, it is joined with `parent_dir` and normalized.
/// Otherwise it is returned as-is (treated as project-root-relative).
fn resolve_ref_path(parent_dir: &Path, ref_str: &str) -> String {
    if ref_str.starts_with("./") || ref_str.starts_with("../") {
        let joined = parent_dir.join(ref_str);
        normalize_path(&joined)
    } else {
        ref_str.to_string()
    }
}

/// Collapse `.` and `..` components without touching the filesystem.
fn normalize_path(path: &Path) -> String {
    let mut out: Vec<std::path::Component<'_>> = Vec::new();
    for comp in path.components() {
        match comp {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                if !matches!(
                    out.last(),
                    Some(std::path::Component::RootDir) | Some(std::path::Component::Prefix(_))
                ) {
                    out.pop();
                }
            }
            c => out.push(c),
        }
    }
    out.iter()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn dfs_find_cycles<'a>(
    graph: &'a HashMap<String, Vec<String>>,
    color: &mut HashMap<&'a str, u8>,
    stack: &mut Vec<String>,
    reported: &mut HashSet<Vec<String>>,
    findings: &mut Vec<CheckFinding>,
) {
    let node = match stack.last() {
        Some(n) => n.clone(),
        None => return,
    };

    // Mark current as on-stack via a lookup into the keys owned by the graph.
    if let Some((k, _)) = graph.get_key_value(&node) {
        color.insert(k.as_str(), 1);
    }

    if let Some(neighbors) = graph.get(&node) {
        let mut sorted = neighbors.clone();
        sorted.sort();
        for next in sorted {
            let c = color.get(next.as_str()).copied().unwrap_or(0);
            match c {
                0 => {
                    stack.push(next);
                    dfs_find_cycles(graph, color, stack, reported, findings);
                    stack.pop();
                }
                1 => {
                    // Found a cycle: slice the stack from the first occurrence of `next`.
                    if let Some(start) = stack.iter().position(|s| s == &next) {
                        let mut cycle: Vec<String> = stack[start..].to_vec();
                        cycle.push(next.clone());
                        // Canonicalize for dedup: rotate so the lexicographically smallest
                        // element comes first, and ignore the trailing duplicate.
                        let canonical = canonicalize_cycle(&cycle);
                        if reported.insert(canonical.clone()) {
                            let display = cycle.join(" -> ");
                            findings.push(CheckFinding {
                                check_type: CheckType::CircularRef,
                                severity: Severity::Error,
                                spec_path: Some(cycle[0].clone()),
                                file_path: None,
                                glob_pattern: None,
                                message: format!("Circular $ref chain: {display}"),
                            });
                        }
                    }
                }
                _ => {}
            }
        }
    }

    if let Some((k, _)) = graph.get_key_value(&node) {
        color.insert(k.as_str(), 2);
    }
}

/// Canonicalize a cycle for deduplication: drop the trailing duplicate node and rotate
/// the remaining nodes so the lexicographically smallest comes first.
fn canonicalize_cycle(cycle: &[String]) -> Vec<String> {
    if cycle.len() < 2 {
        return cycle.to_vec();
    }
    let nodes = &cycle[..cycle.len() - 1];
    let min_idx = nodes
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| a.cmp(b))
        .map(|(i, _)| i)
        .unwrap_or(0);
    let mut rotated: Vec<String> = nodes[min_idx..].to_vec();
    rotated.extend_from_slice(&nodes[..min_idx]);
    rotated
}

/// Flag behaviors missing a `given` or `then` field (or where either is blank).
fn check_behavior_completeness(loaded_specs: &[(String, serde_json::Value)]) -> Vec<CheckFinding> {
    let mut findings = Vec::new();

    for (spec_rel, spec_value) in loaded_specs {
        let Some(behaviors) = spec_value.get("behaviors").and_then(|b| b.as_array()) else {
            continue;
        };
        for (idx, behavior) in behaviors.iter().enumerate() {
            let name = behavior
                .get("name")
                .and_then(|n| n.as_str())
                .map(str::to_string)
                .unwrap_or_else(|| format!("behavior[{idx}]"));

            for field in ["given", "then"] {
                let missing = !matches!(
                    behavior.get(field).and_then(|v| v.as_str()),
                    Some(s) if !s.trim().is_empty()
                );
                if missing {
                    findings.push(CheckFinding {
                        check_type: CheckType::BehaviorIncomplete,
                        severity: Severity::Warning,
                        spec_path: Some(spec_rel.clone()),
                        file_path: None,
                        glob_pattern: None,
                        message: format!(
                            "Behavior '{name}' missing '{field}' field (in {spec_rel})"
                        ),
                    });
                }
            }
        }
    }

    findings
}
