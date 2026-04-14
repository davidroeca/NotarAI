use serde_json::Value;

pub fn parse_yaml(content: &str) -> Result<Value, String> {
    let yaml_value: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(content).map_err(|e| e.to_string())?;

    // Convert YAML value to JSON value for jsonschema validation
    serde_json::to_value(yaml_value).map_err(|e| format!("YAML to JSON conversion error: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_yaml() {
        let data = parse_yaml("foo: bar\nnum: 42").expect("expected Ok");
        assert_eq!(data["foo"], "bar");
        assert_eq!(data["num"], 42);
    }

    #[test]
    fn returns_err_for_invalid_yaml() {
        assert!(parse_yaml("foo: [unterminated").is_err());
    }

    #[test]
    fn returns_null_for_empty_string() {
        let data = parse_yaml("").expect("expected Ok");
        assert!(data.is_null());
    }
}
