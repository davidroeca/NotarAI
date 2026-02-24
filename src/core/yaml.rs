use serde_json::Value;

pub enum YamlResult {
    Ok(Value),
    Err(String),
}

pub fn parse_yaml(content: &str) -> YamlResult {
    let yaml_value: serde_yaml_ng::Value = match serde_yaml_ng::from_str(content) {
        Ok(v) => v,
        Err(e) => return YamlResult::Err(e.to_string()),
    };

    // Convert YAML value to JSON value for jsonschema validation
    match serde_json::to_value(yaml_value) {
        Ok(v) => YamlResult::Ok(v),
        Err(e) => YamlResult::Err(format!("YAML to JSON conversion error: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_yaml() {
        match parse_yaml("foo: bar\nnum: 42") {
            YamlResult::Ok(data) => {
                assert_eq!(data["foo"], "bar");
                assert_eq!(data["num"], 42);
            }
            YamlResult::Err(e) => panic!("expected Ok, got Err: {e}"),
        }
    }

    #[test]
    fn returns_err_for_invalid_yaml() {
        match parse_yaml("foo: [unterminated") {
            YamlResult::Err(_) => {}
            YamlResult::Ok(_) => panic!("expected Err"),
        }
    }

    #[test]
    fn returns_null_for_empty_string() {
        match parse_yaml("") {
            YamlResult::Ok(data) => assert!(data.is_null()),
            YamlResult::Err(e) => panic!("expected Ok, got Err: {e}"),
        }
    }
}
