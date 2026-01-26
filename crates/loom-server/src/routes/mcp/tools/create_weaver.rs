// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! create_weaver tool implementation.

use loom_server_audit::{AuditEventType, AuditLogBuilder, UserId as AuditUserId};
use loom_server_auth::{CurrentUser, OrgId};
use loom_server_weaver::{CreateWeaverRequest, ResourceSpec};
use serde_json::json;
use uuid::Uuid;

use crate::api::AppState;
use crate::routes::mcp::error::McpError;
use crate::routes::mcp::types::{CreateWeaverArgs, Tool, ToolsCallResult};

/// Get the create_weaver tool definition.
pub fn definition() -> Tool {
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

/// Execute the create_weaver tool.
pub async fn execute(
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

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_create_weaver_tool_schema() {
		let tool = definition();
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
