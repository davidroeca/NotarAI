use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// On-disk reconciliation state, committed to the repo.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct ReconciliationState {
    pub schema_version: String, // always "1"
    pub last_reconciliation: ReconciliationMeta,
    pub file_fingerprints: BTreeMap<String, FileFingerprint>,
    pub spec_fingerprints: BTreeMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct ReconciliationMeta {
    pub timestamp: String,        // ISO 8601 (Unix epoch seconds)
    pub git_hash: Option<String>, // HEAD at reconciliation time
    pub branch: Option<String>,   // branch name at reconciliation time
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct FileFingerprint {
    pub blake3: String,
}

/// Canonical path: `<root>/.notarai/reconciliation_state.json`
pub fn state_path(project_root: &Path) -> PathBuf {
    project_root
        .join(".notarai")
        .join("reconciliation_state.json")
}

/// Load state from disk. Returns `None` if the file doesn't exist.
/// Returns `Err` on read/parse failure.
pub fn load_state(project_root: &Path) -> Result<Option<ReconciliationState>, String> {
    let path = state_path(project_root);
    if !path.exists() {
        return Ok(None);
    }
    let content =
        std::fs::read_to_string(&path).map_err(|e| format!("could not read state file: {e}"))?;
    let state: ReconciliationState =
        serde_json::from_str(&content).map_err(|e| format!("could not parse state file: {e}"))?;
    Ok(Some(state))
}

/// Write state to disk (pretty-printed JSON for diffability).
pub fn save_state(project_root: &Path, state: &ReconciliationState) -> Result<(), String> {
    let path = state_path(project_root);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("could not create .notarai directory: {e}"))?;
    }
    let json = serde_json::to_string_pretty(state)
        .map_err(|e| format!("could not serialize state: {e}"))?;
    std::fs::write(&path, json).map_err(|e| format!("could not write state file: {e}"))?;
    Ok(())
}

/// Build a `ReconciliationState` from the current SQLite cache and git context.
///
/// - Reads all (path, blake3_hash) rows from the cache.
/// - Partitions: `.spec.yaml` paths go to `spec_fingerprints`, everything else
///   to `file_fingerprints`.
/// - Captures git HEAD hash and current branch.
pub fn snapshot_from_cache(project_root: &Path) -> Result<ReconciliationState, String> {
    let conn = crate::core::cache::open_cache_db(project_root)?;
    let rows = crate::core::cache::read_all(&conn)?;

    let mut file_fingerprints = BTreeMap::new();
    let mut spec_fingerprints = BTreeMap::new();

    for (path, hash) in rows {
        if path.ends_with(".spec.yaml") {
            spec_fingerprints.insert(path, hash);
        } else {
            file_fingerprints.insert(path, FileFingerprint { blake3: hash });
        }
    }

    let git_hash = git_head(project_root);
    let branch = git_branch(project_root);

    Ok(ReconciliationState {
        schema_version: "1".to_string(),
        last_reconciliation: ReconciliationMeta {
            timestamp: utc_timestamp(),
            git_hash,
            branch,
        },
        file_fingerprints,
        spec_fingerprints,
    })
}

/// Compare current file hashes against a stored state.
/// Returns lists of added/modified/removed/spec_changed/spec_unchanged paths.
#[allow(dead_code)]
pub struct StateDelta {
    pub added: Vec<String>,
    pub modified: Vec<String>,
    pub removed: Vec<String>,
    pub spec_changed: Vec<String>,
    pub spec_unchanged: Vec<String>,
}

#[allow(dead_code)]
pub fn diff_against_state(
    state: &ReconciliationState,
    current_files: &[(String, String)], // (rel_path, blake3_hash)
) -> StateDelta {
    let mut added = Vec::new();
    let mut modified = Vec::new();
    let mut removed = Vec::new();
    let mut spec_changed = Vec::new();
    let mut spec_unchanged = Vec::new();

    // Partition current files into specs and non-specs.
    let (current_specs, current_artifacts): (Vec<_>, Vec<_>) = current_files
        .iter()
        .partition(|(p, _)| p.ends_with(".spec.yaml"));

    // Check non-spec artifacts.
    let current_map: BTreeMap<&str, &str> = current_artifacts
        .iter()
        .map(|(p, h)| (p.as_str(), h.as_str()))
        .collect();

    for (path, fp) in &state.file_fingerprints {
        match current_map.get(path.as_str()) {
            Some(h) if *h == fp.blake3 => {} // unchanged -- not reported
            Some(_) => modified.push(path.clone()),
            None => removed.push(path.clone()),
        }
    }
    for (path, hash) in &current_artifacts {
        if !state.file_fingerprints.contains_key(path) {
            let _ = hash;
            added.push(path.clone());
        }
    }

    // Check spec fingerprints.
    let current_spec_map: BTreeMap<&str, &str> = current_specs
        .iter()
        .map(|(p, h)| (p.as_str(), h.as_str()))
        .collect();

    for (path, stored_hash) in &state.spec_fingerprints {
        match current_spec_map.get(path.as_str()) {
            Some(h) if *h == stored_hash => spec_unchanged.push(path.clone()),
            Some(_) => spec_changed.push(path.clone()),
            None => spec_changed.push(path.clone()), // removed spec counts as changed
        }
    }
    for (path, _) in &current_specs {
        if !state.spec_fingerprints.contains_key(path) {
            spec_changed.push(path.clone()); // new spec
        }
    }

    StateDelta {
        added,
        modified,
        removed,
        spec_changed,
        spec_unchanged,
    }
}

fn utc_timestamp() -> String {
    let duration = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}Z", duration.as_secs())
}

fn git_head(project_root: &Path) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(project_root)
        .output()
        .ok()?;
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

fn git_branch(project_root: &Path) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["branch", "--show-current"])
        .current_dir(project_root)
        .output()
        .ok()?;
    if output.status.success() {
        let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if branch.is_empty() {
            None
        } else {
            Some(branch)
        }
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_state() -> ReconciliationState {
        let mut file_fingerprints = BTreeMap::new();
        file_fingerprints.insert(
            "src/main.rs".to_string(),
            FileFingerprint {
                blake3: "abc123".to_string(),
            },
        );
        let mut spec_fingerprints = BTreeMap::new();
        spec_fingerprints.insert(".notarai/cli.spec.yaml".to_string(), "def456".to_string());
        ReconciliationState {
            schema_version: "1".to_string(),
            last_reconciliation: ReconciliationMeta {
                timestamp: "1700000000Z".to_string(),
                git_hash: Some("deadbeef".to_string()),
                branch: Some("main".to_string()),
            },
            file_fingerprints,
            spec_fingerprints,
        }
    }

    #[test]
    fn test_state_path() {
        let tmp = TempDir::new().unwrap();
        let path = state_path(tmp.path());
        assert!(path.ends_with(".notarai/reconciliation_state.json"));
    }

    #[test]
    fn test_load_missing_state() {
        let tmp = TempDir::new().unwrap();
        let result = load_state(tmp.path()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_state_roundtrip() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join(".notarai")).unwrap();
        let state = make_state();
        save_state(tmp.path(), &state).unwrap();
        let loaded = load_state(tmp.path()).unwrap().unwrap();
        assert_eq!(state, loaded);
    }

    #[test]
    fn test_snapshot_deterministic_output() {
        // BTreeMap keys are always sorted
        let state = make_state();
        let json = serde_json::to_string_pretty(&state).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        let keys: Vec<&str> = parsed["file_fingerprints"]
            .as_object()
            .unwrap()
            .keys()
            .map(|k| k.as_str())
            .collect();
        let mut sorted = keys.clone();
        sorted.sort();
        assert_eq!(keys, sorted, "BTreeMap keys must be in sorted order");
    }

    #[test]
    fn test_diff_against_state_all_cases() {
        let state = make_state();

        // current: src/main.rs modified, new file added, spec changed
        let current: Vec<(String, String)> = vec![
            ("src/main.rs".to_string(), "different_hash".to_string()),
            ("src/lib.rs".to_string(), "new_hash".to_string()),
            (
                ".notarai/cli.spec.yaml".to_string(),
                "changed_spec_hash".to_string(),
            ),
        ];
        // stored has src/main.rs + .notarai/cli.spec.yaml; no src/lib.rs

        let delta = diff_against_state(&state, &current);
        assert!(delta.modified.contains(&"src/main.rs".to_string()));
        assert!(delta.added.contains(&"src/lib.rs".to_string()));
        assert!(delta.removed.is_empty()); // nothing removed from file_fingerprints
        assert!(
            delta
                .spec_changed
                .contains(&".notarai/cli.spec.yaml".to_string())
        );
        assert!(delta.spec_unchanged.is_empty());

        // unchanged case
        let current_unchanged: Vec<(String, String)> = vec![
            ("src/main.rs".to_string(), "abc123".to_string()),
            (".notarai/cli.spec.yaml".to_string(), "def456".to_string()),
        ];
        let delta2 = diff_against_state(&state, &current_unchanged);
        assert!(delta2.modified.is_empty());
        assert!(delta2.added.is_empty());
        assert!(delta2.removed.is_empty());
        assert!(delta2.spec_changed.is_empty());
        assert!(
            delta2
                .spec_unchanged
                .contains(&".notarai/cli.spec.yaml".to_string())
        );

        // removed case
        let current_removed: Vec<(String, String)> =
            vec![(".notarai/cli.spec.yaml".to_string(), "def456".to_string())];
        let delta3 = diff_against_state(&state, &current_removed);
        assert!(delta3.removed.contains(&"src/main.rs".to_string()));
    }
}
