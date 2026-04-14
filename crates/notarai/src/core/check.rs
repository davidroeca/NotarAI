use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::core::lint::LintRuleId;
use crate::core::spec_loader;

/// Configuration loaded from `.notarai/check.yaml`.
pub struct CheckConfig {
    /// Minimum tier that causes a non-zero exit code. Default: error-severity only.
    pub fail_on: Option<SeverityTier>,
    /// Minimum tier to show in output. Tiers below this are suppressed.
    pub warn_on: SeverityTier,
}

impl Default for CheckConfig {
    fn default() -> Self {
        CheckConfig {
            fail_on: None, // Use existing error-severity behavior by default.
            warn_on: SeverityTier::Housekeeping,
        }
    }
}

impl CheckConfig {
    /// Load from `.notarai/check.yaml` if it exists, otherwise return defaults.
    pub fn load(project_root: &Path) -> Self {
        let config_path = project_root.join(".notarai/check.yaml");
        let content = match std::fs::read_to_string(&config_path) {
            Ok(c) => c,
            Err(_) => return CheckConfig::default(),
        };
        let value: serde_json::Value = match serde_yaml_ng::from_str(&content) {
            Ok(v) => v,
            Err(_) => return CheckConfig::default(),
        };

        let fail_on = value
            .get("fail_on")
            .and_then(|v| v.as_str())
            .and_then(SeverityTier::from_str);
        let warn_on = value
            .get("warn_on")
            .and_then(|v| v.as_str())
            .and_then(SeverityTier::from_str)
            .unwrap_or(SeverityTier::Housekeeping);

        CheckConfig { fail_on, warn_on }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CheckType {
    CoverageGap,
    OrphanedGlob,
    ChangedSinceReconciliation,
    OverlappingCoverage,
    CircularRef,
    BehaviorIncomplete,
    LintViolation(LintRuleId),
    TestCoverageMissing,
    TestPathMissing,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Severity {
    Warning,
    Error,
}

/// Reconciliation severity tier. Classifies findings by impact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SeverityTier {
    /// Behavioral/invariant violation or broken references.
    Critical = 0,
    /// Code changed in ways that may not align with spec.
    Drift = 1,
    /// Documentation, style, or organizational misalignment.
    Housekeeping = 2,
}

impl SeverityTier {
    pub fn as_str(&self) -> &'static str {
        match self {
            SeverityTier::Critical => "critical",
            SeverityTier::Drift => "drift",
            SeverityTier::Housekeeping => "housekeeping",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            SeverityTier::Critical => "Critical",
            SeverityTier::Drift => "Drift",
            SeverityTier::Housekeeping => "Housekeeping",
        }
    }

    pub fn from_str(s: &str) -> Option<SeverityTier> {
        match s {
            "critical" => Some(SeverityTier::Critical),
            "drift" => Some(SeverityTier::Drift),
            "housekeeping" => Some(SeverityTier::Housekeeping),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CheckFinding {
    pub check_type: CheckType,
    pub severity: Severity,
    pub tier: SeverityTier,
    pub spec_path: Option<String>,
    pub file_path: Option<String>,
    pub glob_pattern: Option<String>,
    pub message: String,
}

pub struct CheckResult {
    pub findings: Vec<CheckFinding>,
}

/// Derive the severity tier for a given check type.
pub fn tier_for_check_type(ct: &CheckType) -> SeverityTier {
    use crate::core::lint::LintRuleId;
    match ct {
        // Critical: broken references, structural violations.
        CheckType::CircularRef | CheckType::OrphanedGlob => SeverityTier::Critical,
        // Drift: code changed relative to spec.
        CheckType::ChangedSinceReconciliation => SeverityTier::Drift,
        // Housekeeping: organizational, style, coverage.
        CheckType::CoverageGap
        | CheckType::OverlappingCoverage
        | CheckType::BehaviorIncomplete
        | CheckType::TestCoverageMissing => SeverityTier::Housekeeping,
        // Test alignment checks.
        CheckType::TestPathMissing => SeverityTier::Critical,
        // Lint rules mapped individually.
        CheckType::LintViolation(rule_id) => match rule_id {
            LintRuleId::L004 | LintRuleId::L009 => SeverityTier::Critical,
            LintRuleId::L001 | LintRuleId::L010 => SeverityTier::Drift,
            LintRuleId::L011 => SeverityTier::Critical,
            LintRuleId::L002
            | LintRuleId::L003
            | LintRuleId::L005
            | LintRuleId::L006
            | LintRuleId::L007
            | LintRuleId::L008 => SeverityTier::Housekeeping,
        },
    }
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
    findings.extend(check_test_alignment(project_root, &loaded_specs));

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
            tier: SeverityTier::Housekeeping,
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
                        tier: SeverityTier::Critical,
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
            tier: SeverityTier::Drift,
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
                tier: SeverityTier::Housekeeping,
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
/// Delegates to the shared cycle detection in `core::lint` and wraps results as
/// `CheckFinding` values.
fn check_circular_refs(loaded_specs: &[(String, serde_json::Value)]) -> Vec<CheckFinding> {
    crate::core::lint::detect_ref_cycles(loaded_specs)
        .into_iter()
        .map(|cycle| {
            let display = cycle.join(" -> ");
            CheckFinding {
                check_type: CheckType::CircularRef,
                severity: Severity::Error,
                tier: SeverityTier::Critical,
                spec_path: Some(cycle[0].clone()),
                file_path: None,
                glob_pattern: None,
                message: format!("Circular $ref chain: {display}"),
            }
        })
        .collect()
}

/// Flag behaviors missing a `given` or `then` field (or where either is blank).
///
/// Delegates to the shared detection in `core::lint` and wraps results as
/// `CheckFinding` values.
fn check_behavior_completeness(loaded_specs: &[(String, serde_json::Value)]) -> Vec<CheckFinding> {
    crate::core::lint::detect_incomplete_behaviors(loaded_specs)
        .into_iter()
        .map(|(spec_rel, name, field)| CheckFinding {
            check_type: CheckType::BehaviorIncomplete,
            severity: Severity::Warning,
            tier: SeverityTier::Housekeeping,
            spec_path: Some(spec_rel.clone()),
            file_path: None,
            glob_pattern: None,
            message: format!("Behavior '{name}' missing '{field}' field (in {spec_rel})"),
        })
        .collect()
}

/// T001-T002: Test-spec alignment checks.
///
/// - T001 (Housekeeping): A tier-1 behavior has no `tested_by` entry.
/// - T002 (Critical): A `tested_by.path` does not exist on disk.
///
/// T003 (mtime-based staleness) was removed: filesystem timestamps cannot
/// distinguish a test written before its code (TDD) from a genuinely stale
/// test, producing false positives in normal TDD workflows. A reliable
/// replacement would need the hash cache to determine whether code changed
/// after the last reconciliation; that is tracked as an open question.
fn check_test_alignment(
    project_root: &Path,
    loaded_specs: &[(String, serde_json::Value)],
) -> Vec<CheckFinding> {
    let mut findings = Vec::new();

    for (spec_rel, spec_value) in loaded_specs {
        // Only tier-1 (full) specs participate. Absent tier field defaults to full.
        let tier = spec_value
            .get("tier")
            .and_then(|v| v.as_str())
            .unwrap_or("full");
        if tier != "full" {
            continue;
        }
        // Cross-cutting specs govern no code directly; their behaviors express
        // invariants applied via `applies` to other specs, so per-behavior
        // tested_by entries don't carry the same meaning.
        if spec_value
            .get("cross_cutting")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            continue;
        }
        // Specs with no `artifacts.code` entries govern docs/configs/assets
        // that aren't code-tested; T001 doesn't apply to them. T002 (test path
        // missing) still fires below, since it only runs on behaviors that
        // explicitly declare `tested_by`.
        let has_code_artifacts = spec_value
            .get("artifacts")
            .and_then(|a| a.get("code"))
            .and_then(|c| c.as_array())
            .is_some_and(|arr| !arr.is_empty());
        let Some(behaviors) = spec_value.get("behaviors").and_then(|b| b.as_array()) else {
            continue;
        };

        for b in behaviors {
            let name = b
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or("<unnamed>");
            let tested_by = b.get("tested_by").and_then(|t| t.as_array());

            match tested_by {
                None if has_code_artifacts => {
                    findings.push(CheckFinding {
                        check_type: CheckType::TestCoverageMissing,
                        severity: Severity::Warning,
                        tier: SeverityTier::Housekeeping,
                        spec_path: Some(spec_rel.clone()),
                        file_path: None,
                        glob_pattern: None,
                        message: format!("T001: Behavior '{name}' has no tested_by entry"),
                    });
                }
                None => {}
                Some(arr) => {
                    for entry in arr {
                        let Some(path) = entry.get("path").and_then(|p| p.as_str()) else {
                            continue;
                        };
                        let full = project_root.join(path);
                        if !full.exists() {
                            findings.push(CheckFinding {
                                check_type: CheckType::TestPathMissing,
                                severity: Severity::Error,
                                tier: SeverityTier::Critical,
                                spec_path: Some(spec_rel.clone()),
                                file_path: Some(path.to_string()),
                                glob_pattern: None,
                                message: format!(
                                    "T002: Test path does not exist: {path} (behavior '{name}' in {spec_rel})"
                                ),
                            });
                        }
                    }
                }
            }
        }
    }

    findings
}
