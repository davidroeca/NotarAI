use crate::core::mcp_tools;
use serde::{Deserialize, Serialize};
use std::io::BufRead;
use std::io::Write;

#[allow(dead_code)]
#[derive(Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: Option<serde_json::Value>,
    method: String,
    params: Option<serde_json::Value>,
}

#[derive(Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcErrorObj>,
}

#[derive(Serialize)]
struct JsonRpcErrorObj {
    code: i32,
    message: String,
}

pub fn run() -> i32 {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let root = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        let line = line.trim().to_string();
        if line.is_empty() {
            continue;
        }

        let req: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                let resp = error_response(None, -32700, format!("Parse error: {e}"));
                writeln!(out, "{}", serde_json::to_string(&resp).unwrap()).ok();
                continue;
            }
        };

        // Notifications don't get responses
        if req.method.starts_with("notifications/") {
            continue;
        }

        let resp = dispatch(&req, &root);
        writeln!(out, "{}", serde_json::to_string(&resp).unwrap()).ok();
    }

    0
}

fn dispatch(req: &JsonRpcRequest, root: &std::path::Path) -> JsonRpcResponse {
    match req.method.as_str() {
        "initialize" => handle_initialize(req),
        "tools/list" => handle_tools_list(req),
        "tools/call" => handle_tools_call(req, root),
        _ => error_response(
            req.id.clone(),
            -32601,
            format!("Method not found: {}", req.method),
        ),
    }
}

fn handle_initialize(req: &JsonRpcRequest) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id: req.id.clone(),
        result: Some(serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {"tools": {}},
            "serverInfo": {
                "name": "notarai",
                "version": env!("CARGO_PKG_VERSION"),
            },
            "tools": tools_list(),
        })),
        error: None,
    }
}

fn handle_tools_list(req: &JsonRpcRequest) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id: req.id.clone(),
        result: Some(serde_json::json!({"tools": tools_list()})),
        error: None,
    }
}

fn tools_list() -> serde_json::Value {
    serde_json::json!([
        {
            "name": "list_affected_specs",
            "description": "List specs affected by changes on the current branch vs base branch",
            "inputSchema": {
                "type": "object",
                "required": ["base_branch"],
                "properties": {
                    "base_branch": {"type": "string", "description": "The base branch to diff against"}
                }
            }
        },
        {
            "name": "get_spec_diff",
            "description": "Get the git diff filtered to files governed by a specific spec",
            "inputSchema": {
                "type": "object",
                "required": ["spec_path", "base_branch"],
                "properties": {
                    "spec_path": {"type": "string", "description": "Relative path to the spec file"},
                    "base_branch": {"type": "string", "description": "The base branch to diff against"}
                }
            }
        },
        {
            "name": "get_changed_artifacts",
            "description": "Get artifacts governed by a spec that have changed since last cache update",
            "inputSchema": {
                "type": "object",
                "required": ["spec_path"],
                "properties": {
                    "spec_path": {"type": "string", "description": "Relative path to the spec file"},
                    "artifact_type": {"type": "string", "description": "Optional artifact type filter (e.g. 'docs', 'code')"}
                }
            }
        },
        {
            "name": "mark_reconciled",
            "description": "Update the cache for files after reconciliation",
            "inputSchema": {
                "type": "object",
                "required": ["files"],
                "properties": {
                    "files": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Relative file paths to cache"
                    }
                }
            }
        }
    ])
}

fn handle_tools_call(req: &JsonRpcRequest, root: &std::path::Path) -> JsonRpcResponse {
    let Some(params) = req.params.as_ref() else {
        return error_response(req.id.clone(), -32602, "Missing params".to_string());
    };

    let Some(tool_name) = params.get("name").and_then(|n| n.as_str()) else {
        return error_response(req.id.clone(), -32602, "Missing tool name".to_string());
    };

    let args = params
        .get("arguments")
        .cloned()
        .unwrap_or(serde_json::json!({}));

    let result = match tool_name {
        "list_affected_specs" => {
            let base = args
                .get("base_branch")
                .and_then(|b| b.as_str())
                .unwrap_or("main");
            mcp_tools::list_affected_specs(base, root)
        }
        "get_spec_diff" => {
            let Some(spec) = args.get("spec_path").and_then(|s| s.as_str()) else {
                return error_response(
                    req.id.clone(),
                    -32602,
                    "Missing spec_path".to_string(),
                );
            };
            let base = args
                .get("base_branch")
                .and_then(|b| b.as_str())
                .unwrap_or("main");
            mcp_tools::get_spec_diff(spec, base, root)
        }
        "get_changed_artifacts" => {
            let Some(spec) = args.get("spec_path").and_then(|s| s.as_str()) else {
                return error_response(
                    req.id.clone(),
                    -32602,
                    "Missing spec_path".to_string(),
                );
            };
            let art_type = args.get("artifact_type").and_then(|t| t.as_str());
            mcp_tools::get_changed_artifacts(spec, art_type, root)
        }
        "mark_reconciled" => {
            let Some(arr) = args.get("files").and_then(|f| f.as_array()) else {
                return error_response(req.id.clone(), -32602, "Missing files".to_string());
            };
            let files: Vec<String> = arr
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
            mcp_tools::mark_reconciled(&files, root)
        }
        _ => Err(mcp_tools::McpError {
            code: -32601,
            message: format!("Unknown tool: {tool_name}"),
        }),
    };

    match result {
        Ok(value) => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: req.id.clone(),
            result: Some(serde_json::json!({
                "content": [{"type": "text", "text": value.to_string()}]
            })),
            error: None,
        },
        Err(e) => error_response(req.id.clone(), e.code, e.message),
    }
}

fn error_response(id: Option<serde_json::Value>, code: i32, message: String) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: None,
        error: Some(JsonRpcErrorObj { code, message }),
    }
}
