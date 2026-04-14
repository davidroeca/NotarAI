use std::collections::HashSet;
use std::path::Path;

use crate::core::decisions::DecisionLog;
use crate::core::spec_loader;

/// Weights for scoring signals. Configurable via `.notarai/scoring.yaml`.
#[derive(Debug, Clone)]
pub struct ScoringConfig {
    pub files_changed: f64,
    pub days_since_reconciliation: f64,
    pub unresolved_decisions: f64,
    pub orphaned_globs: f64,
    pub open_questions: f64,
    pub unspecced_files: f64,
}

impl Default for ScoringConfig {
    fn default() -> Self {
        ScoringConfig {
            files_changed: 0.3,
            days_since_reconciliation: 0.2,
            unresolved_decisions: 0.15,
            orphaned_globs: 0.15,
            open_questions: 0.1,
            unspecced_files: 0.1,
        }
    }
}

impl ScoringConfig {
    pub fn load(project_root: &Path) -> Self {
        let config_path = project_root.join(".notarai/scoring.yaml");
        let content = match std::fs::read_to_string(&config_path) {
            Ok(c) => c,
            Err(_) => return ScoringConfig::default(),
        };
        let value: serde_json::Value = match serde_yaml_ng::from_str(&content) {
            Ok(v) => v,
            Err(_) => return ScoringConfig::default(),
        };
        let get = |key: &str, default: f64| -> f64 {
            value.get(key).and_then(|v| v.as_f64()).unwrap_or(default)
        };
        let defaults = ScoringConfig::default();
        ScoringConfig {
            files_changed: get("files_changed", defaults.files_changed),
            days_since_reconciliation: get(
                "days_since_reconciliation",
                defaults.days_since_reconciliation,
            ),
            unresolved_decisions: get("unresolved_decisions", defaults.unresolved_decisions),
            orphaned_globs: get("orphaned_globs", defaults.orphaned_globs),
            open_questions: get("open_questions", defaults.open_questions),
            unspecced_files: get("unspecced_files", defaults.unspecced_files),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SpecScore {
    pub spec_path: String,
    pub score: f64,
    pub status: &'static str,
}

impl SpecScore {
    pub fn status_from_score(score: f64) -> &'static str {
        if score < 0.3 {
            "healthy"
        } else if score < 0.6 {
            "review"
        } else {
            "overdue"
        }
    }
}

pub struct OverallScore {
    pub specs: Vec<SpecScore>,
    pub overall: f64,
    pub status: &'static str,
}

/// Compute drift scores for all specs (or a single spec if filter is set).
pub fn compute_scores(
    project_root: &Path,
    config: &ScoringConfig,
    filter_spec: Option<&str>,
) -> Result<OverallScore, String> {
    let all_specs = spec_loader::collect_specs(project_root)?;
    let exclude_patterns = spec_loader::get_exclude_patterns(project_root)?;
    let decision_log = DecisionLog::load(project_root);

    // State file for days-since-reconciliation.
    let state_path = project_root.join(".notarai/reconciliation_state.json");
    let state_value: Option<serde_json::Value> = std::fs::read_to_string(&state_path)
        .ok()
        .and_then(|c| serde_json::from_str(&c).ok());
    let days_since = state_value
        .as_ref()
        .and_then(|v| v.get("metadata"))
        .and_then(|m| m.get("timestamp"))
        .and_then(|t| t.as_str())
        .and_then(parse_days_since)
        .unwrap_or(30.0); // Default to 30 days if no state.

    // Cache for changed file detection.
    let db_path = crate::core::cache::db_path(project_root);
    let conn = if db_path.exists() {
        crate::core::cache::open_cache_db(project_root).ok()
    } else {
        None
    };

    // Git tracked files for coverage gap computation.
    let tracked = crate::core::git::tracked_files(project_root).unwrap_or_default();
    let exclude_matchers: Vec<glob::Pattern> = exclude_patterns
        .iter()
        .filter_map(|p| glob::Pattern::new(p).ok())
        .collect();

    // Collect all governed files across all specs.
    let mut all_governed: HashSet<String> = HashSet::new();
    let mut spec_data: Vec<(String, serde_json::Value, Vec<String>)> = Vec::new();

    for spec_path in &all_specs {
        let spec_rel = spec_path
            .strip_prefix(project_root)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| spec_path.to_string_lossy().to_string());

        if let Some(filter) = filter_spec
            && spec_rel != filter
        {
            continue;
        }

        let spec_value = spec_loader::load_spec(spec_path)?;
        let files = spec_loader::expand_artifact_globs(&spec_value, project_root);
        all_governed.extend(files.iter().cloned());
        spec_data.push((spec_rel, spec_value, files));
    }

    let mut scores = Vec::new();

    for (spec_rel, spec_value, governed_files) in &spec_data {
        let total_governed = governed_files.len().max(1) as f64;

        // Signal 1: files changed since reconciliation.
        let changed_ratio = if let Some(ref c) = conn {
            let pairs: Vec<(String, std::path::PathBuf)> = governed_files
                .iter()
                .map(|rel| (rel.clone(), project_root.join(rel)))
                .collect();
            let (changed, _) =
                crate::core::cache::check_changed_batch(c, &pairs).unwrap_or((vec![], vec![]));
            (changed.len() as f64 / total_governed).min(1.0)
        } else {
            1.0 // No cache = assume all changed.
        };

        // Signal 2: days since reconciliation (normalized to 0-1 over 90 days).
        let days_ratio = (days_since / 90.0).min(1.0);

        // Signal 3: unresolved decisions for this spec.
        let unresolved = decision_log
            .proposals
            .iter()
            .filter(|p| p.spec_path == *spec_rel && p.status == "proposed")
            .count();
        let decisions_ratio = (unresolved as f64 / 5.0).min(1.0); // Normalize to 5 max.

        // Signal 4: orphaned globs.
        let orphaned = count_orphaned_globs(spec_value, project_root);
        let total_globs = count_total_globs(spec_value).max(1);
        let orphaned_ratio = (orphaned as f64 / total_globs as f64).min(1.0);

        // Signal 5: open questions.
        let open_q = spec_value
            .get("open_questions")
            .and_then(|v| v.as_array())
            .map(|a| a.len())
            .unwrap_or(0);
        let open_q_ratio = (open_q as f64 / 5.0).min(1.0);

        // Signal 6: unspecced files in governed directories.
        let governed_dirs: HashSet<&str> = governed_files
            .iter()
            .filter_map(|f| f.rsplit_once('/').map(|(d, _)| d))
            .collect();
        let unspecced_in_dirs = tracked
            .iter()
            .filter(|f| {
                if f.starts_with(".notarai/") || f.starts_with(".git/") {
                    return false;
                }
                if all_governed.contains(f.as_str()) {
                    return false;
                }
                if exclude_matchers.iter().any(|p| p.matches(f)) {
                    return false;
                }
                f.rsplit_once('/')
                    .map(|(d, _)| governed_dirs.contains(d))
                    .unwrap_or(false)
            })
            .count();
        let unspecced_ratio = (unspecced_in_dirs as f64 / total_governed).min(1.0);

        let score = config.files_changed * changed_ratio
            + config.days_since_reconciliation * days_ratio
            + config.unresolved_decisions * decisions_ratio
            + config.orphaned_globs * orphaned_ratio
            + config.open_questions * open_q_ratio
            + config.unspecced_files * unspecced_ratio;

        let score = score.min(1.0);

        scores.push(SpecScore {
            spec_path: spec_rel.clone(),
            score,
            status: SpecScore::status_from_score(score),
        });
    }

    // Sort by score descending (most drifted first).
    scores.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let overall = if scores.is_empty() {
        0.0
    } else {
        scores.iter().map(|s| s.score).sum::<f64>() / scores.len() as f64
    };

    Ok(OverallScore {
        specs: scores,
        overall,
        status: SpecScore::status_from_score(overall),
    })
}

fn count_orphaned_globs(spec_value: &serde_json::Value, project_root: &Path) -> usize {
    let Some(artifacts) = spec_value.get("artifacts").and_then(|a| a.as_object()) else {
        return 0;
    };
    let mut count = 0;
    for (_category, refs) in artifacts {
        let Some(arr) = refs.as_array() else {
            continue;
        };
        for item in arr {
            if let Some(pattern) = item.get("path").and_then(|p| p.as_str())
                && spec_loader::expand_glob(pattern, project_root).is_empty()
            {
                count += 1;
            }
        }
    }
    count
}

fn count_total_globs(spec_value: &serde_json::Value) -> usize {
    let Some(artifacts) = spec_value.get("artifacts").and_then(|a| a.as_object()) else {
        return 0;
    };
    let mut count = 0;
    for (_category, refs) in artifacts {
        if let Some(arr) = refs.as_array() {
            count += arr.len();
        }
    }
    count
}

/// Parse an ISO 8601 timestamp and return the number of days since then.
fn parse_days_since(timestamp: &str) -> Option<f64> {
    // Expect format like "2026-04-13T12:00:00Z" or "2026-04-13".
    let date_str = if timestamp.len() >= 10 {
        &timestamp[..10]
    } else {
        return None;
    };
    let parts: Vec<&str> = date_str.split('-').collect();
    if parts.len() != 3 {
        return None;
    }
    let y: i64 = parts[0].parse().ok()?;
    let m: i64 = parts[1].parse().ok()?;
    let d: i64 = parts[2].parse().ok()?;
    let then_days = civil_days(y, m, d);

    // Get today using the same approach as lint (supports NOTARAI_TODAY env var).
    let today = std::env::var("NOTARAI_TODAY")
        .ok()
        .and_then(|s| {
            let p: Vec<&str> = s.split('-').collect();
            if p.len() == 3 {
                Some(civil_days(
                    p[0].parse().ok()?,
                    p[1].parse().ok()?,
                    p[2].parse().ok()?,
                ))
            } else {
                None
            }
        })
        .unwrap_or_else(|| {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
            now / 86400
        });

    let diff = today - then_days;
    Some(if diff < 0 { 0.0 } else { diff as f64 })
}

/// Convert a civil date to days since epoch (Howard Hinnant's algorithm).
fn civil_days(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u64;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) as u64 + 2) / 5 + d as u64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe as i64 - 719468
}
