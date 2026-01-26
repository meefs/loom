// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! MCP endpoint handler.

use axum::{
	extract::State,
	http::{HeaderMap, HeaderValue},
	response::{IntoResponse, Response},
	Json,
};
use serde_json::json;

use crate::{api::AppState, auth_middleware::RequireAuth};

use super::{
	error::{McpError, McpErrorResponse},
	prompts, resources, tools,
	types::{
		InitializeParams, InitializeResult, JsonRpcRequest, JsonRpcResponse, PromptsGetParams,
		ResourcesReadParams, ToolsCallParams, ToolsListResult, JSONRPC_VERSION,
	},
};

/// Header name for MCP session ID.
pub const MCP_SESSION_HEADER: &str = "Mcp-Session-Id";

/// Main MCP endpoint handler.
///
/// Handles JSON-RPC 2.0 requests for the Model Context Protocol.
#[axum::debug_handler]
pub async fn mcp_handler(
	State(state): State<AppState>,
	RequireAuth(current_user): RequireAuth,
	headers: HeaderMap,
	Json(request): Json<JsonRpcRequest>,
) -> Response {
	// Validate JSON-RPC version
	if request.jsonrpc != JSONRPC_VERSION {
		return McpErrorResponse::new(
			McpError::InvalidRequest(format!(
				"Invalid jsonrpc version: expected '{}', got '{}'",
				JSONRPC_VERSION, request.jsonrpc
			)),
			request.id,
		)
		.into_response();
	}

	// Get or create session if header is present
	let session_id = headers
		.get(MCP_SESSION_HEADER)
		.and_then(|v| v.to_str().ok())
		.map(String::from);

	// Touch session if it exists
	if let Some(ref sid) = session_id {
		if let Some(ref store) = state.mcp_sessions {
			store.touch_session(sid);
		}
	}

	// Route the request to the appropriate handler
	let result = route_method(&state, &current_user, &request, session_id.as_deref()).await;

	match result {
		Ok((response, new_session_id)) => {
			let mut http_response = Json(response).into_response();

			// Add session header if a new session was created
			if let Some(sid) = new_session_id {
				if let Ok(value) = HeaderValue::from_str(&sid) {
					http_response
						.headers_mut()
						.insert(MCP_SESSION_HEADER, value);
				}
			}

			http_response
		}
		Err(err) => McpErrorResponse::new(err, request.id).into_response(),
	}
}

/// Route a JSON-RPC method to its handler.
async fn route_method(
	state: &AppState,
	current_user: &loom_server_auth::CurrentUser,
	request: &JsonRpcRequest,
	session_id: Option<&str>,
) -> Result<(JsonRpcResponse, Option<String>), McpError> {
	match request.method.as_str() {
		"initialize" => handle_initialize(state, current_user, request, session_id).await,
		"notifications/initialized" => handle_initialized_notification(request),
		"tools/list" => handle_tools_list(request),
		"tools/call" => handle_tools_call(state, current_user, request).await,
		"resources/list" => handle_resources_list(state, current_user, request).await,
		"resources/read" => handle_resources_read(state, current_user, request).await,
		"resources/templates/list" => handle_resource_templates_list(request),
		"prompts/list" => handle_prompts_list(request),
		"prompts/get" => handle_prompts_get(request),
		"ping" => handle_ping(request),
		_ => Err(McpError::MethodNotFound(request.method.clone())),
	}
}

/// Handle the initialize method.
async fn handle_initialize(
	state: &AppState,
	current_user: &loom_server_auth::CurrentUser,
	request: &JsonRpcRequest,
	_existing_session_id: Option<&str>,
) -> Result<(JsonRpcResponse, Option<String>), McpError> {
	// Parse initialize params
	let params: InitializeParams = request
		.params
		.as_ref()
		.map(|v| serde_json::from_value(v.clone()))
		.transpose()
		.map_err(|e| McpError::InvalidParams(format!("Invalid initialize params: {e}")))?
		.ok_or_else(|| McpError::InvalidParams("initialize requires params".to_string()))?;

	tracing::info!(
		protocol_version = %params.protocol_version,
		client_name = %params.client_info.name,
		client_version = ?params.client_info.version,
		user_id = %current_user.user.id,
		"MCP initialize request"
	);

	// Create a new session
	let new_session_id = if let Some(ref store) = state.mcp_sessions {
		let mut session = store.create_session(current_user.user.id.to_string());
		session.mark_initialized(
			params.protocol_version.clone(),
			params.client_info.name.clone(),
			params.client_info.version.clone(),
		);
		let sid = session.id.clone();
		store.update_session(session);
		Some(sid)
	} else {
		None
	};

	let result = InitializeResult::default();
	let response = JsonRpcResponse::success(request.id.clone(), serde_json::to_value(result)?);

	Ok((response, new_session_id))
}

/// Handle the notifications/initialized notification.
fn handle_initialized_notification(
	request: &JsonRpcRequest,
) -> Result<(JsonRpcResponse, Option<String>), McpError> {
	// This is a notification (no response expected per JSON-RPC spec)
	// But we return an empty success for simplicity since we're using POST
	if request.is_notification() {
		// For true notifications, we could return 204 No Content
		// But for simplicity, return a minimal success
		let response = JsonRpcResponse {
			jsonrpc: JSONRPC_VERSION.to_string(),
			result: Some(json!({})),
			error: None,
			id: None,
		};
		Ok((response, None))
	} else {
		// If they sent an id, respond with empty result
		let response = JsonRpcResponse::success(request.id.clone(), json!({}));
		Ok((response, None))
	}
}

/// Handle the tools/list method.
fn handle_tools_list(request: &JsonRpcRequest) -> Result<(JsonRpcResponse, Option<String>), McpError> {
	let result: ToolsListResult = tools::list_tools();
	let response = JsonRpcResponse::success(request.id.clone(), serde_json::to_value(result)?);
	Ok((response, None))
}

/// Handle the tools/call method.
async fn handle_tools_call(
	state: &AppState,
	current_user: &loom_server_auth::CurrentUser,
	request: &JsonRpcRequest,
) -> Result<(JsonRpcResponse, Option<String>), McpError> {
	// Parse tools/call params
	let params: ToolsCallParams = request
		.params
		.as_ref()
		.map(|v| serde_json::from_value(v.clone()))
		.transpose()
		.map_err(|e| McpError::InvalidParams(format!("Invalid tools/call params: {e}")))?
		.ok_or_else(|| McpError::InvalidParams("tools/call requires params".to_string()))?;

	tracing::info!(
		tool = %params.name,
		user_id = %current_user.user.id,
		"MCP tools/call request"
	);

	// Execute the tool
	let result = tools::execute_tool(state, current_user, &params.name, params.arguments).await;

	match result {
		Ok(call_result) => {
			let response =
				JsonRpcResponse::success(request.id.clone(), serde_json::to_value(call_result)?);
			Ok((response, None))
		}
		Err(e) => {
			// For tool execution errors, we return a success response with isError: true
			// This follows MCP spec for tool errors vs protocol errors
			let error_result = super::types::ToolsCallResult::error(e.to_string());
			let response =
				JsonRpcResponse::success(request.id.clone(), serde_json::to_value(error_result)?);
			Ok((response, None))
		}
	}
}

/// Handle the ping method (for connection testing).
fn handle_ping(request: &JsonRpcRequest) -> Result<(JsonRpcResponse, Option<String>), McpError> {
	let response = JsonRpcResponse::success(request.id.clone(), json!({}));
	Ok((response, None))
}

/// Handle the resources/list method.
async fn handle_resources_list(
	state: &AppState,
	current_user: &loom_server_auth::CurrentUser,
	request: &JsonRpcRequest,
) -> Result<(JsonRpcResponse, Option<String>), McpError> {
	tracing::info!(
		user_id = %current_user.user.id,
		"MCP resources/list request"
	);

	let result = resources::list_resources(state, current_user).await?;
	let response = JsonRpcResponse::success(request.id.clone(), serde_json::to_value(result)?);
	Ok((response, None))
}

/// Handle the resources/read method.
async fn handle_resources_read(
	state: &AppState,
	current_user: &loom_server_auth::CurrentUser,
	request: &JsonRpcRequest,
) -> Result<(JsonRpcResponse, Option<String>), McpError> {
	let params: ResourcesReadParams = request
		.params
		.as_ref()
		.map(|v| serde_json::from_value(v.clone()))
		.transpose()
		.map_err(|e| McpError::InvalidParams(format!("Invalid resources/read params: {e}")))?
		.ok_or_else(|| McpError::InvalidParams("resources/read requires params".to_string()))?;

	tracing::info!(
		uri = %params.uri,
		user_id = %current_user.user.id,
		"MCP resources/read request"
	);

	let result = resources::read_resource(state, current_user, &params.uri).await?;
	let response = JsonRpcResponse::success(request.id.clone(), serde_json::to_value(result)?);
	Ok((response, None))
}

/// Handle the resources/templates/list method.
fn handle_resource_templates_list(
	request: &JsonRpcRequest,
) -> Result<(JsonRpcResponse, Option<String>), McpError> {
	let result = resources::list_resource_templates();
	let response = JsonRpcResponse::success(request.id.clone(), serde_json::to_value(result)?);
	Ok((response, None))
}

/// Handle the prompts/list method.
fn handle_prompts_list(
	request: &JsonRpcRequest,
) -> Result<(JsonRpcResponse, Option<String>), McpError> {
	let result = prompts::list_prompts();
	let response = JsonRpcResponse::success(request.id.clone(), serde_json::to_value(result)?);
	Ok((response, None))
}

/// Handle the prompts/get method.
fn handle_prompts_get(
	request: &JsonRpcRequest,
) -> Result<(JsonRpcResponse, Option<String>), McpError> {
	let params: PromptsGetParams = request
		.params
		.as_ref()
		.map(|v| serde_json::from_value(v.clone()))
		.transpose()
		.map_err(|e| McpError::InvalidParams(format!("Invalid prompts/get params: {e}")))?
		.ok_or_else(|| McpError::InvalidParams("prompts/get requires params".to_string()))?;

	tracing::info!(
		prompt_name = %params.name,
		"MCP prompts/get request"
	);

	let result = prompts::get_prompt(&params.name, &params.arguments)?;
	let response = JsonRpcResponse::success(request.id.clone(), serde_json::to_value(result)?);
	Ok((response, None))
}

impl From<serde_json::Error> for McpError {
	fn from(err: serde_json::Error) -> Self {
		McpError::Internal(format!("JSON serialization error: {err}"))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::routes::mcp::types::JsonRpcId;

	#[test]
	fn test_jsonrpc_version_validation() {
		let request = JsonRpcRequest {
			jsonrpc: "1.0".to_string(),
			method: "test".to_string(),
			params: None,
			id: Some(JsonRpcId::Number(1)),
		};
		assert_ne!(request.jsonrpc, JSONRPC_VERSION);
	}

	#[test]
	fn test_notification_detection() {
		let notification = JsonRpcRequest {
			jsonrpc: JSONRPC_VERSION.to_string(),
			method: "notifications/initialized".to_string(),
			params: None,
			id: None,
		};
		assert!(notification.is_notification());

		let request = JsonRpcRequest {
			jsonrpc: JSONRPC_VERSION.to_string(),
			method: "tools/list".to_string(),
			params: None,
			id: Some(JsonRpcId::Number(1)),
		};
		assert!(!request.is_notification());
	}
}
