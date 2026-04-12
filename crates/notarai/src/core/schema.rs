//! Bundled JSON Schema for NotarAI spec files.
//!
//! The schema is embedded at compile time via `include_str!` and parsed once
//! into a `serde_json::Value` via `OnceLock`. All callers share the same
//! parsed instance -- never re-parse per call.

use serde_json::Value;
use std::sync::OnceLock;

/// The bundled JSON Schema string, embedded at compile time.
pub const SCHEMA_STR: &str = include_str!("../../notarai.spec.json");

static SCHEMA: OnceLock<Value> = OnceLock::new();

/// Return the bundled schema as a parsed `serde_json::Value`.
///
/// Parsed once on first call; subsequent calls return the cached value.
/// Panics if the bundled schema is not valid JSON (should never happen in a
/// correctly built binary).
pub fn schema() -> &'static Value {
    SCHEMA.get_or_init(|| serde_json::from_str(SCHEMA_STR).expect("bundled schema is valid JSON"))
}

/// Return the `$id` URL from the bundled schema, if present.
///
/// Used by `notarai validate` to compare the local schema copy against the
/// bundled version and warn when they diverge.
pub fn schema_id() -> Option<&'static str> {
    schema().get("$id").and_then(|v| v.as_str())
}
