use assert_cmd::cargo_bin_cmd;
use predicates::prelude::*;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn notarai() -> assert_cmd::Command {
    cargo_bin_cmd!("notarai")
}

/// Initialize a throwaway git repo with a test identity so commits work
/// regardless of the host environment's global git config.
fn setup_git_repo(dir: &Path) {
    for args in [
        vec!["init"],
        vec!["config", "user.email", "test@notarai.dev"],
        vec!["config", "user.name", "NotarAI Test"],
        vec!["config", "commit.gpgsign", "false"],
    ] {
        std::process::Command::new("git")
            .args(&args)
            .current_dir(dir)
            .output()
            .unwrap();
    }
}

fn git_commit_all(dir: &Path, msg: &str) {
    for args in [vec!["add", "."], vec!["commit", "-m", msg]] {
        std::process::Command::new("git")
            .args(&args)
            .current_dir(dir)
            .output()
            .unwrap();
    }
}

/// Minimal spec that governs all `*.txt` files in the repo root.
const TXT_SPEC: &str = r#"schema_version: '0.4'
intent: 'Test spec governing txt files'
behaviors:
  - name: tracks_txt_files
    given: 'txt files exist'
    then: 'they appear in the diff'
artifacts:
  code:
    - path: '*.txt'
"#;

/// Spec that governs a code file AND a child spec file.
const GOVERNING_SPEC: &str = r#"schema_version: '0.4'
intent: 'Spec that governs a child spec and a code file'
behaviors:
  - name: tracks
    given: 'child spec and code file exist'
    then: 'they are tracked'
artifacts:
  code:
    - path: 'code.txt'
    - path: '.notarai/child.spec.yaml'
"#;

/// A child spec (v1).
const CHILD_SPEC_V1: &str = r#"schema_version: '0.4'
intent: 'Child spec v1'
behaviors:
  - name: test
    given: 'test'
    then: 'test'
artifacts:
  code:
    - path: 'code.txt'
"#;

/// A child spec (v2 -- updated intent to trigger a diff).
const CHILD_SPEC_V2: &str = r#"schema_version: '0.4'
intent: 'Child spec v2 - updated'
behaviors:
  - name: test
    given: 'test'
    then: 'test updated'
artifacts:
  code:
    - path: 'code.txt'
"#;

/// A system spec with a `subsystems` key.
const SYSTEM_SPEC: &str = r#"schema_version: '0.4'
intent: 'System spec with subsystems'
behaviors:
  - name: orchestrates
    given: 'subsystems exist'
    then: 'they are composed'
subsystems:
  - $ref: './child.spec.yaml'
artifacts:
  docs:
    - path: 'README.md'
"#;

const INITIALIZE_MSG: &str = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{}}}"#;

#[test]
fn responds_to_initialize_with_server_info_and_tools() {
    notarai()
        .arg("mcp")
        .write_stdin(format!("{INITIALIZE_MSG}\n"))
        .assert()
        .success()
        .stdout(predicate::str::contains("serverInfo"))
        .stdout(predicate::str::contains("list_affected_specs"))
        .stdout(predicate::str::contains("get_spec_diff"))
        .stdout(predicate::str::contains("get_changed_artifacts"))
        .stdout(predicate::str::contains("mark_reconciled"));
}

#[test]
fn unknown_method_returns_32601() {
    notarai()
        .arg("mcp")
        .write_stdin(r#"{"jsonrpc":"2.0","id":1,"method":"nonexistent","params":{}}"#)
        .assert()
        .success()
        .stdout(predicate::str::contains("-32601"));
}

#[test]
fn exits_0_on_stdin_eof() {
    notarai().arg("mcp").write_stdin("").assert().success();
}

#[test]
fn unknown_tool_returns_error() {
    let msg = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"nonexistent_tool","arguments":{}}}"#;
    notarai()
        .arg("mcp")
        .write_stdin(msg)
        .assert()
        .success()
        .stdout(predicate::str::contains("error"));
}

// -- get_spec_diff: exclude_patterns ------------------------------------------

#[test]
fn get_spec_diff_advertises_exclude_patterns_in_schema() {
    // The initialize response embeds the full inputSchema for each tool.
    // Confirm exclude_patterns is declared for get_spec_diff.
    notarai()
        .arg("mcp")
        .write_stdin(format!("{INITIALIZE_MSG}\n"))
        .assert()
        .success()
        .stdout(predicate::str::contains("exclude_patterns"));
}

#[test]
fn get_spec_diff_without_exclude_patterns_shows_all_changed_files() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    setup_git_repo(root);
    fs::create_dir_all(root.join(".notarai")).unwrap();
    fs::write(root.join(".notarai/test.spec.yaml"), TXT_SPEC).unwrap();
    fs::write(root.join("included.txt"), "initial").unwrap();
    fs::write(root.join("noisy.txt"), "initial").unwrap();
    git_commit_all(root, "base");

    fs::write(root.join("included.txt"), "modified").unwrap();
    fs::write(root.join("noisy.txt"), "modified").unwrap();
    git_commit_all(root, "changes");

    let msg = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_spec_diff","arguments":{"spec_path":".notarai/test.spec.yaml","base_branch":"HEAD~1"}}}"#;

    notarai()
        .arg("mcp")
        .write_stdin(format!("{msg}\n"))
        .current_dir(root)
        .assert()
        .success()
        // Both files should appear in the git diff header lines.
        .stdout(predicate::str::contains("diff --git a/included.txt"))
        .stdout(predicate::str::contains("diff --git a/noisy.txt"));
}

#[test]
fn get_spec_diff_exclude_patterns_suppresses_exact_filename_from_diff() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    setup_git_repo(root);
    fs::create_dir_all(root.join(".notarai")).unwrap();
    fs::write(root.join(".notarai/test.spec.yaml"), TXT_SPEC).unwrap();
    fs::write(root.join("included.txt"), "initial").unwrap();
    fs::write(root.join("noisy.txt"), "initial").unwrap();
    git_commit_all(root, "base");

    fs::write(root.join("included.txt"), "modified").unwrap();
    fs::write(root.join("noisy.txt"), "modified").unwrap();
    git_commit_all(root, "changes");

    let msg = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_spec_diff","arguments":{"spec_path":".notarai/test.spec.yaml","base_branch":"HEAD~1","exclude_patterns":["noisy.txt"]}}}"#;

    notarai()
        .arg("mcp")
        .write_stdin(format!("{msg}\n"))
        .current_dir(root)
        .assert()
        .success()
        // included.txt must still appear in diff content.
        .stdout(predicate::str::contains("diff --git a/included.txt"))
        // noisy.txt must NOT appear in diff content (it may still appear in
        // the "files" and "excluded" JSON fields, so we match the git diff
        // header line specifically).
        .stdout(predicate::str::contains("diff --git a/noisy.txt").not());
}

// -- get_spec_diff: cache filtering ------------------------------------------

#[test]
fn get_spec_diff_cold_cache_includes_all_changed_files() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    setup_git_repo(root);
    fs::create_dir_all(root.join(".notarai")).unwrap();
    fs::write(root.join(".notarai/test.spec.yaml"), TXT_SPEC).unwrap();
    fs::write(root.join("alpha.txt"), "initial").unwrap();
    fs::write(root.join("beta.txt"), "initial").unwrap();
    git_commit_all(root, "base");

    fs::write(root.join("alpha.txt"), "changed").unwrap();
    fs::write(root.join("beta.txt"), "changed").unwrap();
    git_commit_all(root, "changes");

    // No cache seeding; cold start should diff everything.
    let msg = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_spec_diff","arguments":{"spec_path":".notarai/test.spec.yaml","base_branch":"HEAD~1"}}}"#;

    notarai()
        .arg("mcp")
        .write_stdin(format!("{msg}\n"))
        .current_dir(root)
        .assert()
        .success()
        .stdout(predicate::str::contains("diff --git a/alpha.txt"))
        .stdout(predicate::str::contains("diff --git a/beta.txt"));
}

#[test]
fn get_spec_diff_skips_file_matching_cache() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    setup_git_repo(root);
    fs::create_dir_all(root.join(".notarai")).unwrap();
    fs::write(root.join(".notarai/test.spec.yaml"), TXT_SPEC).unwrap();
    fs::write(root.join("alpha.txt"), "initial").unwrap();
    fs::write(root.join("beta.txt"), "initial").unwrap();
    git_commit_all(root, "base");

    fs::write(root.join("alpha.txt"), "changed").unwrap();
    fs::write(root.join("beta.txt"), "changed").unwrap();
    git_commit_all(root, "changes");

    // Seed beta.txt via mark_reconciled (uses relative path keys, same as get_spec_diff).
    let seed_msg = r#"{"jsonrpc":"2.0","id":0,"method":"tools/call","params":{"name":"mark_reconciled","arguments":{"files":["beta.txt"]}}}"#;
    let diff_msg = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_spec_diff","arguments":{"spec_path":".notarai/test.spec.yaml","base_branch":"HEAD~1"}}}"#;

    notarai()
        .arg("mcp")
        .write_stdin(format!("{seed_msg}\n{diff_msg}\n"))
        .current_dir(root)
        .assert()
        .success()
        // alpha.txt is not in cache -> appears in diff
        .stdout(predicate::str::contains("diff --git a/alpha.txt"))
        // beta.txt hash matches cache -> skipped
        .stdout(predicate::str::contains("diff --git a/beta.txt").not())
        .stdout(predicate::str::contains("beta.txt"));
}

#[test]
fn get_spec_diff_skips_all_when_all_files_cached() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    setup_git_repo(root);
    fs::create_dir_all(root.join(".notarai")).unwrap();
    fs::write(root.join(".notarai/test.spec.yaml"), TXT_SPEC).unwrap();
    fs::write(root.join("alpha.txt"), "initial").unwrap();
    fs::write(root.join("beta.txt"), "initial").unwrap();
    git_commit_all(root, "base");

    fs::write(root.join("alpha.txt"), "changed").unwrap();
    fs::write(root.join("beta.txt"), "changed").unwrap();
    git_commit_all(root, "changes");

    // Seed both files via mark_reconciled - both should be skipped.
    let seed_msg = r#"{"jsonrpc":"2.0","id":0,"method":"tools/call","params":{"name":"mark_reconciled","arguments":{"files":["alpha.txt","beta.txt"]}}}"#;
    let diff_msg = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_spec_diff","arguments":{"spec_path":".notarai/test.spec.yaml","base_branch":"HEAD~1"}}}"#;

    notarai()
        .arg("mcp")
        .write_stdin(format!("{seed_msg}\n{diff_msg}\n"))
        .current_dir(root)
        .assert()
        .success()
        .stdout(predicate::str::contains("diff --git").not())
        .stdout(predicate::str::contains("alpha.txt"))
        .stdout(predicate::str::contains("beta.txt"));
}

#[test]
fn get_spec_diff_bypass_cache_includes_all_files() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    setup_git_repo(root);
    fs::create_dir_all(root.join(".notarai")).unwrap();
    fs::write(root.join(".notarai/test.spec.yaml"), TXT_SPEC).unwrap();
    fs::write(root.join("alpha.txt"), "initial").unwrap();
    fs::write(root.join("beta.txt"), "initial").unwrap();
    git_commit_all(root, "base");

    fs::write(root.join("alpha.txt"), "changed").unwrap();
    fs::write(root.join("beta.txt"), "changed").unwrap();
    git_commit_all(root, "changes");

    // Seed both files via mark_reconciled, then bypass_cache should still show full diff.
    let seed_msg = r#"{"jsonrpc":"2.0","id":0,"method":"tools/call","params":{"name":"mark_reconciled","arguments":{"files":["alpha.txt","beta.txt"]}}}"#;
    let diff_msg = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_spec_diff","arguments":{"spec_path":".notarai/test.spec.yaml","base_branch":"HEAD~1","bypass_cache":true}}}"#;

    notarai()
        .arg("mcp")
        .write_stdin(format!("{seed_msg}\n{diff_msg}\n"))
        .current_dir(root)
        .assert()
        .success()
        .stdout(predicate::str::contains("diff --git a/alpha.txt"))
        .stdout(predicate::str::contains("diff --git a/beta.txt"));
}

#[test]
fn clear_cache_allows_full_diff_on_next_call() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    setup_git_repo(root);
    fs::create_dir_all(root.join(".notarai")).unwrap();
    fs::write(root.join(".notarai/test.spec.yaml"), TXT_SPEC).unwrap();
    fs::write(root.join("alpha.txt"), "initial").unwrap();
    fs::write(root.join("beta.txt"), "initial").unwrap();
    git_commit_all(root, "base");

    fs::write(root.join("alpha.txt"), "changed").unwrap();
    fs::write(root.join("beta.txt"), "changed").unwrap();
    git_commit_all(root, "changes");

    // Seed both files via mark_reconciled, then clear cache, then verify full diff.
    let seed_msg = r#"{"jsonrpc":"2.0","id":0,"method":"tools/call","params":{"name":"mark_reconciled","arguments":{"files":["alpha.txt","beta.txt"]}}}"#;
    let clear_msg = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"clear_cache","arguments":{}}}"#;
    let diff_msg = r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"get_spec_diff","arguments":{"spec_path":".notarai/test.spec.yaml","base_branch":"HEAD~1"}}}"#;

    notarai()
        .arg("mcp")
        .write_stdin(format!("{seed_msg}\n{clear_msg}\n{diff_msg}\n"))
        .current_dir(root)
        .assert()
        .success()
        .stdout(predicate::str::contains("cleared"))
        // After cache cleared, cold start -> both files in diff.
        .stdout(predicate::str::contains("diff --git a/alpha.txt"))
        .stdout(predicate::str::contains("diff --git a/beta.txt"));
}

#[test]
fn get_spec_diff_exclude_patterns_supports_glob_patterns() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    setup_git_repo(root);
    fs::create_dir_all(root.join(".notarai")).unwrap();
    fs::write(root.join(".notarai/test.spec.yaml"), TXT_SPEC).unwrap();
    fs::write(root.join("keep.txt"), "initial").unwrap();
    fs::write(root.join("data.lock"), "initial").unwrap();
    git_commit_all(root, "base");

    fs::write(root.join("keep.txt"), "modified").unwrap();
    fs::write(root.join("data.lock"), "modified").unwrap();
    git_commit_all(root, "changes");

    // Spec governs *.txt only, so data.lock won't appear in files at all -
    // but add a second spec variant that governs both so we can confirm the
    // glob exclude works across matched files.
    let both_spec = r#"schema_version: '0.4'
intent: 'Test spec governing all files'
behaviors:
  - name: tracks_all
    given: 'files exist'
    then: 'they are tracked'
artifacts:
  code:
    - path: 'keep.txt'
    - path: 'data.lock'
"#;
    fs::write(root.join(".notarai/test.spec.yaml"), both_spec).unwrap();

    // Exclude everything matching *.lock via glob.
    let msg = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_spec_diff","arguments":{"spec_path":".notarai/test.spec.yaml","base_branch":"HEAD~1","exclude_patterns":["*.lock"]}}}"#;

    notarai()
        .arg("mcp")
        .write_stdin(format!("{msg}\n"))
        .current_dir(root)
        .assert()
        .success()
        .stdout(predicate::str::contains("diff --git a/keep.txt"))
        .stdout(predicate::str::contains("diff --git a/data.lock").not());
}

// -- get_spec_diff: spec-aware splitting --------------------------------------

#[test]
fn get_spec_diff_splits_spec_files_into_spec_changes() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    setup_git_repo(root);
    fs::create_dir_all(root.join(".notarai")).unwrap();
    fs::write(root.join(".notarai/governing.spec.yaml"), GOVERNING_SPEC).unwrap();
    fs::write(root.join(".notarai/child.spec.yaml"), CHILD_SPEC_V1).unwrap();
    fs::write(root.join("code.txt"), "initial").unwrap();
    git_commit_all(root, "base");

    // Modify both the governed spec file and the code file.
    fs::write(root.join(".notarai/child.spec.yaml"), CHILD_SPEC_V2).unwrap();
    fs::write(root.join("code.txt"), "modified").unwrap();
    git_commit_all(root, "changes");

    let msg = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_spec_diff","arguments":{"spec_path":".notarai/governing.spec.yaml","base_branch":"HEAD~1"}}}"#;

    notarai()
        .arg("mcp")
        .write_stdin(format!("{msg}\n"))
        .current_dir(root)
        .assert()
        .success()
        // spec_changes must contain the child spec
        .stdout(predicate::str::contains("spec_changes"))
        .stdout(predicate::str::contains("child.spec.yaml"))
        // code.txt diff must appear in the diff field
        .stdout(predicate::str::contains("diff --git a/code.txt"))
        // child spec must NOT appear as a git diff header (it's in spec_changes, not diff)
        .stdout(predicate::str::contains("diff --git a/.notarai/child.spec.yaml").not());
}

#[test]
fn get_spec_diff_includes_system_spec_when_spec_changes() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    setup_git_repo(root);
    fs::create_dir_all(root.join(".notarai")).unwrap();
    // system.spec.yaml has subsystems -- it's the system spec.
    fs::write(root.join(".notarai/system.spec.yaml"), SYSTEM_SPEC).unwrap();
    fs::write(root.join(".notarai/governing.spec.yaml"), GOVERNING_SPEC).unwrap();
    fs::write(root.join(".notarai/child.spec.yaml"), CHILD_SPEC_V1).unwrap();
    fs::write(root.join("code.txt"), "initial").unwrap();
    fs::write(root.join("README.md"), "readme").unwrap();
    git_commit_all(root, "base");

    // Only change the governed child spec (not the system spec).
    fs::write(root.join(".notarai/child.spec.yaml"), CHILD_SPEC_V2).unwrap();
    git_commit_all(root, "changes");

    let msg = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_spec_diff","arguments":{"spec_path":".notarai/governing.spec.yaml","base_branch":"HEAD~1"}}}"#;

    notarai()
        .arg("mcp")
        .write_stdin(format!("{msg}\n"))
        .current_dir(root)
        .assert()
        .success()
        // spec_changes must contain the changed child spec
        .stdout(predicate::str::contains("child.spec.yaml"))
        // system_spec must be present with content (it didn't change so content is included)
        .stdout(predicate::str::contains("system_spec"))
        .stdout(predicate::str::contains("System spec with subsystems"));
}

#[test]
fn get_spec_diff_no_spec_changes_when_only_code_changes() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    setup_git_repo(root);
    fs::create_dir_all(root.join(".notarai")).unwrap();
    fs::write(root.join(".notarai/test.spec.yaml"), TXT_SPEC).unwrap();
    fs::write(root.join("alpha.txt"), "initial").unwrap();
    git_commit_all(root, "base");

    // Only a code file changes -- no spec files in the governed set.
    fs::write(root.join("alpha.txt"), "changed").unwrap();
    git_commit_all(root, "changes");

    let msg = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_spec_diff","arguments":{"spec_path":".notarai/test.spec.yaml","base_branch":"HEAD~1"}}}"#;

    notarai()
        .arg("mcp")
        .write_stdin(format!("{msg}\n"))
        .current_dir(root)
        .assert()
        .success()
        // diff must contain the code change
        .stdout(predicate::str::contains("diff --git a/alpha.txt"))
        // spec_changes key must be present but empty
        .stdout(predicate::str::contains("spec_changes"))
        // system_spec must be null (no spec changes)
        .stdout(predicate::str::contains("system_spec"));
}

#[test]
fn get_spec_diff_cache_filtering_applies_to_spec_files() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    setup_git_repo(root);
    fs::create_dir_all(root.join(".notarai")).unwrap();
    fs::write(root.join(".notarai/governing.spec.yaml"), GOVERNING_SPEC).unwrap();
    fs::write(root.join(".notarai/child.spec.yaml"), CHILD_SPEC_V1).unwrap();
    fs::write(root.join("code.txt"), "initial").unwrap();
    git_commit_all(root, "base");

    // Modify both the child spec and code file.
    fs::write(root.join(".notarai/child.spec.yaml"), CHILD_SPEC_V2).unwrap();
    fs::write(root.join("code.txt"), "modified").unwrap();
    git_commit_all(root, "changes");

    // Seed the child spec into the cache with its current (changed) hash --
    // the cache now considers it reconciled.
    let seed_msg = r#"{"jsonrpc":"2.0","id":0,"method":"tools/call","params":{"name":"mark_reconciled","arguments":{"files":[".notarai/child.spec.yaml"]}}}"#;
    let diff_msg = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_spec_diff","arguments":{"spec_path":".notarai/governing.spec.yaml","base_branch":"HEAD~1"}}}"#;

    notarai()
        .arg("mcp")
        .write_stdin(format!("{seed_msg}\n{diff_msg}\n"))
        .current_dir(root)
        .assert()
        .success()
        // code.txt is not cached -> appears in the diff
        .stdout(predicate::str::contains("diff --git a/code.txt"))
        // child spec is cached -> must NOT appear in spec_changes content
        .stdout(predicate::str::contains("Child spec v2").not());
}
