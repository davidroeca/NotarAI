use assert_cmd::cargo_bin_cmd;
use predicates::prelude::*;

fn notarai() -> assert_cmd::Command {
    cargo_bin_cmd!("notarai")
}

const INITIALIZE_MSG: &str = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{}}}"#;

#[test]
fn responds_to_initialize_with_server_info_and_tools() {
    notarai()
        .arg("mcp")
        .write_stdin(format!("{INITIALIZE_MSG}\n"))
        .assert()
        .success()
        .stdout(predicate::str::contains("serverInfo"))
        .stdout(predicate::str::contains("list_affected_specs"))
        .stdout(predicate::str::contains("get_spec_diff"))
        .stdout(predicate::str::contains("get_changed_artifacts"))
        .stdout(predicate::str::contains("mark_reconciled"));
}

#[test]
fn unknown_method_returns_32601() {
    notarai()
        .arg("mcp")
        .write_stdin(
            r#"{"jsonrpc":"2.0","id":1,"method":"nonexistent","params":{}}"#,
        )
        .assert()
        .success()
        .stdout(predicate::str::contains("-32601"));
}

#[test]
fn exits_0_on_stdin_eof() {
    notarai()
        .arg("mcp")
        .write_stdin("")
        .assert()
        .success();
}

#[test]
fn unknown_tool_returns_error() {
    let msg = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"nonexistent_tool","arguments":{}}}"#;
    notarai()
        .arg("mcp")
        .write_stdin(msg)
        .assert()
        .success()
        .stdout(predicate::str::contains("error"));
}
