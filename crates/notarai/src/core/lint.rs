use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::core::spec_loader;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LintRuleId {
    L001,
    L002,
    L003,
    L004,
    L005,
    L006,
    L007,
    L008,
    L009,
    L010,
}

impl LintRuleId {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::L001 => "L001",
            Self::L002 => "L002",
            Self::L003 => "L003",
            Self::L004 => "L004",
            Self::L005 => "L005",
            Self::L006 => "L006",
            Self::L007 => "L007",
            Self::L008 => "L008",
            Self::L009 => "L009",
            Self::L010 => "L010",
        }
    }
}

impl std::fmt::Display for LintRuleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LintSeverity {
    Error,
    Warning,
    Info,
}

impl LintSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Info => "info",
        }
    }
}

#[derive(Debug, Clone)]
pub struct LintFinding {
    pub rule_id: LintRuleId,
    pub severity: LintSeverity,
    pub spec_path: String,
    pub message: String,
}

/// Per-rule configuration.
#[derive(Debug, Clone)]
pub struct RuleConfig {
    pub enabled: bool,
    pub severity: Option<LintSeverity>,
    /// L006-specific: number of days before a rationale-less decision triggers.
    pub decision_age_days: Option<u64>,
}

impl Default for RuleConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            severity: None,
            decision_age_days: None,
        }
    }
}

/// Lint configuration loaded from `.notarai/lint.yaml`.
#[derive(Debug, Clone, Default)]
pub struct LintConfig {
    pub rules: HashMap<LintRuleId, RuleConfig>,
}

impl LintConfig {
    /// Load config from `.notarai/lint.yaml`. Returns defaults if file is missing.
    pub fn load(project_root: &Path) -> Self {
        let config_path = project_root.join(".notarai/lint.yaml");
        let content = match std::fs::read_to_string(&config_path) {
            Ok(c) => c,
            Err(_) => return Self::default(),
        };
        let value: serde_json::Value = match serde_yaml_ng::from_str(&content) {
            Ok(v) => v,
            Err(_) => return Self::default(),
        };

        let mut config = Self::default();
        let Some(rules_obj) = value.get("rules").and_then(|r| r.as_object()) else {
            return config;
        };

        for (key, val) in rules_obj {
            let Some(rule_id) = parse_rule_id(key) else {
                continue;
            };
            let mut rc = RuleConfig::default();
            if let Some(enabled) = val.get("enabled").and_then(|e| e.as_bool()) {
                rc.enabled = enabled;
            }
            if let Some(sev_str) = val.get("severity").and_then(|s| s.as_str()) {
                rc.severity = parse_severity(sev_str);
            }
            if let Some(days) = val.get("decision_age_days").and_then(|d| d.as_u64()) {
                rc.decision_age_days = Some(days);
            }
            config.rules.insert(rule_id, rc);
        }

        config
    }

    fn is_enabled(&self, rule: LintRuleId) -> bool {
        self.rules.get(&rule).map(|rc| rc.enabled).unwrap_or(true)
    }

    fn effective_severity(&self, rule: LintRuleId, default: LintSeverity) -> LintSeverity {
        self.rules
            .get(&rule)
            .and_then(|rc| rc.severity)
            .unwrap_or(default)
    }

    fn decision_age_days(&self) -> u64 {
        self.rules
            .get(&LintRuleId::L006)
            .and_then(|rc| rc.decision_age_days)
            .unwrap_or(90)
    }
}

fn parse_rule_id(s: &str) -> Option<LintRuleId> {
    match s {
        "L001" => Some(LintRuleId::L001),
        "L002" => Some(LintRuleId::L002),
        "L003" => Some(LintRuleId::L003),
        "L004" => Some(LintRuleId::L004),
        "L005" => Some(LintRuleId::L005),
        "L006" => Some(LintRuleId::L006),
        "L007" => Some(LintRuleId::L007),
        "L008" => Some(LintRuleId::L008),
        "L009" => Some(LintRuleId::L009),
        "L010" => Some(LintRuleId::L010),
        _ => None,
    }
}

fn parse_severity(s: &str) -> Option<LintSeverity> {
    match s {
        "error" => Some(LintSeverity::Error),
        "warning" => Some(LintSeverity::Warning),
        "info" => Some(LintSeverity::Info),
        _ => None,
    }
}

/// Run all lint rules against specs in the project. Never modifies files.
pub fn run_all_lints(project_root: &Path, config: &LintConfig) -> Result<Vec<LintFinding>, String> {
    let specs = spec_loader::collect_specs(project_root)?;

    let mut loaded_specs = Vec::new();
    for spec_path in &specs {
        let spec_rel = spec_path
            .strip_prefix(project_root)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| spec_path.to_string_lossy().to_string());
        let spec_value = spec_loader::load_spec(spec_path)?;
        loaded_specs.push((spec_rel, spec_value));
    }

    let mut findings = Vec::new();

    if config.is_enabled(LintRuleId::L001) {
        findings.extend(lint_l001_no_behaviors(&loaded_specs, config));
    }
    if config.is_enabled(LintRuleId::L002) {
        findings.extend(lint_l002_missing_given(&loaded_specs, config));
    }
    if config.is_enabled(LintRuleId::L003) {
        findings.extend(lint_l003_missing_then(&loaded_specs, config));
    }
    if config.is_enabled(LintRuleId::L004) {
        findings.extend(lint_l004_ref_missing(&loaded_specs, project_root, config));
    }
    if config.is_enabled(LintRuleId::L005) {
        findings.extend(lint_l005_circular_ref(&loaded_specs, config));
    }
    if config.is_enabled(LintRuleId::L006) {
        findings.extend(lint_l006_stale_decision(&loaded_specs, config));
    }
    if config.is_enabled(LintRuleId::L007) {
        findings.extend(lint_l007_open_questions(&loaded_specs, config));
    }
    if config.is_enabled(LintRuleId::L008) {
        findings.extend(lint_l008_broad_glob(&loaded_specs, config));
    }
    if config.is_enabled(LintRuleId::L009) {
        findings.extend(lint_l009_schema_mismatch(&loaded_specs, config));
    }
    if config.is_enabled(LintRuleId::L010) {
        findings.extend(lint_l010_duplicate_behaviors(&loaded_specs, config));
    }

    Ok(findings)
}

// ---------------------------------------------------------------------------
// Shared detection helpers (used by both lint and check)
// ---------------------------------------------------------------------------

/// Detect behaviors missing a `given` or `then` field (or where the field is blank).
/// Returns tuples of (spec_rel_path, behavior_name, missing_field).
pub fn detect_incomplete_behaviors(
    loaded_specs: &[(String, serde_json::Value)],
) -> Vec<(String, String, String)> {
    let mut results = Vec::new();
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
                    results.push((spec_rel.clone(), name.clone(), field.to_string()));
                }
            }
        }
    }
    results
}

/// Build a directed graph of `$ref` chains and detect cycles.
/// Returns a list of cycles, each represented as a path of spec-relative paths
/// (e.g., `["a.spec.yaml", "b.spec.yaml", "a.spec.yaml"]`).
pub fn detect_ref_cycles(loaded_specs: &[(String, serde_json::Value)]) -> Vec<Vec<String>> {
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

    let mut color: HashMap<&str, u8> = loaded_specs.iter().map(|(p, _)| (p.as_str(), 0)).collect();
    let mut reported: HashSet<Vec<String>> = HashSet::new();
    let mut cycles = Vec::new();

    let mut roots: Vec<&str> = loaded_specs.iter().map(|(p, _)| p.as_str()).collect();
    roots.sort();

    for root in roots {
        if color.get(root).copied().unwrap_or(2) != 0 {
            continue;
        }
        let mut stack: Vec<String> = vec![root.to_string()];
        dfs_find_cycles(&graph, &mut color, &mut stack, &mut reported, &mut cycles);
    }

    cycles
}

/// Resolve a `$ref` string relative to the containing spec's parent directory.
pub fn resolve_ref_path(parent_dir: &Path, ref_str: &str) -> String {
    if ref_str.starts_with("./") || ref_str.starts_with("../") {
        let joined = parent_dir.join(ref_str);
        normalize_path(&joined)
    } else {
        ref_str.to_string()
    }
}

/// Collapse `.` and `..` components without touching the filesystem.
pub fn normalize_path(path: &Path) -> String {
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
    cycles: &mut Vec<Vec<String>>,
) {
    let node = match stack.last() {
        Some(n) => n.clone(),
        None => return,
    };

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
                    dfs_find_cycles(graph, color, stack, reported, cycles);
                    stack.pop();
                }
                1 => {
                    if let Some(start) = stack.iter().position(|s| s == &next) {
                        let mut cycle: Vec<String> = stack[start..].to_vec();
                        cycle.push(next.clone());
                        let canonical = canonicalize_cycle(&cycle);
                        if reported.insert(canonical) {
                            cycles.push(cycle);
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

/// Collect all `$ref` targets from `subsystems`, `applies`, and `dependencies`.
fn collect_refs(spec_value: &serde_json::Value) -> Vec<String> {
    let mut refs = Vec::new();
    for field in ["subsystems", "applies", "dependencies"] {
        let Some(arr) = spec_value.get(field).and_then(|v| v.as_array()) else {
            continue;
        };
        for item in arr {
            if let Some(ref_str) = item.get("$ref").and_then(|r| r.as_str()) {
                refs.push(ref_str.to_string());
            }
        }
    }
    refs
}

/// Get the bundled schema version string from the schema_version enum's first entry.
fn bundled_version() -> Option<&'static str> {
    crate::core::schema::schema()
        .get("properties")
        .and_then(|p| p.get("schema_version"))
        .and_then(|sv| sv.get("enum"))
        .and_then(|e| e.as_array())
        .and_then(|arr| arr.first())
        .and_then(|v| v.as_str())
}

// ---------------------------------------------------------------------------
// Individual lint rules
// ---------------------------------------------------------------------------

/// L001: Tier 1 (full) spec has zero behaviors.
fn lint_l001_no_behaviors(
    loaded_specs: &[(String, serde_json::Value)],
    config: &LintConfig,
) -> Vec<LintFinding> {
    let severity = config.effective_severity(LintRuleId::L001, LintSeverity::Error);
    let mut findings = Vec::new();

    for (spec_rel, spec_value) in loaded_specs {
        let tier = spec_value
            .get("tier")
            .and_then(|t| t.as_str())
            .unwrap_or("full");

        if tier != "full" {
            continue;
        }

        let has_behaviors = spec_value
            .get("behaviors")
            .and_then(|b| b.as_array())
            .is_some_and(|arr| !arr.is_empty());

        if !has_behaviors {
            findings.push(LintFinding {
                rule_id: LintRuleId::L001,
                severity,
                spec_path: spec_rel.clone(),
                message: format!("Tier 1 (full) spec has zero behaviors: {spec_rel}"),
            });
        }
    }

    findings
}

/// L002: Behavior missing `given` field.
fn lint_l002_missing_given(
    loaded_specs: &[(String, serde_json::Value)],
    config: &LintConfig,
) -> Vec<LintFinding> {
    let severity = config.effective_severity(LintRuleId::L002, LintSeverity::Warning);
    detect_incomplete_behaviors(loaded_specs)
        .into_iter()
        .filter(|(_, _, field)| field == "given")
        .map(|(spec_rel, name, _)| LintFinding {
            rule_id: LintRuleId::L002,
            severity,
            spec_path: spec_rel.clone(),
            message: format!("Behavior '{name}' missing 'given' field (in {spec_rel})"),
        })
        .collect()
}

/// L003: Behavior missing `then` field.
fn lint_l003_missing_then(
    loaded_specs: &[(String, serde_json::Value)],
    config: &LintConfig,
) -> Vec<LintFinding> {
    let severity = config.effective_severity(LintRuleId::L003, LintSeverity::Warning);
    detect_incomplete_behaviors(loaded_specs)
        .into_iter()
        .filter(|(_, _, field)| field == "then")
        .map(|(spec_rel, name, _)| LintFinding {
            rule_id: LintRuleId::L003,
            severity,
            spec_path: spec_rel.clone(),
            message: format!("Behavior '{name}' missing 'then' field (in {spec_rel})"),
        })
        .collect()
}

/// L004: `$ref` target file does not exist on disk.
fn lint_l004_ref_missing(
    loaded_specs: &[(String, serde_json::Value)],
    project_root: &Path,
    config: &LintConfig,
) -> Vec<LintFinding> {
    let severity = config.effective_severity(LintRuleId::L004, LintSeverity::Error);
    let mut findings = Vec::new();

    for (spec_rel, spec_value) in loaded_specs {
        let parent_dir = Path::new(spec_rel)
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_default();

        for ref_str in collect_refs(spec_value) {
            let resolved = resolve_ref_path(&parent_dir, &ref_str);
            let abs_path = project_root.join(&resolved);
            if !abs_path.exists() {
                findings.push(LintFinding {
                    rule_id: LintRuleId::L004,
                    severity,
                    spec_path: spec_rel.clone(),
                    message: format!(
                        "$ref target does not exist: {ref_str} (resolved to {resolved}, in {spec_rel})"
                    ),
                });
            }
        }
    }

    findings
}

/// L005: Circular `$ref` dependency detected.
fn lint_l005_circular_ref(
    loaded_specs: &[(String, serde_json::Value)],
    config: &LintConfig,
) -> Vec<LintFinding> {
    let severity = config.effective_severity(LintRuleId::L005, LintSeverity::Warning);
    detect_ref_cycles(loaded_specs)
        .into_iter()
        .map(|cycle| {
            let display = cycle.join(" -> ");
            LintFinding {
                rule_id: LintRuleId::L005,
                severity,
                spec_path: cycle[0].clone(),
                message: format!("Circular $ref dependency: {display}"),
            }
        })
        .collect()
}

/// L006: Decision older than threshold with no rationale.
fn lint_l006_stale_decision(
    loaded_specs: &[(String, serde_json::Value)],
    config: &LintConfig,
) -> Vec<LintFinding> {
    let severity = config.effective_severity(LintRuleId::L006, LintSeverity::Warning);
    let max_age_days = config.decision_age_days();
    let mut findings = Vec::new();

    // Use a simple date comparison: parse YYYY-MM-DD and compute days since.
    let today = chrono_free_today();

    for (spec_rel, spec_value) in loaded_specs {
        let Some(decisions) = spec_value.get("decisions").and_then(|d| d.as_array()) else {
            continue;
        };
        for (idx, decision) in decisions.iter().enumerate() {
            let has_rationale = decision
                .get("rationale")
                .and_then(|r| r.as_str())
                .is_some_and(|s| !s.trim().is_empty());

            if has_rationale {
                continue;
            }

            let Some(date_str) = decision.get("date").and_then(|d| d.as_str()) else {
                continue;
            };

            let Some(date) = parse_date(date_str) else {
                continue;
            };

            let age_days = days_between(date, today);
            if age_days > max_age_days as i64 {
                let choice = decision
                    .get("choice")
                    .and_then(|c| c.as_str())
                    .unwrap_or("unknown");
                findings.push(LintFinding {
                    rule_id: LintRuleId::L006,
                    severity,
                    spec_path: spec_rel.clone(),
                    message: format!(
                        "Decision #{} ('{choice}', {date_str}) is {age_days} days old with no rationale (in {spec_rel})",
                        idx + 1
                    ),
                });
            }
        }
    }

    findings
}

/// L007: Spec has open_questions.
fn lint_l007_open_questions(
    loaded_specs: &[(String, serde_json::Value)],
    config: &LintConfig,
) -> Vec<LintFinding> {
    let severity = config.effective_severity(LintRuleId::L007, LintSeverity::Info);
    let mut findings = Vec::new();

    for (spec_rel, spec_value) in loaded_specs {
        let count = spec_value
            .get("open_questions")
            .and_then(|q| q.as_array())
            .map(|arr| arr.len())
            .unwrap_or(0);

        if count > 0 {
            findings.push(LintFinding {
                rule_id: LintRuleId::L007,
                severity,
                spec_path: spec_rel.clone(),
                message: format!(
                    "Spec has {count} open question{}: {spec_rel}",
                    if count == 1 { "" } else { "s" }
                ),
            });
        }
    }

    findings
}

/// L008: Artifact glob uses `**/*` (overly broad).
fn lint_l008_broad_glob(
    loaded_specs: &[(String, serde_json::Value)],
    config: &LintConfig,
) -> Vec<LintFinding> {
    let severity = config.effective_severity(LintRuleId::L008, LintSeverity::Warning);
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
                let Some(pattern) = item.get("path").and_then(|p| p.as_str()) else {
                    continue;
                };
                if pattern == "**/*" {
                    findings.push(LintFinding {
                        rule_id: LintRuleId::L008,
                        severity,
                        spec_path: spec_rel.clone(),
                        message: format!("Artifact glob '**/*' is overly broad (in {spec_rel})"),
                    });
                }
            }
        }
    }

    findings
}

/// L009: schema_version does not match bundled schema.
fn lint_l009_schema_mismatch(
    loaded_specs: &[(String, serde_json::Value)],
    config: &LintConfig,
) -> Vec<LintFinding> {
    let severity = config.effective_severity(LintRuleId::L009, LintSeverity::Error);
    let Some(bundled) = bundled_version() else {
        return vec![];
    };
    let mut findings = Vec::new();

    for (spec_rel, spec_value) in loaded_specs {
        let spec_version = spec_value
            .get("schema_version")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if spec_version != bundled {
            findings.push(LintFinding {
                rule_id: LintRuleId::L009,
                severity,
                spec_path: spec_rel.clone(),
                message: format!(
                    "schema_version '{spec_version}' does not match bundled '{bundled}' (in {spec_rel})"
                ),
            });
        }
    }

    findings
}

/// L010: Duplicate behavior names within a spec.
fn lint_l010_duplicate_behaviors(
    loaded_specs: &[(String, serde_json::Value)],
    config: &LintConfig,
) -> Vec<LintFinding> {
    let severity = config.effective_severity(LintRuleId::L010, LintSeverity::Warning);
    let mut findings = Vec::new();

    for (spec_rel, spec_value) in loaded_specs {
        let Some(behaviors) = spec_value.get("behaviors").and_then(|b| b.as_array()) else {
            continue;
        };

        let mut seen: HashMap<&str, usize> = HashMap::new();
        for behavior in behaviors {
            if let Some(name) = behavior.get("name").and_then(|n| n.as_str()) {
                *seen.entry(name).or_insert(0) += 1;
            }
        }

        for (name, count) in &seen {
            if *count > 1 {
                findings.push(LintFinding {
                    rule_id: LintRuleId::L010,
                    severity,
                    spec_path: spec_rel.clone(),
                    message: format!(
                        "Duplicate behavior name '{name}' ({count} occurrences, in {spec_rel})"
                    ),
                });
            }
        }
    }

    findings
}

// ---------------------------------------------------------------------------
// Simple date helpers (no chrono dependency)
// ---------------------------------------------------------------------------

/// (year, month, day)
type SimpleDate = (i64, u32, u32);

fn chrono_free_today() -> SimpleDate {
    // Read from environment for testing, otherwise use system time.
    if let Ok(s) = std::env::var("NOTARAI_TODAY")
        && let Some(d) = parse_date(&s)
    {
        return d;
    }

    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    // Convert epoch seconds to a date. Simple civil-day calculation.
    epoch_to_date(secs)
}

fn parse_date(s: &str) -> Option<SimpleDate> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 3 {
        return None;
    }
    let year: i64 = parts[0].parse().ok()?;
    let month: u32 = parts[1].parse().ok()?;
    let day: u32 = parts[2].parse().ok()?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    Some((year, month, day))
}

fn epoch_to_date(epoch_secs: i64) -> SimpleDate {
    // Algorithm from Howard Hinnant's civil_from_days.
    let z = epoch_secs / 86400 + 719468;
    let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m as u32, d as u32)
}

fn date_to_days(d: SimpleDate) -> i64 {
    let (y, m, day) = d;
    let y = if m <= 2 { y - 1 } else { y };
    let era = (if y >= 0 { y } else { y - 399 }) / 400;
    let yoe = y - era * 400;
    let m_adj = if m > 2 { m as i64 - 3 } else { m as i64 + 9 };
    let doy = (153 * m_adj + 2) / 5 + day as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

fn days_between(from: SimpleDate, to: SimpleDate) -> i64 {
    date_to_days(to) - date_to_days(from)
}
