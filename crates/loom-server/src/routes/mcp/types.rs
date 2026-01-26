// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! JSON-RPC 2.0 and MCP protocol types.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// JSON-RPC 2.0 version string.
pub const JSONRPC_VERSION: &str = "2.0";

/// MCP protocol version.
pub const MCP_PROTOCOL_VERSION: &str = "2025-11-25";

/// Server name for MCP.
pub const MCP_SERVER_NAME: &str = "loom-mcp";

/// Server version for MCP.
pub const MCP_SERVER_VERSION: &str = "0.1.0";

/// JSON-RPC request ID - can be string, number, or null.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum JsonRpcId {
	String(String),
	Number(i64),
	Null,
}

/// JSON-RPC 2.0 request.
#[derive(Debug, Clone, Deserialize)]
pub struct JsonRpcRequest {
	pub jsonrpc: String,
	pub method: String,
	#[serde(default)]
	pub params: Option<Value>,
	#[serde(default)]
	pub id: Option<JsonRpcId>,
}

impl JsonRpcRequest {
	/// Check if this is a notification (no id).
	pub fn is_notification(&self) -> bool {
		self.id.is_none()
	}
}

/// JSON-RPC 2.0 error object.
#[derive(Debug, Clone, Serialize)]
pub struct JsonRpcError {
	pub code: i32,
	pub message: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub data: Option<Value>,
}

/// JSON-RPC 2.0 response.
#[derive(Debug, Clone, Serialize)]
pub struct JsonRpcResponse {
	pub jsonrpc: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub result: Option<Value>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub error: Option<JsonRpcError>,
	pub id: Option<JsonRpcId>,
}

impl JsonRpcResponse {
	/// Create a success response.
	pub fn success(id: Option<JsonRpcId>, result: Value) -> Self {
		Self {
			jsonrpc: JSONRPC_VERSION.to_string(),
			result: Some(result),
			error: None,
			id,
		}
	}

	/// Create an error response.
	pub fn error(id: Option<JsonRpcId>, error: JsonRpcError) -> Self {
		Self {
			jsonrpc: JSONRPC_VERSION.to_string(),
			result: None,
			error: Some(error),
			id,
		}
	}
}

/// MCP client info from initialize request.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientInfo {
	pub name: String,
	#[serde(default)]
	pub version: Option<String>,
}

/// MCP client capabilities from initialize request.
/// These fields are part of the MCP spec and kept for forward compatibility.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct ClientCapabilities {
	#[serde(default)]
	pub experimental: Option<HashMap<String, Value>>,
	#[serde(default)]
	pub sampling: Option<Value>,
}

/// MCP initialize request params.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeParams {
	pub protocol_version: String,
	/// Client capabilities - kept for forward compatibility with MCP spec.
	#[serde(default)]
	#[allow(dead_code)]
	pub capabilities: ClientCapabilities,
	pub client_info: ClientInfo,
}

/// MCP server info for initialize response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerInfo {
	pub name: String,
	pub version: String,
}

/// MCP server capabilities.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerCapabilities {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub tools: Option<ToolsCapability>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub resources: Option<ResourcesCapability>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub prompts: Option<PromptsCapability>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub experimental: Option<HashMap<String, Value>>,
}

/// Tools capability marker.
#[derive(Debug, Clone, Default, Serialize)]
pub struct ToolsCapability {}

/// Resources capability marker.
#[derive(Debug, Clone, Default, Serialize)]
pub struct ResourcesCapability {}

/// Prompts capability marker.
#[derive(Debug, Clone, Default, Serialize)]
pub struct PromptsCapability {}

/// MCP initialize response result.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeResult {
	pub protocol_version: String,
	pub capabilities: ServerCapabilities,
	pub server_info: ServerInfo,
}

impl Default for InitializeResult {
	fn default() -> Self {
		Self {
			protocol_version: MCP_PROTOCOL_VERSION.to_string(),
			capabilities: ServerCapabilities {
				tools: Some(ToolsCapability {}),
				resources: Some(ResourcesCapability {}),
				prompts: Some(PromptsCapability {}),
				experimental: None,
			},
			server_info: ServerInfo {
				name: MCP_SERVER_NAME.to_string(),
				version: MCP_SERVER_VERSION.to_string(),
			},
		}
	}
}

/// MCP tool definition.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tool {
	pub name: String,
	pub description: String,
	pub input_schema: Value,
}

/// MCP tools/list response result.
#[derive(Debug, Clone, Serialize)]
pub struct ToolsListResult {
	pub tools: Vec<Tool>,
}

/// MCP tools/call request params.
#[derive(Debug, Clone, Deserialize)]
pub struct ToolsCallParams {
	pub name: String,
	#[serde(default)]
	pub arguments: Option<Value>,
}

/// Content block in tool result.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ContentBlock {
	Text { text: String },
}

/// MCP tools/call response result.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolsCallResult {
	pub content: Vec<ContentBlock>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub is_error: Option<bool>,
}

impl ToolsCallResult {
	/// Create a success result with text content.
	pub fn text(text: impl Into<String>) -> Self {
		Self {
			content: vec![ContentBlock::Text { text: text.into() }],
			is_error: None,
		}
	}

	/// Create an error result with text content.
	pub fn error(text: impl Into<String>) -> Self {
		Self {
			content: vec![ContentBlock::Text { text: text.into() }],
			is_error: Some(true),
		}
	}
}

/// Arguments for create_weaver tool.
#[derive(Debug, Clone, Deserialize)]
pub struct CreateWeaverArgs {
	pub image: String,
	pub org_id: String,
	#[serde(default)]
	pub env: HashMap<String, String>,
	pub memory_limit: Option<String>,
	pub cpu_limit: Option<String>,
	pub lifetime_hours: Option<u32>,
	#[serde(default)]
	pub tags: HashMap<String, String>,
}

/// Arguments for list_weavers tool.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ListWeaversArgs {
	pub org_id: Option<String>,
	pub status: Option<String>,
	#[serde(default)]
	pub tags: HashMap<String, String>,
}

/// Arguments for get_weaver tool.
#[derive(Debug, Clone, Deserialize)]
pub struct GetWeaverArgs {
	pub weaver_id: String,
}

/// Arguments for delete_weaver tool.
#[derive(Debug, Clone, Deserialize)]
pub struct DeleteWeaverArgs {
	pub weaver_id: String,
}

/// Arguments for attach_weaver tool.
#[derive(Debug, Clone, Deserialize)]
pub struct AttachWeaverArgs {
	pub weaver_id: String,
}

// ============================================================================
// Resource Types
// ============================================================================

/// MCP resource definition.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Resource {
	pub uri: String,
	pub name: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub description: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub mime_type: Option<String>,
}

/// MCP resource template definition.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceTemplate {
	pub uri_template: String,
	pub name: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub description: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub mime_type: Option<String>,
}

/// MCP resources/list response result.
#[derive(Debug, Clone, Serialize)]
pub struct ResourcesListResult {
	pub resources: Vec<Resource>,
}

/// MCP resources/templates/list response result.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceTemplatesListResult {
	pub resource_templates: Vec<ResourceTemplate>,
}

/// MCP resources/read request params.
#[derive(Debug, Clone, Deserialize)]
pub struct ResourcesReadParams {
	pub uri: String,
}

/// Resource content block.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ResourceContent {
	Text {
		text: String,
		#[serde(skip_serializing_if = "Option::is_none")]
		mime_type: Option<String>,
	},
	Blob {
		blob: String, // base64 encoded
		#[serde(skip_serializing_if = "Option::is_none")]
		mime_type: Option<String>,
	},
}

/// MCP resources/read response result.
#[derive(Debug, Clone, Serialize)]
pub struct ResourcesReadResult {
	pub contents: Vec<ResourceContent>,
}

// ============================================================================
// Prompt Types
// ============================================================================

/// MCP prompt argument definition.
#[derive(Debug, Clone, Serialize)]
pub struct PromptArgument {
	pub name: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub description: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub required: Option<bool>,
}

/// MCP prompt definition.
#[derive(Debug, Clone, Serialize)]
pub struct Prompt {
	pub name: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub description: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub arguments: Option<Vec<PromptArgument>>,
}

/// MCP prompts/list response result.
#[derive(Debug, Clone, Serialize)]
pub struct PromptsListResult {
	pub prompts: Vec<Prompt>,
}

/// MCP prompts/get request params.
#[derive(Debug, Clone, Deserialize)]
pub struct PromptsGetParams {
	pub name: String,
	#[serde(default)]
	pub arguments: HashMap<String, String>,
}

/// Prompt message role.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PromptRole {
	User,
	Assistant,
}

/// Prompt message content.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum PromptContent {
	Text { text: String },
	Resource { resource: Resource },
}

/// Prompt message.
#[derive(Debug, Clone, Serialize)]
pub struct PromptMessage {
	pub role: PromptRole,
	pub content: PromptContent,
}

/// MCP prompts/get response result.
#[derive(Debug, Clone, Serialize)]
pub struct PromptsGetResult {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub description: Option<String>,
	pub messages: Vec<PromptMessage>,
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_json_rpc_id_deserialize_string() {
		let json = r#""test-id""#;
		let id: JsonRpcId = serde_json::from_str(json).unwrap();
		assert_eq!(id, JsonRpcId::String("test-id".to_string()));
	}

	#[test]
	fn test_json_rpc_id_deserialize_number() {
		let json = "42";
		let id: JsonRpcId = serde_json::from_str(json).unwrap();
		assert_eq!(id, JsonRpcId::Number(42));
	}

	#[test]
	fn test_json_rpc_id_deserialize_null() {
		let json = "null";
		let id: JsonRpcId = serde_json::from_str(json).unwrap();
		assert_eq!(id, JsonRpcId::Null);
	}

	#[test]
	fn test_json_rpc_request_parse() {
		let json = r#"{
            "jsonrpc": "2.0",
            "method": "tools/list",
            "params": {},
            "id": 1
        }"#;
		let req: JsonRpcRequest = serde_json::from_str(json).unwrap();
		assert_eq!(req.jsonrpc, "2.0");
		assert_eq!(req.method, "tools/list");
		assert!(!req.is_notification());
	}

	#[test]
	fn test_json_rpc_request_notification() {
		let json = r#"{
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        }"#;
		let req: JsonRpcRequest = serde_json::from_str(json).unwrap();
		assert!(req.is_notification());
	}

	#[test]
	fn test_json_rpc_response_success() {
		let response = JsonRpcResponse::success(
			Some(JsonRpcId::Number(1)),
			serde_json::json!({"status": "ok"}),
		);
		let json = serde_json::to_string(&response).unwrap();
		assert!(json.contains(r#""result":"#));
		assert!(!json.contains(r#""error""#));
	}

	#[test]
	fn test_json_rpc_response_error() {
		let response = JsonRpcResponse::error(
			Some(JsonRpcId::Number(1)),
			JsonRpcError {
				code: -32601,
				message: "Method not found".to_string(),
				data: None,
			},
		);
		let json = serde_json::to_string(&response).unwrap();
		assert!(json.contains(r#""error":"#));
		assert!(!json.contains(r#""result""#));
	}

	#[test]
	fn test_initialize_result_default() {
		let result = InitializeResult::default();
		assert_eq!(result.protocol_version, MCP_PROTOCOL_VERSION);
		assert!(result.capabilities.tools.is_some());
		assert_eq!(result.server_info.name, MCP_SERVER_NAME);
	}

	#[test]
	fn test_tools_call_result_text() {
		let result = ToolsCallResult::text("Hello, world!");
		assert!(result.is_error.is_none());
		assert_eq!(result.content.len(), 1);
	}

	#[test]
	fn test_tools_call_result_error() {
		let result = ToolsCallResult::error("Something went wrong");
		assert_eq!(result.is_error, Some(true));
	}

	#[test]
	fn test_create_weaver_args_parse() {
		let json = r#"{
            "image": "python:3.12",
            "org_id": "550e8400-e29b-41d4-a716-446655440000",
            "env": {"FOO": "bar"},
            "memory_limit": "8Gi"
        }"#;
		let args: CreateWeaverArgs = serde_json::from_str(json).unwrap();
		assert_eq!(args.image, "python:3.12");
		assert_eq!(args.env.get("FOO"), Some(&"bar".to_string()));
		assert_eq!(args.memory_limit, Some("8Gi".to_string()));
	}
}
