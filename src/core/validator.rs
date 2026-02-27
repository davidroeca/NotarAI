use crate::core::schema;
use crate::core::yaml;
use jsonschema::Validator;
use std::sync::OnceLock;

pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
}

static VALIDATOR: OnceLock<Validator> = OnceLock::new();

/// Return the compiled jsonschema validator for the bundled schema.
///
/// Compiled once on first call via `OnceLock`. Panics if the bundled schema
/// cannot be compiled -- which would indicate a defect in the bundled JSON
/// Schema, not in user input.
fn validator() -> &'static Validator {
    VALIDATOR.get_or_init(|| {
        jsonschema::validator_for(schema::schema())
            .expect("bundled schema compiles to a valid validator")
    })
}

/// Validate a YAML spec string against the bundled NotarAI JSON Schema.
///
/// Parses `content` as YAML, converts it to a JSON value, then runs the
/// compiled validator. Returns a `ValidationResult` with `valid: true` and an
/// empty error list on success, or `valid: false` with human-readable error
/// strings on failure.
pub fn validate_spec(content: &str) -> ValidationResult {
    let data = match yaml::parse_yaml(content) {
        Ok(v) => v,
        Err(e) => {
            return ValidationResult {
                valid: false,
                errors: vec![format!("YAML parse error: {e}")],
            };
        }
    };

    let errors: Vec<String> = validator()
        .iter_errors(&data)
        .map(|err| {
            let path = err.instance_path().to_string();
            let path = if path.is_empty() {
                "/".to_string()
            } else {
                path
            };
            format!("{path}: {}", err)
        })
        .collect();

    if errors.is_empty() {
        ValidationResult {
            valid: true,
            errors: vec![],
        }
    } else {
        ValidationResult {
            valid: false,
            errors,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    const MINIMAL_VALID: &str = "\
schema_version: \"0.5\"
intent: \"Test intent\"
behaviors:
  - name: b
    given: \"some precondition\"
    then: \"expected outcome\"
artifacts:
  code:
    - path: \"src/**\"
";

    fn fixture_spec() -> String {
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(".notarai/cli.spec.yaml"))
            .expect("fixture spec exists")
    }

    #[test]
    fn validates_minimal_inline_spec() {
        let result = validate_spec(MINIMAL_VALID);
        assert!(result.valid, "errors: {:?}", result.errors);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn validates_real_spec_as_valid() {
        let result = validate_spec(&fixture_spec());
        assert!(result.valid, "errors: {:?}", result.errors);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn returns_invalid_for_missing_required_fields() {
        let result = validate_spec("schema_version: \"0.5\"\n");
        assert!(!result.valid);
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn returns_yaml_parse_error_for_malformed_yaml() {
        let result = validate_spec("foo: [unterminated");
        assert!(!result.valid);
        assert!(result.errors[0].contains("YAML parse error"));
    }

    #[test]
    fn returns_invalid_for_wrong_schema_version() {
        let result = validate_spec("schema_version: \"99.99\"\nintent: \"test\"\n");
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.contains("schema_version")));
    }
}
