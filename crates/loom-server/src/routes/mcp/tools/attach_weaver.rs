// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! attach_weaver tool implementation.

use loom_server_audit::{AuditEventType, AuditLogBuilder, UserId as AuditUserId};
use loom_server_auth::CurrentUser;
use loom_server_weaver::WeaverId;
use serde_json::json;

use crate::api::AppState;
use crate::routes::mcp::error::McpError;
use crate::routes::mcp::types::{AttachWeaverArgs, Tool, ToolsCallResult};

use super::{can_read_weaver, is_weaver_owner_or_admin};

/// Get the attach_weaver tool definition.
pub fn definition() -> Tool {
	Tool {
		name: "attach_weaver".to_string(),
		description: "Get connection information for attaching to a weaver's terminal. \
			Returns the WebSocket URL and connection details."
			.to_string(),
		input_schema: json!({
			"type": "object",
			"properties": {
				"weaver_id": {
					"type": "string",
					"description": "The unique identifier of the weaver to attach to"
				}
			},
			"required": ["weaver_id"]
		}),
	}
}

/// Execute the attach_weaver tool.
pub async fn execute(
	state: &AppState,
	current_user: &CurrentUser,
	args: AttachWeaverArgs,
) -> Result<ToolsCallResult, McpError> {
	let provisioner = state.provisioner.as_ref().ok_or_else(|| {
		McpError::Internal("Weaver provisioner not configured on this server".to_string())
	})?;

	let weaver_id: WeaverId = args.weaver_id.parse().map_err(|_| {
		McpError::InvalidParams(format!("Invalid weaver_id format: {}", args.weaver_id))
	})?;

	let weaver = provisioner.get_weaver(&weaver_id).await?;

	// Check access
	if !can_read_weaver(current_user, &weaver) {
		return Err(McpError::Forbidden(
			"You don't have access to this weaver".to_string(),
		));
	}

	let read_only = !is_weaver_owner_or_admin(current_user, &weaver);

	// Build the WebSocket URL
	let ws_url = format!(
		"{}/api/weaver/{}/attach",
		state
			.base_url
			.replace("http://", "ws://")
			.replace("https://", "wss://"),
		args.weaver_id
	);

	// Log audit event
	state.audit_service.log(
		AuditLogBuilder::new(AuditEventType::WeaverAttached)
			.actor(AuditUserId::new(current_user.user.id.into_inner()))
			.resource("weaver", args.weaver_id.clone())
			.details(json!({
				"source": "mcp",
				"pod_name": &weaver.pod_name,
				"read_only": read_only,
			}))
			.build(),
	);

	let result_text = format!(
		"Weaver {} connection info:\n\n\
		Status: {:?}\n\
		WebSocket URL: {}\n\
		Access: {}\n\n\
		To attach, connect to the WebSocket URL with your authentication token.",
		args.weaver_id,
		weaver.status,
		ws_url,
		if read_only { "Read-only" } else { "Full access" }
	);

	Ok(ToolsCallResult::text(result_text))
}
