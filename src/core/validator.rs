use crate::core::schema;
use crate::core::yaml;
use jsonschema::Validator;
use std::sync::OnceLock;

pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
}

static VALIDATOR: OnceLock<Validator> = OnceLock::new();

fn validator() -> &'static Validator {
    VALIDATOR.get_or_init(|| {
        jsonschema::validator_for(schema::schema())
            .expect("bundled schema compiles to a valid validator")
    })
}

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
schema_version: \"0.4\"
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
        let result = validate_spec("schema_version: \"0.4\"\n");
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
