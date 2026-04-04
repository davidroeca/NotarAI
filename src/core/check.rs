use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::core::spec_loader;

#[derive(Debug, Clone, PartialEq)]
pub enum CheckType {
    CoverageGap,
    OrphanedGlob,
    ChangedSinceReconciliation,
    OverlappingCoverage,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Severity {
    Warning,
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
                        severity: Severity::Warning,
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
