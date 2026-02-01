//! MCP (Model Context Protocol) server for Claude Code integration.
//!
//! This module implements an MCP server that exposes worky operations
//! as tools that Claude Code can use directly.

mod protocol;
mod tools;

use anyhow::{Context, Result};
use protocol::{
    InitializeParams, InitializeResult, JsonRpcRequest, JsonRpcResponse, ServerCapabilities,
    ServerInfo, ToolCallParams, ToolsCapability, ToolsListResult,
};
use serde_json::json;
use std::io::{self, BufRead, Write};
use std::path::Path;
use tracing::{debug, error, info};

const PROTOCOL_VERSION: &str = "2024-11-05";
const SERVER_NAME: &str = "worky";
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Run the MCP server, reading from stdin and writing to stdout.
pub fn serve(workspace_path: &Path) -> Result<()> {
    info!("Starting MCP server for workspace: {}", workspace_path.display());

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = line.context("Failed to read from stdin")?;

        if line.trim().is_empty() {
            continue;
        }

        debug!("Received: {}", line);

        let request: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(req) => req,
            Err(e) => {
                error!("Failed to parse request: {}", e);
                let response = JsonRpcResponse::error(None, -32700, format!("Parse error: {e}"));
                write_response(&mut stdout, &response)?;
                continue;
            }
        };

        if let Some(response) = handle_request(workspace_path, &request) {
            write_response(&mut stdout, &response)?;
        }
    }

    Ok(())
}

fn write_response(stdout: &mut io::Stdout, response: &JsonRpcResponse) -> Result<()> {
    let json = serde_json::to_string(response)?;
    debug!("Sending: {}", json);
    writeln!(stdout, "{json}")?;
    stdout.flush()?;
    Ok(())
}

fn handle_request(workspace_path: &Path, request: &JsonRpcRequest) -> Option<JsonRpcResponse> {
    match request.method.as_str() {
        "initialize" => Some(handle_initialize(request)),
        "initialized" => {
            // Notification - no response expected
            debug!("Received initialized notification");
            None
        }
        "tools/list" => Some(handle_tools_list(request)),
        "tools/call" => Some(handle_tools_call(workspace_path, request)),
        "ping" => Some(JsonRpcResponse::success(request.id.clone(), json!({}))),
        "notifications/cancelled" => {
            // Notification - no response expected
            debug!("Received cancellation notification");
            None
        }
        method if method.starts_with("notifications/") => {
            // All notifications - no response expected
            debug!("Received notification: {}", method);
            None
        }
        _ => {
            error!("Unknown method: {}", request.method);
            Some(JsonRpcResponse::error(
                request.id.clone(),
                -32601,
                format!("Method not found: {}", request.method),
            ))
        }
    }
}

fn handle_initialize(request: &JsonRpcRequest) -> JsonRpcResponse {
    let _params: InitializeParams = match &request.params {
        Some(params) => match serde_json::from_value(params.clone()) {
            Ok(p) => p,
            Err(e) => {
                return JsonRpcResponse::error(
                    request.id.clone(),
                    -32602,
                    format!("Invalid params: {e}"),
                )
            }
        },
        None => {
            return JsonRpcResponse::error(request.id.clone(), -32602, "Missing params")
        }
    };

    let result = InitializeResult {
        protocol_version: PROTOCOL_VERSION.to_string(),
        capabilities: ServerCapabilities {
            tools: ToolsCapability { list_changed: false },
        },
        server_info: ServerInfo {
            name: SERVER_NAME.to_string(),
            version: SERVER_VERSION.to_string(),
        },
    };

    JsonRpcResponse::success(request.id.clone(), serde_json::to_value(result).unwrap())
}

fn handle_tools_list(request: &JsonRpcRequest) -> JsonRpcResponse {
    let tools = tools::get_tool_definitions();
    let result = ToolsListResult { tools };
    JsonRpcResponse::success(request.id.clone(), serde_json::to_value(result).unwrap())
}

fn handle_tools_call(workspace_path: &Path, request: &JsonRpcRequest) -> JsonRpcResponse {
    let params: ToolCallParams = match &request.params {
        Some(params) => match serde_json::from_value(params.clone()) {
            Ok(p) => p,
            Err(e) => {
                return JsonRpcResponse::error(
                    request.id.clone(),
                    -32602,
                    format!("Invalid params: {e}"),
                )
            }
        },
        None => {
            return JsonRpcResponse::error(request.id.clone(), -32602, "Missing params")
        }
    };

    info!("Tool call: {} with args: {:?}", params.name, params.arguments);

    let result = tools::handle_tool_call(workspace_path, &params.name, params.arguments);

    JsonRpcResponse::success(request.id.clone(), serde_json::to_value(result).unwrap())
}
