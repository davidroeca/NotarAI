use crate::core::schema;
use crate::core::validator;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

fn find_spec_files(dir: &Path) -> Vec<String> {
    let mut results = Vec::new();
    for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file()
            && entry
                .file_name()
                .to_str()
                .is_some_and(|n| n.ends_with(".spec.yaml"))
        {
            results.push(entry.path().to_string_lossy().to_string());
        }
    }
    results.sort();
    results
}

fn check_schema_freshness() {
    let local_path = Path::new(".claude/notarai.spec.json");
    if !local_path.exists() {
        return;
    }

    let local_content = match fs::read_to_string(local_path) {
        Ok(c) => c,
        Err(_) => return,
    };

    let local: serde_json::Value = match serde_json::from_str(&local_content) {
        Ok(v) => v,
        Err(_) => return,
    };

    let bundled_id = schema::schema_id();
    let local_id = local.get("$id").and_then(|v| v.as_str());

    if bundled_id != local_id {
        eprintln!(
            "Warning: .claude/notarai.spec.json is out of date (local: {}, bundled: {}). Run `notarai init` to update.",
            local_id.unwrap_or("unknown"),
            bundled_id.unwrap_or("unknown"),
        );
    }
}

pub fn run(path: Option<String>) -> i32 {
    check_schema_freshness();

    let target = path.unwrap_or_else(|| ".notarai".to_string());
    let resolved = Path::new(&target);

    let files = if resolved.is_dir() {
        find_spec_files(resolved)
    } else if resolved.is_file() {
        vec![resolved.to_string_lossy().to_string()]
    } else {
        eprintln!("Error: path not found: {}", resolved.display());
        return 1;
    };

    if files.is_empty() {
        eprintln!(
            "Warning: no .spec.yaml files found in {}",
            resolved.display()
        );
        return 0;
    }

    let mut has_failure = false;

    for file in &files {
        let content = match fs::read_to_string(file) {
            Ok(c) => c,
            Err(e) => {
                println!("FAIL {file}");
                println!("  Could not read file: {e}");
                has_failure = true;
                continue;
            }
        };

        let result = validator::validate_spec(&content);

        if result.valid {
            println!("PASS {file}");
        } else {
            has_failure = true;
            println!("FAIL {file}");
            for err in &result.errors {
                println!("  {err}");
            }
        }
    }

    if has_failure {
        1
    } else {
        0
    }
}
