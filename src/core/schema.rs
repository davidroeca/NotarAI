use serde_json::Value;
use std::sync::OnceLock;

const SCHEMA_STR: &str = include_str!("../../notarai.spec.json");

static SCHEMA: OnceLock<Value> = OnceLock::new();

pub fn schema() -> &'static Value {
    SCHEMA.get_or_init(|| serde_json::from_str(SCHEMA_STR).expect("bundled schema is valid JSON"))
}

pub fn schema_id() -> Option<&'static str> {
    schema().get("$id").and_then(|v| v.as_str())
}
