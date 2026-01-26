// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! MCP tool definitions and execution.

use loom_server_audit::{AuditEventType, AuditLogBuilder, UserId as AuditUserId};
use loom_server_auth::{CurrentUser, OrgId};
use loom_server_weaver::{CreateWeaverRequest, ResourceSpec, Weaver, WeaverId};
use serde_json::json;
use uuid::Uuid;

use crate::api::AppState;

use super::{
	error::McpError,
	types::{
		AttachWeaverArgs, CreateWeaverArgs, DeleteWeaverArgs, GetWeaverArgs, ListWeaversArgs,
		Tool, ToolsCallResult, ToolsListResult,
	},
};

/// Get the list of available tools.
pub fn list_tools() -> ToolsListResult {
	ToolsListResult {
		tools: vec![
			create_weaver_tool(),
			list_weavers_tool(),
			get_weaver_tool(),
			delete_weaver_tool(),
			attach_weaver_tool(),
		],
	}
}

/// Get the create_weaver tool definition.
fn create_weaver_tool() -> Tool {
	Tool {
		name: "create_weaver".to_string(),
		description: "Create an ephemeral Kubernetes pod for code execution. \
			The pod will be automatically cleaned up after the specified lifetime."
			.to_string(),
		input_schema: json!({
			"type": "object",
			"properties": {
				"image": {
					"type": "string",
					"description": "Container image to run (e.g., 'python:3.12', 'node:20', 'ubuntu:22.04')"
				},
				"org_id": {
					"type": "string",
					"description": "Organization ID (UUID) that will own this weaver for billing and isolation"
				},
				"env": {
					"type": "object",
					"additionalProperties": { "type": "string" },
					"description": "Environment variables to set in the container"
				},
				"memory_limit": {
					"type": "string",
					"description": "Memory limit for the container (e.g., '512Mi', '2Gi', '8Gi')"
				},
				"cpu_limit": {
					"type": "string",
					"description": "CPU limit for the container (e.g., '0.5', '1', '4')"
				},
				"lifetime_hours": {
					"type": "integer",
					"description": "How long the weaver should run before automatic cleanup (max 48 hours)"
				},
				"tags": {
					"type": "object",
					"additionalProperties": { "type": "string" },
					"description": "Custom tags/labels for organizing and filtering weavers"
				}
			},
			"required": ["image", "org_id"]
		}),
	}
}

/// Get the list_weavers tool definition.
fn list_weavers_tool() -> Tool {
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

/// Get the get_weaver tool definition.
fn get_weaver_tool() -> Tool {
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

/// Get the delete_weaver tool definition.
fn delete_weaver_tool() -> Tool {
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

/// Get the attach_weaver tool definition.
fn attach_weaver_tool() -> Tool {
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

/// Execute the create_weaver tool.
pub async fn execute_create_weaver(
	state: &AppState,
	current_user: &CurrentUser,
	args: CreateWeaverArgs,
) -> Result<ToolsCallResult, McpError> {
	// Get the provisioner
	let provisioner = state.provisioner.as_ref().ok_or_else(|| {
		McpError::Internal("Weaver provisioner not configured on this server".to_string())
	})?;

	// Parse and validate org_id
	let org_uuid = Uuid::parse_str(&args.org_id)
		.map_err(|_| McpError::InvalidParams(format!("Invalid org_id format: {}", args.org_id)))?;
	let org_id = OrgId::new(org_uuid);

	// Check organization membership
	if !current_user.user.is_system_admin() {
		match state
			.org_repo
			.get_membership(&org_id, &current_user.user.id)
			.await
		{
			Ok(Some(_)) => {}
			Ok(None) => {
				return Err(McpError::Forbidden(format!(
					"Not a member of organization {}",
					args.org_id
				)));
			}
			Err(e) => {
				tracing::error!(error = %e, org_id = %args.org_id, "Failed to check org membership");
				return Err(McpError::Internal(
					"Failed to verify organization membership".to_string(),
				));
			}
		}
	}

	let actor_id = current_user.user.id.to_string();
	let image_for_audit = args.image.clone();
	let org_id_for_audit = args.org_id.clone();

	tracing::info!(
		image = %args.image,
		org_id = %args.org_id,
		actor_id = %actor_id,
		source = "mcp",
		"Creating weaver via MCP"
	);

	// Build the create request
	let create_request = CreateWeaverRequest {
		image: args.image,
		env: args.env,
		resources: ResourceSpec {
			memory_limit: args.memory_limit,
			cpu_limit: args.cpu_limit,
		},
		tags: args.tags,
		lifetime_hours: args.lifetime_hours,
		command: None,
		args: None,
		workdir: None,
		repo: None,
		branch: None,
		owner_user_id: Some(actor_id.clone()),
		org_id: args.org_id,
		repo_id: None,
	};

	// Create the weaver
	let weaver = provisioner.create_weaver(create_request).await?;

	// Log audit event
	state.audit_service.log(
		AuditLogBuilder::new(AuditEventType::WeaverCreated)
			.actor(AuditUserId::new(current_user.user.id.into_inner()))
			.resource("weaver", weaver.id.to_string())
			.details(json!({
				"source": "mcp",
				"image": &image_for_audit,
				"org_id": &org_id_for_audit,
				"pod_name": &weaver.pod_name,
			}))
			.build(),
	);

	tracing::info!(
		weaver_id = %weaver.id,
		pod_name = %weaver.pod_name,
		actor_id = %actor_id,
		source = "mcp",
		"Weaver created via MCP"
	);

	// Build success response
	let lifetime_display = weaver.lifetime_hours;
	let result_text = format!(
		"Created weaver {}\n\n\
		Image: {}\n\
		Status: {:?}\n\
		Pod: {}\n\
		Lifetime: {} hours",
		weaver.id, weaver.image, weaver.status, weaver.pod_name, lifetime_display
	);

	Ok(ToolsCallResult::text(result_text))
}

/// Format a weaver for display.
fn format_weaver(weaver: &Weaver) -> String {
	format!(
		"ID: {}\n\
		Image: {}\n\
		Status: {:?}\n\
		Pod: {}\n\
		Age: {:.1} hours\n\
		Lifetime: {} hours\n\
		Tags: {}",
		weaver.id,
		weaver.image,
		weaver.status,
		weaver.pod_name,
		weaver.age_hours,
		weaver.lifetime_hours,
		if weaver.tags.is_empty() {
			"(none)".to_string()
		} else {
			weaver
				.tags
				.iter()
				.map(|(k, v)| format!("{}={}", k, v))
				.collect::<Vec<_>>()
				.join(", ")
		}
	)
}

/// Execute the list_weavers tool.
pub async fn execute_list_weavers(
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

/// Execute the get_weaver tool.
pub async fn execute_get_weaver(
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

/// Execute the delete_weaver tool.
pub async fn execute_delete_weaver(
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

/// Execute the attach_weaver tool.
pub async fn execute_attach_weaver(
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
		state.base_url.replace("http://", "ws://").replace("https://", "wss://"),
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

/// Check if a user can read a weaver.
fn can_read_weaver(current_user: &CurrentUser, weaver: &Weaver) -> bool {
	current_user.user.is_system_admin()
		|| current_user.user.is_support()
		|| weaver.owner_user_id == current_user.user.id.to_string()
}

/// Check if a user is the owner or admin of a weaver.
fn is_weaver_owner_or_admin(current_user: &CurrentUser, weaver: &Weaver) -> bool {
	current_user.user.is_system_admin()
		|| weaver.owner_user_id == current_user.user.id.to_string()
}

/// Execute a tool by name.
pub async fn execute_tool(
	state: &AppState,
	current_user: &CurrentUser,
	name: &str,
	arguments: Option<serde_json::Value>,
) -> Result<ToolsCallResult, McpError> {
	match name {
		"create_weaver" => {
			let args: CreateWeaverArgs = arguments
				.map(serde_json::from_value)
				.transpose()
				.map_err(|e| McpError::InvalidParams(format!("Invalid create_weaver arguments: {e}")))?
				.ok_or_else(|| McpError::InvalidParams("create_weaver requires arguments".to_string()))?;

			execute_create_weaver(state, current_user, args).await
		}
		"list_weavers" => {
			let args: ListWeaversArgs = arguments
				.map(serde_json::from_value)
				.transpose()
				.map_err(|e| McpError::InvalidParams(format!("Invalid list_weavers arguments: {e}")))?
				.unwrap_or_default();

			execute_list_weavers(state, current_user, args).await
		}
		"get_weaver" => {
			let args: GetWeaverArgs = arguments
				.map(serde_json::from_value)
				.transpose()
				.map_err(|e| McpError::InvalidParams(format!("Invalid get_weaver arguments: {e}")))?
				.ok_or_else(|| McpError::InvalidParams("get_weaver requires arguments".to_string()))?;

			execute_get_weaver(state, current_user, args).await
		}
		"delete_weaver" => {
			let args: DeleteWeaverArgs = arguments
				.map(serde_json::from_value)
				.transpose()
				.map_err(|e| McpError::InvalidParams(format!("Invalid delete_weaver arguments: {e}")))?
				.ok_or_else(|| McpError::InvalidParams("delete_weaver requires arguments".to_string()))?;

			execute_delete_weaver(state, current_user, args).await
		}
		"attach_weaver" => {
			let args: AttachWeaverArgs = arguments
				.map(serde_json::from_value)
				.transpose()
				.map_err(|e| McpError::InvalidParams(format!("Invalid attach_weaver arguments: {e}")))?
				.ok_or_else(|| McpError::InvalidParams("attach_weaver requires arguments".to_string()))?;

			execute_attach_weaver(state, current_user, args).await
		}
		_ => Err(McpError::ToolNotFound(name.to_string())),
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_list_tools() {
		let result = list_tools();
		assert_eq!(result.tools.len(), 5);
		let tool_names: Vec<&str> = result.tools.iter().map(|t| t.name.as_str()).collect();
		assert!(tool_names.contains(&"create_weaver"));
		assert!(tool_names.contains(&"list_weavers"));
		assert!(tool_names.contains(&"get_weaver"));
		assert!(tool_names.contains(&"delete_weaver"));
		assert!(tool_names.contains(&"attach_weaver"));
	}

	#[test]
	fn test_create_weaver_tool_schema() {
		let tool = create_weaver_tool();
		let schema = &tool.input_schema;

		// Check required fields
		let required = schema.get("required").unwrap().as_array().unwrap();
		assert!(required.contains(&json!("image")));
		assert!(required.contains(&json!("org_id")));

		// Check properties exist
		let props = schema.get("properties").unwrap().as_object().unwrap();
		assert!(props.contains_key("image"));
		assert!(props.contains_key("org_id"));
		assert!(props.contains_key("env"));
		assert!(props.contains_key("memory_limit"));
		assert!(props.contains_key("cpu_limit"));
		assert!(props.contains_key("lifetime_hours"));
		assert!(props.contains_key("tags"));
	}
}
