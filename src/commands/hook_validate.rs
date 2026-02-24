use crate::core::validator;
use std::fs;
use std::io::{self, Read};
use std::path::Path;

pub struct HookResult {
    pub exit_code: i32,
    pub errors: Vec<String>,
    pub file_path: Option<String>,
}

fn is_spec_file(file_path: &str, cwd: &Path) -> bool {
    let Ok(rel) = pathdiff(file_path, cwd) else {
        return false;
    };
    rel.starts_with(".notarai/") && rel.ends_with(".spec.yaml")
}

fn pathdiff(file_path: &str, base: &Path) -> Result<String, ()> {
    let file = Path::new(file_path);
    match file.strip_prefix(base) {
        Ok(rel) => Ok(rel.to_string_lossy().to_string()),
        Err(_) => Err(()),
    }
}

pub fn process_hook_input(input: &str, cwd: &Path) -> HookResult {
    let payload: serde_json::Value = match serde_json::from_str(input) {
        Ok(v) => v,
        Err(_) => {
            return HookResult {
                exit_code: 0,
                errors: vec![],
                file_path: None,
            };
        }
    };

    let file_path = payload
        .get("tool_input")
        .and_then(|ti| ti.get("file_path"))
        .and_then(|fp| fp.as_str());

    let file_path = match file_path {
        Some(fp) => fp,
        None => {
            return HookResult {
                exit_code: 0,
                errors: vec![],
                file_path: None,
            };
        }
    };

    if !is_spec_file(file_path, cwd) {
        return HookResult {
            exit_code: 0,
            errors: vec![],
            file_path: None,
        };
    }

    let content = match fs::read_to_string(file_path) {
        Ok(c) => c,
        Err(_) => {
            return HookResult {
                exit_code: 0,
                errors: vec![],
                file_path: Some(file_path.to_string()),
            };
        }
    };

    let result = validator::validate_spec(&content);

    if result.valid {
        HookResult {
            exit_code: 0,
            errors: vec![],
            file_path: Some(file_path.to_string()),
        }
    } else {
        HookResult {
            exit_code: 1,
            errors: result.errors,
            file_path: Some(file_path.to_string()),
        }
    }
}

pub fn run() -> i32 {
    let mut input = String::new();
    if io::stdin().read_to_string(&mut input).is_err() {
        return 0;
    }

    let cwd = std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf());
    let result = process_hook_input(&input, &cwd);

    if result.exit_code != 0 {
        if let Some(ref path) = result.file_path {
            eprintln!("Spec validation failed: {path}");
        }
        for err in &result.errors {
            eprintln!("  {err}");
        }
    }

    result.exit_code
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, create_dir_all};
    use tempfile::TempDir;

    const VALID_SPEC_YAML: &str = "\
schema_version: \"0.4\"
intent: \"Test spec\"
behaviors:
  - name: b1
    given: \"x\"
    then: \"y\"
artifacts:
  code:
    - path: \"src/foo.ts\"
      role: \"test\"
";

    #[test]
    fn returns_0_for_non_spec_file() {
        let tmp = TempDir::new().unwrap();
        let input = serde_json::json!({
            "tool_input": { "file_path": tmp.path().join("src/foo.ts").to_str().unwrap() }
        });
        let result = process_hook_input(&input.to_string(), tmp.path());
        assert_eq!(result.exit_code, 0);
    }

    #[test]
    fn returns_0_for_malformed_json() {
        let tmp = TempDir::new().unwrap();
        let result = process_hook_input("not json!", tmp.path());
        assert_eq!(result.exit_code, 0);
    }

    #[test]
    fn returns_0_for_missing_file_path() {
        let tmp = TempDir::new().unwrap();
        let input = serde_json::json!({ "tool_input": {} });
        let result = process_hook_input(&input.to_string(), tmp.path());
        assert_eq!(result.exit_code, 0);
    }

    #[test]
    fn returns_0_for_valid_spec() {
        let tmp = TempDir::new().unwrap();
        let spec_dir = tmp.path().join(".notarai");
        create_dir_all(&spec_dir).unwrap();
        let spec_path = spec_dir.join("test.spec.yaml");
        fs::write(&spec_path, VALID_SPEC_YAML).unwrap();

        let input = serde_json::json!({
            "tool_input": { "file_path": spec_path.to_str().unwrap() }
        });
        let result = process_hook_input(&input.to_string(), tmp.path());
        assert_eq!(result.exit_code, 0);
    }

    #[test]
    fn returns_1_for_invalid_spec() {
        let tmp = TempDir::new().unwrap();
        let spec_dir = tmp.path().join(".notarai");
        create_dir_all(&spec_dir).unwrap();
        let spec_path = spec_dir.join("test.spec.yaml");
        fs::write(&spec_path, "schema_version: \"0.4\"\n").unwrap();

        let input = serde_json::json!({
            "tool_input": { "file_path": spec_path.to_str().unwrap() }
        });
        let result = process_hook_input(&input.to_string(), tmp.path());
        assert_eq!(result.exit_code, 1);
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn returns_0_for_missing_file_on_disk() {
        let tmp = TempDir::new().unwrap();
        let spec_path = tmp.path().join(".notarai/missing.spec.yaml");
        let input = serde_json::json!({
            "tool_input": { "file_path": spec_path.to_str().unwrap() }
        });
        let result = process_hook_input(&input.to_string(), tmp.path());
        assert_eq!(result.exit_code, 0);
    }
}
