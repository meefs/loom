// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! delete_weaver tool implementation.

use loom_server_audit::{AuditEventType, AuditLogBuilder, UserId as AuditUserId};
use loom_server_auth::CurrentUser;
use loom_server_weaver::WeaverId;
use serde_json::json;

use crate::api::AppState;
use crate::routes::mcp::error::McpError;
use crate::routes::mcp::types::{DeleteWeaverArgs, Tool, ToolsCallResult};

use super::is_weaver_owner_or_admin;

/// Get the delete_weaver tool definition.
pub fn definition() -> Tool {
	Tool {
		name: "delete_weaver".to_string(),
		description: "Delete a weaver and terminate its Kubernetes pod. \
			Only the owner or system admins can delete a weaver."
			.to_string(),
		input_schema: json!({
			"type": "object",
			"properties": {
				"weaver_id": {
					"type": "string",
					"description": "The unique identifier of the weaver to delete"
				}
			},
			"required": ["weaver_id"]
		}),
	}
}

/// Execute the delete_weaver tool.
pub async fn execute(
	state: &AppState,
	current_user: &CurrentUser,
	args: DeleteWeaverArgs,
) -> Result<ToolsCallResult, McpError> {
	let provisioner = state.provisioner.as_ref().ok_or_else(|| {
		McpError::Internal("Weaver provisioner not configured on this server".to_string())
	})?;

	let weaver_id: WeaverId = args.weaver_id.parse().map_err(|_| {
		McpError::InvalidParams(format!("Invalid weaver_id format: {}", args.weaver_id))
	})?;

	let weaver = provisioner.get_weaver(&weaver_id).await?;

	// Check delete access (owner or admin only)
	if !is_weaver_owner_or_admin(current_user, &weaver) {
		return Err(McpError::Forbidden(
			"Only the owner or system admin can delete this weaver".to_string(),
		));
	}

	let actor_id = current_user.user.id.to_string();
	tracing::info!(
		weaver_id = %args.weaver_id,
		actor_id = %actor_id,
		source = "mcp",
		"Deleting weaver via MCP"
	);

	provisioner.delete_weaver(&weaver_id).await?;

	// Log audit event
	state.audit_service.log(
		AuditLogBuilder::new(AuditEventType::WeaverDeleted)
			.actor(AuditUserId::new(current_user.user.id.into_inner()))
			.resource("weaver", args.weaver_id.clone())
			.details(json!({
				"source": "mcp",
				"pod_name": &weaver.pod_name,
			}))
			.build(),
	);

	Ok(ToolsCallResult::text(format!(
		"Deleted weaver {}\n\nPod {} has been terminated.",
		args.weaver_id, weaver.pod_name
	)))
}
