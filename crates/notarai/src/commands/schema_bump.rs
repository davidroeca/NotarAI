use crate::core::schema;
use crate::core::validator;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// The schema version string embedded in the bundled schema's `$id` URL.
///
/// Extracted from the `schema_version` enum value, e.g.
/// `https://notarai.dev/schema/0.5/spec.schema.json` -> `"0.5"`.
fn bundled_version() -> Option<&'static str> {
    schema::schema()
        .get("properties")
        .and_then(|p| p.get("schema_version"))
        .and_then(|sv| sv.get("enum"))
        .and_then(|e| e.as_array())
        .and_then(|arr| arr.first())
        .and_then(|v| v.as_str())
}

/// Locate the project root by walking up from `cwd` until a `.notarai/` directory
/// is found, or return `cwd` as a fallback.
fn find_project_root(cwd: &Path) -> PathBuf {
    let mut current = cwd.to_path_buf();
    loop {
        if current.join(".notarai").is_dir() {
            return current;
        }
        match current.parent() {
            Some(p) => current = p.to_path_buf(),
            None => return cwd.to_path_buf(),
        }
    }
}

/// Detect the schema version recorded in `.notarai/notarai.spec.json`.
///
/// Returns `None` if the file is absent or cannot be parsed.
fn project_schema_version(project_root: &Path) -> Option<String> {
    let schema_path = project_root.join(".notarai/notarai.spec.json");
    let content = std::fs::read_to_string(&schema_path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&content).ok()?;
    // Try enum first (our own format), then $id URL as fallback.
    if let Some(v) = json
        .get("properties")
        .and_then(|p| p.get("schema_version"))
        .and_then(|sv| sv.get("enum"))
        .and_then(|e| e.as_array())
        .and_then(|arr| arr.first())
        .and_then(|v| v.as_str())
    {
        return Some(v.to_string());
    }
    // Fallback: extract version segment from $id URL like
    // "https://notarai.dev/schema/0.5/spec.schema.json"
    let id = json.get("$id").and_then(|v| v.as_str())?;
    id.split('/').rev().nth(1).map(String::from)
}

/// Walk `.notarai/` and collect all `*.spec.yaml` file paths.
fn collect_spec_files(notarai_dir: &Path) -> Vec<PathBuf> {
    let mut specs = Vec::new();
    for entry in WalkDir::new(notarai_dir) {
        let Ok(entry) = entry else { continue };
        if !entry.file_type().is_file() {
            continue;
        }
        if entry
            .file_name()
            .to_str()
            .is_some_and(|n| n.ends_with(".spec.yaml"))
        {
            specs.push(entry.into_path());
        }
    }
    specs
}

/// Update all `*.spec.yaml` files in `.notarai/` by replacing `schema_version`
/// values from `old_version` to `new_version` using a text-safe replacement.
///
/// Both `'0.4'` and `"0.4"` quoting styles are handled.
fn update_spec_files(
    spec_files: &[PathBuf],
    old_version: &str,
    new_version: &str,
) -> Result<usize, String> {
    let single_old = format!("schema_version: '{old_version}'");
    let single_new = format!("schema_version: '{new_version}'");
    let double_old = format!("schema_version: \"{old_version}\"");
    let double_new = format!("schema_version: \"{new_version}\"");

    let mut updated = 0;
    for path in spec_files {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("could not read {}: {e}", path.display()))?;

        if !content.contains(&single_old) && !content.contains(&double_old) {
            continue;
        }

        let new_content = content
            .replace(&single_old, &single_new)
            .replace(&double_old, &double_new);

        std::fs::write(path, new_content)
            .map_err(|e| format!("could not write {}: {e}", path.display()))?;
        updated += 1;
    }
    Ok(updated)
}

/// Update schema version across all `.notarai/*.spec.yaml` files and the
/// `.notarai/notarai.spec.json` copy to match the bundled schema version.
pub fn run(project_root: Option<&Path>) -> i32 {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let root = match project_root {
        Some(p) => p.to_path_buf(),
        None => find_project_root(&cwd),
    };

    let new_version = match bundled_version() {
        Some(v) => v,
        None => {
            eprintln!("Error: could not determine bundled schema version");
            return 1;
        }
    };

    let old_version = project_schema_version(&root);

    match &old_version {
        Some(v) if v == new_version => {
            println!("Already at current schema version ({new_version})");
            return 0;
        }
        Some(v) => println!("Updating schema from {v} to {new_version}"),
        None => println!("No local schema found, installing version {new_version}"),
    }

    // Overwrite .notarai/notarai.spec.json with bundled schema
    let notarai_dir = root.join(".notarai");
    if !notarai_dir.exists()
        && let Err(e) = std::fs::create_dir_all(&notarai_dir)
    {
        eprintln!("Error: could not create .notarai/ directory: {e}");
        return 1;
    }
    let schema_dest = notarai_dir.join("notarai.spec.json");
    if let Err(e) = std::fs::write(&schema_dest, crate::core::schema::SCHEMA_STR) {
        eprintln!("Error: could not write .notarai/notarai.spec.json: {e}");
        return 1;
    }

    // Update all spec files
    let spec_files = collect_spec_files(&notarai_dir);
    if spec_files.is_empty() {
        println!("No spec files found in .notarai/");
        return 0;
    }

    let old = old_version.as_deref().unwrap_or("0.0");
    let updated = match update_spec_files(&spec_files, old, new_version) {
        Ok(n) => n,
        Err(e) => {
            eprintln!("Error: {e}");
            return 1;
        }
    };

    // Validate all updated specs
    let mut has_failure = false;
    for path in &spec_files {
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("FAIL {}: {e}", path.display());
                has_failure = true;
                continue;
            }
        };
        let result = validator::validate_spec(&content);
        if !result.valid {
            eprintln!("FAIL {}", path.display());
            for err in &result.errors {
                eprintln!("  {err}");
            }
            has_failure = true;
        }
    }

    if has_failure {
        eprintln!("Validation failed after schema bump -- please review the errors above");
        return 1;
    }

    println!(
        "Updated {updated} spec file(s) from {} to {new_version}",
        old
    );
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_version_is_current() {
        let v = bundled_version().expect("bundled version present");
        assert!(!v.is_empty());
    }

    #[test]
    fn update_spec_files_handles_single_quotes() {
        let tmp = tempfile::TempDir::new().unwrap();
        let spec = tmp.path().join("test.spec.yaml");
        std::fs::write(
            &spec,
            "schema_version: '0.4'\nintent: 'test'\nbehaviors: []\nartifacts: {}\n",
        )
        .unwrap();
        let updated = update_spec_files(&[spec.clone()], "0.4", "0.5").unwrap();
        assert_eq!(updated, 1);
        let content = std::fs::read_to_string(&spec).unwrap();
        assert!(content.contains("schema_version: '0.5'"));
    }

    #[test]
    fn update_spec_files_handles_double_quotes() {
        let tmp = tempfile::TempDir::new().unwrap();
        let spec = tmp.path().join("test.spec.yaml");
        std::fs::write(
            &spec,
            "schema_version: \"0.4\"\nintent: 'test'\nbehaviors: []\nartifacts: {}\n",
        )
        .unwrap();
        let updated = update_spec_files(&[spec.clone()], "0.4", "0.5").unwrap();
        assert_eq!(updated, 1);
        let content = std::fs::read_to_string(&spec).unwrap();
        assert!(content.contains("schema_version: \"0.5\""));
    }
}
