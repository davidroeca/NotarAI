use std::path::Path;

/// List files changed between `base_branch` and HEAD.
///
/// Returns relative file paths from `git diff --name-only`.
pub fn changed_files(base_branch: &str, project_root: &Path) -> Result<Vec<String>, String> {
    let output = std::process::Command::new("git")
        .args(["diff", base_branch, "--name-only"])
        .current_dir(project_root)
        .output()
        .map_err(|e| format!("git error: {e}"))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect())
}

/// Run `git diff <base_branch>` on specific files with optional exclude patterns.
///
/// Exclude patterns are passed as `:(exclude)<pattern>` pathspecs so git
/// resolves them as globs.
pub fn diff_files(
    base_branch: &str,
    files: &[String],
    exclude_patterns: &[String],
    project_root: &Path,
) -> Result<String, String> {
    if files.is_empty() {
        return Ok(String::new());
    }

    let exclude_args: Vec<String> = exclude_patterns
        .iter()
        .map(|p| format!(":(exclude){p}"))
        .collect();

    let mut args: Vec<&str> = vec!["diff", base_branch, "--"];
    args.extend(files.iter().map(String::as_str));
    args.extend(exclude_args.iter().map(String::as_str));

    let output = std::process::Command::new("git")
        .args(&args)
        .current_dir(project_root)
        .output()
        .map_err(|e| format!("git error: {e}"))?;

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// List all tracked files via `git ls-files`.
///
/// Returns relative paths. Useful for coverage gap detection since it
/// naturally excludes untracked and gitignored files.
pub fn tracked_files(project_root: &Path) -> Result<Vec<String>, String> {
    let output = std::process::Command::new("git")
        .args(["ls-files"])
        .current_dir(project_root)
        .output()
        .map_err(|e| format!("git error: {e}"))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect())
}

/// Return the git HEAD hash, or `None` if not in a git repo.
pub fn head_hash(project_root: &Path) -> Option<String> {
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

/// Return the current branch name, or `None` if detached or not in a git repo.
pub fn current_branch(project_root: &Path) -> Option<String> {
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
