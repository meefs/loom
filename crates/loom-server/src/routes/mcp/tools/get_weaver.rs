// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! get_weaver tool implementation.

use loom_server_auth::CurrentUser;
use loom_server_weaver::WeaverId;
use serde_json::json;

use crate::api::AppState;
use crate::routes::mcp::error::McpError;
use crate::routes::mcp::types::{GetWeaverArgs, Tool, ToolsCallResult};

use super::{can_read_weaver, format_weaver};

/// Get the get_weaver tool definition.
pub fn definition() -> Tool {
	Tool {
		name: "get_weaver".to_string(),
		description: "Get detailed information about a specific weaver including status, \
			resource usage, and connection details."
			.to_string(),
		input_schema: json!({
			"type": "object",
			"properties": {
				"weaver_id": {
					"type": "string",
					"description": "The unique identifier of the weaver"
				}
			},
			"required": ["weaver_id"]
		}),
	}
}

/// Execute the get_weaver tool.
pub async fn execute(
	state: &AppState,
	current_user: &CurrentUser,
	args: GetWeaverArgs,
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

	Ok(ToolsCallResult::text(format_weaver(&weaver)))
}
