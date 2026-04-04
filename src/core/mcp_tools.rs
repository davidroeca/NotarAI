use std::path::Path;

use crate::core::spec_loader;

/// A JSON-RPC error returned by an MCP tool.
pub struct McpError {
    pub code: i32,
    pub message: String,
}

/// Shorthand result type for MCP tool functions.
pub type McpResult = Result<serde_json::Value, McpError>;

fn mcp_err(message: String) -> McpError {
    McpError {
        code: -32603,
        message,
    }
}

/// List specs whose governed files overlap with files changed since `base_branch`.
///
/// Runs `git diff <base_branch> --name-only`, then cross-references each
/// `.notarai/*.spec.yaml` artifact glob against the changed file list. Returns
/// a JSON object with `changed_files` (all changed paths) and `affected_specs`
/// (specs with at least one matching artifact, including their `behaviors`,
/// `constraints`, and `invariants`).
pub fn list_affected_specs(base_branch: &str, project_root: &Path) -> McpResult {
    let changed = crate::core::git::changed_files(base_branch, project_root).map_err(mcp_err)?;

    let specs = spec_loader::collect_specs(project_root).map_err(mcp_err)?;

    let mut affected = Vec::new();
    for spec_path in &specs {
        let spec_rel = spec_path
            .strip_prefix(project_root)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| spec_path.to_string_lossy().to_string());

        let spec_value = spec_loader::load_spec(spec_path).map_err(mcp_err)?;

        if spec_loader::is_spec_affected(&spec_value, &changed) {
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
    let spec_value = spec_loader::load_spec(&abs_spec).map_err(mcp_err)?;

    let files = spec_loader::expand_artifact_globs(&spec_value, project_root);

    if files.is_empty() {
        return Ok(serde_json::json!({
            "diff": "",
            "files": [],
            "skipped": [],
            "excluded": exclude_patterns,
            "spec_changes": [],
            "system_spec": null,
            "spec_invalidated": [],
        }));
    }

    // Partition governed files: .notarai/**/*.spec.yaml vs. everything else.
    let (spec_files, artifact_files): (Vec<String>, Vec<String>) = files
        .into_iter()
        .partition(|f| spec_loader::is_spec_file(f));

    // Apply cache filtering to both groups independently.
    let (spec_to_show, artifact_to_diff, artifact_skipped, primary_spec_changed) = if bypass_cache {
        (spec_files, artifact_files, vec![], false)
    } else {
        match crate::core::cache::open_cache_db(project_root) {
            Ok(conn) => {
                // Check whether the primary spec file itself has changed vs cache.
                let psc = is_spec_changed_vs_cache(&conn, spec_path, &abs_spec);

                let spec_pairs: Vec<(String, std::path::PathBuf)> = spec_files
                    .into_iter()
                    .map(|rel| {
                        let abs = project_root.join(&rel);
                        (rel, abs)
                    })
                    .collect();
                let (s_show, _s_skip) = crate::core::cache::check_changed_batch(&conn, &spec_pairs)
                    .unwrap_or_else(|_| {
                        let all: Vec<String> = spec_pairs.into_iter().map(|(r, _)| r).collect();
                        (all, vec![])
                    });

                let artifact_pairs: Vec<(String, std::path::PathBuf)> = artifact_files
                    .into_iter()
                    .map(|rel| {
                        let abs = project_root.join(&rel);
                        (rel, abs)
                    })
                    .collect();
                let (a_diff, a_skip) = crate::core::cache::check_changed_batch(
                    &conn,
                    &artifact_pairs,
                )
                .unwrap_or_else(|_| {
                    let all: Vec<String> = artifact_pairs.into_iter().map(|(r, _)| r).collect();
                    (all, vec![])
                });

                (s_show, a_diff, a_skip, psc)
            }
            Err(_) => (spec_files, artifact_files, vec![], false), // cache unavailable: include everything
        }
    };

    // When the primary spec changed, cached artifacts that would normally be
    // skipped are reclassified as spec_invalidated: they need review because
    // their governing spec has drifted even though the artifacts themselves
    // have not changed on disk.
    let (artifact_skipped, spec_invalidated) = if primary_spec_changed || !spec_to_show.is_empty() {
        (vec![], artifact_skipped)
    } else {
        (artifact_skipped, vec![])
    };

    // Read full content of each changed spec file.
    let mut spec_changes = Vec::new();
    for spec_rel in &spec_to_show {
        let abs = project_root.join(spec_rel);
        let spec_content = std::fs::read_to_string(&abs)
            .map_err(|e| mcp_err(format!("read error for {spec_rel}: {e}")))?;
        spec_changes.push(serde_json::json!({
            "path": spec_rel,
            "content": spec_content,
        }));
    }

    // Locate and include the system spec when any spec files changed.
    let system_spec = if spec_changes.is_empty() {
        serde_json::Value::Null
    } else {
        spec_loader::find_system_spec(project_root, &spec_to_show).map_err(mcp_err)?
    };

    // Partition artifact files into binary (known extension) and non-binary.
    let (binary_by_ext, non_binary): (Vec<String>, Vec<String>) = artifact_to_diff
        .iter()
        .cloned()
        .partition(|f| is_binary_by_extension(f));

    // Run git diff on non-binary artifact files.
    let diff =
        crate::core::git::diff_files(base_branch, &non_binary, exclude_patterns, project_root)
            .map_err(mcp_err)?;

    // Collect additional binary files detected from "Binary files ... differ" in the diff,
    // and strip those lines so the returned diff stays clean.
    let mut binary_changes: Vec<String> = binary_by_ext;
    let mut clean_lines: Vec<&str> = Vec::new();
    for line in diff.lines() {
        if line.starts_with("Binary files") && line.contains("differ") {
            if let Some(rest) = line.strip_prefix("Binary files a/")
                && let Some(path) = rest.split(" and b/").next()
                && !binary_changes.iter().any(|b| b == path)
            {
                binary_changes.push(path.to_string());
            }
            // Drop this line from the clean diff output.
        } else {
            clean_lines.push(line);
        }
    }
    let diff = clean_lines.join("\n");

    // Build file_categories: map each changed artifact file to its spec category.
    let file_categories =
        spec_loader::build_file_categories(&spec_value, &artifact_to_diff, project_root);

    Ok(serde_json::json!({
        "diff": diff,
        "files": artifact_to_diff,
        "skipped": artifact_skipped,
        "excluded": exclude_patterns,
        "spec_changes": spec_changes,
        "system_spec": system_spec,
        "binary_changes": binary_changes,
        "file_categories": file_categories,
        "spec_invalidated": spec_invalidated,
    }))
}

/// Delete the cache database file, if it exists.
///
/// Returns `{"cleared": true}` when the file was deleted, `{"cleared": false}`
/// when it did not exist.
pub fn clear_cache(project_root: &Path) -> McpResult {
    let path = crate::core::cache::db_path(project_root);
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| mcp_err(format!("could not delete cache: {e}")))?;
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
    let spec_value = spec_loader::load_spec(&abs_spec).map_err(mcp_err)?;

    let files = match artifact_type {
        Some(art_type) => {
            spec_loader::expand_artifact_type_globs(&spec_value, project_root, art_type)
        }
        None => spec_loader::expand_artifact_globs(&spec_value, project_root),
    };

    let conn = crate::core::cache::open_cache_db(project_root).map_err(mcp_err)?;

    // Check whether the primary spec file itself has changed vs cache.
    let primary_spec_changed = is_spec_changed_vs_cache(&conn, spec_path, &abs_spec);

    // Partition governed files: .notarai/**/*.spec.yaml vs. everything else.
    let (spec_files, artifact_files): (Vec<String>, Vec<String>) = files
        .into_iter()
        .partition(|f| spec_loader::is_spec_file(f));

    // Check governed spec files against cache.
    let spec_pairs: Vec<(String, std::path::PathBuf)> = spec_files
        .into_iter()
        .map(|rel| {
            let abs = project_root.join(&rel);
            (rel, abs)
        })
        .collect();
    let (governed_specs_changed, _) = crate::core::cache::check_changed_batch(&conn, &spec_pairs)
        .unwrap_or_else(|_| {
            let all: Vec<String> = spec_pairs.into_iter().map(|(r, _)| r).collect();
            (all, vec![])
        });

    let artifact_pairs: Vec<(String, std::path::PathBuf)> = artifact_files
        .into_iter()
        .map(|rel| {
            let abs = project_root.join(&rel);
            (rel, abs)
        })
        .collect();

    let (changed, unchanged) =
        crate::core::cache::check_changed_batch(&conn, &artifact_pairs).map_err(mcp_err)?;

    let spec_invalidated = if primary_spec_changed || !governed_specs_changed.is_empty() {
        unchanged
    } else {
        vec![]
    };

    Ok(serde_json::json!({
        "changed_artifacts": changed,
        "spec_invalidated": spec_invalidated,
    }))
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
    let conn = crate::core::cache::open_cache_db(project_root).map_err(mcp_err)?;

    let mut to_upsert = Vec::new();
    for f in files {
        let abs = project_root.join(f);
        if abs.exists() {
            match crate::core::cache::hash_file(&abs) {
                Ok(hash) => to_upsert.push((f.clone(), hash)),
                Err(e) => eprintln!("Warning: {e}"),
            }
        }
    }

    let count = crate::core::cache::upsert_batch(&conn, &to_upsert).map_err(mcp_err)?;

    Ok(serde_json::json!({"updated": count}))
}

/// Snapshot the current cache + git state into reconciliation_state.json.
///
/// Called at the end of a reconciliation pass to persist the baseline.
/// Returns `{"state_path": "...", "files": N, "specs": N, "git_hash": "..."}`.
pub fn snapshot_state(project_root: &Path) -> McpResult {
    let state = crate::core::state::snapshot_from_cache(project_root).map_err(mcp_err)?;
    crate::core::state::save_state(project_root, &state).map_err(mcp_err)?;
    let state_path = crate::core::state::state_path(project_root)
        .to_string_lossy()
        .to_string();
    let git_hash = state
        .last_reconciliation
        .git_hash
        .as_deref()
        .unwrap_or("")
        .to_string();
    Ok(serde_json::json!({
        "state_path": state_path,
        "files": state.file_fingerprints.len(),
        "specs": state.spec_fingerprints.len(),
        "git_hash": git_hash,
    }))
}

/// Known binary file extensions whose unified diffs are uninformative noise.
const BINARY_EXTENSIONS: &[&str] = &[
    ".png", ".jpg", ".jpeg", ".gif", ".webp", ".ico", ".pptx", ".docx", ".xlsx", ".pdf", ".zip",
    ".tar", ".gz", ".wasm", ".exe", ".dll", ".so", ".dylib",
];

fn is_binary_by_extension(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    BINARY_EXTENSIONS.iter().any(|ext| lower.ends_with(ext))
}

/// Check whether a spec file's on-disk content differs from the cache.
///
/// Returns `true` when the spec has changed (or the cache lookup fails),
/// `false` when the cached hash matches the current file.
fn is_spec_changed_vs_cache(conn: &rusqlite::Connection, spec_rel: &str, spec_abs: &Path) -> bool {
    let pair = vec![(spec_rel.to_string(), spec_abs.to_path_buf())];
    let (changed, _) = crate::core::cache::check_changed_batch(conn, &pair)
        .unwrap_or_else(|_| (vec![spec_rel.to_string()], vec![]));
    !changed.is_empty()
}
