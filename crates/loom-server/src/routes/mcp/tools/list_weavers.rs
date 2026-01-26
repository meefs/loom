// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! list_weavers tool implementation.

use loom_server_auth::CurrentUser;
use loom_server_weaver::Weaver;
use serde_json::json;

use crate::api::AppState;
use crate::routes::mcp::error::McpError;
use crate::routes::mcp::types::{ListWeaversArgs, Tool, ToolsCallResult};

/// Get the list_weavers tool definition.
pub fn definition() -> Tool {
	Tool {
		name: "list_weavers".to_string(),
		description: "List weavers (ephemeral Kubernetes pods) owned by the current user. \
			System admins can see all weavers."
			.to_string(),
		input_schema: json!({
			"type": "object",
			"properties": {
				"org_id": {
					"type": "string",
					"description": "Filter by organization ID (optional)"
				},
				"status": {
					"type": "string",
					"enum": ["pending", "running", "succeeded", "failed", "terminating"],
					"description": "Filter by weaver status (optional)"
				},
				"tags": {
					"type": "object",
					"additionalProperties": { "type": "string" },
					"description": "Filter by tags (optional)"
				}
			}
		}),
	}
}

/// Execute the list_weavers tool.
pub async fn execute(
	state: &AppState,
	current_user: &CurrentUser,
	args: ListWeaversArgs,
) -> Result<ToolsCallResult, McpError> {
	let provisioner = state.provisioner.as_ref().ok_or_else(|| {
		McpError::Internal("Weaver provisioner not configured on this server".to_string())
	})?;

	// Build tag filter
	let tag_filter = if args.tags.is_empty() {
		None
	} else {
		Some(args.tags.clone())
	};

	// Get weavers based on user role
	let weavers = if current_user.user.is_system_admin() || current_user.user.is_support() {
		provisioner.list_weavers(tag_filter).await?
	} else {
		let user_id = current_user.user.id.to_string();
		let all_weavers = provisioner.list_weavers_for_user(&user_id).await?;
		if let Some(ref tags) = tag_filter {
			all_weavers
				.into_iter()
				.filter(|w| tags.iter().all(|(k, v)| w.tags.get(k) == Some(v)))
				.collect()
		} else {
			all_weavers
		}
	};

	// Apply status filter
	let weavers: Vec<Weaver> = if let Some(ref status) = args.status {
		weavers
			.into_iter()
			.filter(|w| format!("{:?}", w.status).to_lowercase() == status.to_lowercase())
			.collect()
	} else {
		weavers
	};

	if weavers.is_empty() {
		return Ok(ToolsCallResult::text("No weavers found."));
	}

	let result_text = format!(
		"Found {} weaver(s):\n\n{}",
		weavers.len(),
		weavers
			.iter()
			.map(|w| format!(
				"- {} ({:?}) - {} [{:.1}h old]",
				w.id, w.status, w.image, w.age_hours
			))
			.collect::<Vec<_>>()
			.join("\n")
	);

	Ok(ToolsCallResult::text(result_text))
}
