// Atlas OS — MCP wire protocol (RFC 07 §1).
//
// The Model Context Protocol speaks JSON-RPC 2.0 over stdio, one JSON
// object per line (newline-delimited). This module owns the *pure* half
// of the client: request/response/notification shapes, the method names
// and the payload parsers. Nothing here touches a process or an I/O
// stream, so every shape is unit-testable without a server.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const JSONRPC_VERSION: &str = "2.0";

/// MCP revision Atlas advertises in `initialize`. The server may answer
/// with a different (older) revision; we accept what it returns rather
/// than hard-failing, which is what the spec asks clients to do.
pub const MCP_PROTOCOL_VERSION: &str = "2025-06-18";

pub const METHOD_INITIALIZE: &str = "initialize";
pub const METHOD_INITIALIZED: &str = "notifications/initialized";
pub const METHOD_TOOLS_LIST: &str = "tools/list";
pub const METHOD_TOOLS_CALL: &str = "tools/call";

/// Client identity sent in `initialize` (`clientInfo`).
pub const CLIENT_NAME: &str = "atlas-os";

#[derive(Clone, Debug, Serialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: &'static str,
    pub id: u64,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl JsonRpcRequest {
    pub fn new(id: u64, method: impl Into<String>, params: Option<Value>) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION,
            id,
            method: method.into(),
            params,
        }
    }
}

/// A JSON-RPC notification carries no `id` and expects no response
/// (`notifications/initialized` is the only one Fase 29.0 sends).
#[derive(Clone, Debug, Serialize)]
pub struct JsonRpcNotification {
    pub jsonrpc: &'static str,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl JsonRpcNotification {
    pub fn new(method: impl Into<String>, params: Option<Value>) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION,
            method: method.into(),
            params,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct JsonRpcResponse {
    #[serde(default)]
    pub jsonrpc: String,
    #[serde(default)]
    pub id: Option<u64>,
    #[serde(default)]
    pub result: Option<Value>,
    #[serde(default)]
    pub error: Option<JsonRpcError>,
}

impl JsonRpcResponse {
    pub fn parse(line: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(line)
    }

    /// A response is ours when the id matches. Notifications (no id) and
    /// responses to other in-flight ids are skipped by the client loop.
    pub fn is_response_to(&self, id: u64) -> bool {
        self.id == Some(id)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl std::fmt::Display for JsonRpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "rpc error {}: {}", self.code, self.message)
    }
}

/// Identity the server reports back from `initialize` (`serverInfo`).
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
pub struct ServerIdent {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub version: String,
}

/// Parsed `initialize` result. Unknown fields are ignored so a newer
/// server revision does not break the client.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
pub struct ServerInfo {
    #[serde(default, rename = "protocolVersion")]
    pub protocol_version: Option<String>,
    #[serde(default, rename = "serverInfo")]
    pub server_info: Option<ServerIdent>,
}

impl ServerInfo {
    pub fn display_name(&self) -> String {
        match &self.server_info {
            Some(id) if !id.name.is_empty() => {
                format!("{} {}", id.name, id.version).trim().to_string()
            }
            _ => "(unnamed server)".to_string(),
        }
    }
}

/// One entry of `tools/list` (RFC 07 §4).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolInfo {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(
        default,
        rename = "inputSchema",
        skip_serializing_if = "Option::is_none"
    )]
    pub input_schema: Option<Value>,
}

/// Parsed `tools/call` result: the concatenated text content plus the
/// `isError` flag. `raw` keeps the full payload for the Journal.
#[derive(Clone, Debug, PartialEq)]
pub struct ToolCallOutcome {
    pub text: String,
    pub is_error: bool,
    pub raw: Value,
}

impl ToolCallOutcome {
    pub fn from_result(result: &Value) -> Self {
        let is_error = result
            .get("isError")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let text = result
            .get("content")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(|c| c.get("text").and_then(Value::as_str))
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_default();
        Self {
            text,
            is_error,
            raw: result.clone(),
        }
    }
}

/// `initialize` params: protocol revision + capabilities + clientInfo.
pub fn initialize_params(client_version: &str) -> Value {
    json!({
        "protocolVersion": MCP_PROTOCOL_VERSION,
        "capabilities": {},
        "clientInfo": { "name": CLIENT_NAME, "version": client_version },
    })
}

/// `tools/list` params. The spec allows an optional `cursor`; Fase 29.0
/// does not paginate yet, so an empty object is sent.
pub fn tools_list_params() -> Value {
    json!({})
}

/// `tools/call` params for `name` with `arguments`.
pub fn tools_call_params(tool: &str, arguments: &Value) -> Value {
    json!({ "name": tool, "arguments": arguments })
}

/// Extract the tool list from a `tools/list` result, skipping entries
/// that fail to deserialize rather than discarding the whole batch.
pub fn parse_tools(result: &Value) -> Vec<ToolInfo> {
    result
        .get("tools")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|t| serde_json::from_value::<ToolInfo>(t.clone()).ok())
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_serializes_with_jsonrpc_and_id() {
        let req = JsonRpcRequest::new(7, METHOD_TOOLS_LIST, Some(tools_list_params()));
        let v: Value = serde_json::to_value(&req).unwrap();
        assert_eq!(v["jsonrpc"], JSONRPC_VERSION);
        assert_eq!(v["id"], 7);
        assert_eq!(v["method"], "tools/list");
        assert_eq!(v["params"], json!({}));
    }

    #[test]
    fn request_omits_null_params() {
        let req = JsonRpcRequest::new(1, METHOD_INITIALIZED, None);
        let v: Value = serde_json::to_value(&req).unwrap();
        assert!(v.get("params").is_none());
    }

    #[test]
    fn notification_has_no_id() {
        let n = JsonRpcNotification::new(METHOD_INITIALIZED, None);
        let v: Value = serde_json::to_value(&n).unwrap();
        assert!(v.get("id").is_none());
        assert_eq!(v["method"], "notifications/initialized");
    }

    #[test]
    fn response_parses_result_and_matches_id() {
        let resp =
            JsonRpcResponse::parse(r#"{"jsonrpc":"2.0","id":3,"result":{"ok":true}}"#).unwrap();
        assert!(resp.is_response_to(3));
        assert!(!resp.is_response_to(4));
        assert_eq!(resp.result.unwrap()["ok"], true);
        assert!(resp.error.is_none());
    }

    #[test]
    fn response_parses_error() {
        let resp = JsonRpcResponse::parse(
            r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32601,"message":"nope"}}"#,
        )
        .unwrap();
        let err = resp.error.expect("error present");
        assert_eq!(err.code, -32601);
        assert!(err.to_string().contains("nope"));
    }

    #[test]
    fn parse_tools_reads_name_description_schema() {
        let result = json!({
            "tools": [
                { "name": "get_library_docs", "description": "docs", "inputSchema": {"type":"object"} },
                { "name": "search_docs" }
            ]
        });
        let tools = parse_tools(&result);
        assert_eq!(tools.len(), 2);
        assert_eq!(tools[0].name, "get_library_docs");
        assert_eq!(tools[0].description.as_deref(), Some("docs"));
        assert!(tools[0].input_schema.is_some());
        assert!(tools[1].input_schema.is_none());
    }

    #[test]
    fn parse_tools_missing_field_is_empty() {
        assert!(parse_tools(&json!({})).is_empty());
    }

    #[test]
    fn tool_call_outcome_concatenates_text_and_flags_error() {
        let result = json!({
            "content": [{ "type": "text", "text": "line one" }, { "type": "text", "text": "line two" }],
            "isError": true
        });
        let out = ToolCallOutcome::from_result(&result);
        assert_eq!(out.text, "line one\nline two");
        assert!(out.is_error);
        assert_eq!(out.raw["isError"], true);
    }

    #[test]
    fn tool_call_outcome_defaults_not_error() {
        let out = ToolCallOutcome::from_result(&json!({ "content": [] }));
        assert!(!out.is_error);
        assert_eq!(out.text, "");
    }

    #[test]
    fn server_info_display_name_falls_back() {
        let unnamed = ServerInfo::default();
        assert_eq!(unnamed.display_name(), "(unnamed server)");
        let named = ServerInfo {
            protocol_version: Some("2025-06-18".into()),
            server_info: Some(ServerIdent {
                name: "context7".into(),
                version: "3.2.4".into(),
            }),
        };
        assert_eq!(named.display_name(), "context7 3.2.4");
    }
}
