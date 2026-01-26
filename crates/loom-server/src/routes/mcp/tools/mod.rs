// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! MCP tool definitions and execution.
//!
//! This module provides the tools capability for the MCP server.
//! Each tool is implemented in its own submodule.

mod attach_weaver;
mod create_weaver;
mod delete_weaver;
mod get_weaver;
mod list_weavers;

use loom_server_auth::CurrentUser;
use loom_server_weaver::Weaver;

use crate::api::AppState;

use super::{
	error::McpError,
	types::{
		AttachWeaverArgs, CreateWeaverArgs, DeleteWeaverArgs, GetWeaverArgs, ListWeaversArgs,
		ToolsCallResult, ToolsListResult,
	},
};

/// Get the list of available tools.
pub fn list_tools() -> ToolsListResult {
	ToolsListResult {
		tools: vec![
			create_weaver::definition(),
			list_weavers::definition(),
			get_weaver::definition(),
			delete_weaver::definition(),
			attach_weaver::definition(),
		],
	}
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

			create_weaver::execute(state, current_user, args).await
		}
		"list_weavers" => {
			let args: ListWeaversArgs = arguments
				.map(serde_json::from_value)
				.transpose()
				.map_err(|e| McpError::InvalidParams(format!("Invalid list_weavers arguments: {e}")))?
				.unwrap_or_default();

			list_weavers::execute(state, current_user, args).await
		}
		"get_weaver" => {
			let args: GetWeaverArgs = arguments
				.map(serde_json::from_value)
				.transpose()
				.map_err(|e| McpError::InvalidParams(format!("Invalid get_weaver arguments: {e}")))?
				.ok_or_else(|| McpError::InvalidParams("get_weaver requires arguments".to_string()))?;

			get_weaver::execute(state, current_user, args).await
		}
		"delete_weaver" => {
			let args: DeleteWeaverArgs = arguments
				.map(serde_json::from_value)
				.transpose()
				.map_err(|e| McpError::InvalidParams(format!("Invalid delete_weaver arguments: {e}")))?
				.ok_or_else(|| McpError::InvalidParams("delete_weaver requires arguments".to_string()))?;

			delete_weaver::execute(state, current_user, args).await
		}
		"attach_weaver" => {
			let args: AttachWeaverArgs = arguments
				.map(serde_json::from_value)
				.transpose()
				.map_err(|e| McpError::InvalidParams(format!("Invalid attach_weaver arguments: {e}")))?
				.ok_or_else(|| McpError::InvalidParams("attach_weaver requires arguments".to_string()))?;

			attach_weaver::execute(state, current_user, args).await
		}
		_ => Err(McpError::ToolNotFound(name.to_string())),
	}
}

// Shared helper functions used by multiple tools

/// Format a weaver for display.
pub(crate) fn format_weaver(weaver: &Weaver) -> String {
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

/// Check if a user can read a weaver.
pub(crate) fn can_read_weaver(current_user: &CurrentUser, weaver: &Weaver) -> bool {
	current_user.user.is_system_admin()
		|| current_user.user.is_support()
		|| weaver.owner_user_id == current_user.user.id.to_string()
}

/// Check if a user is the owner or admin of a weaver.
pub(crate) fn is_weaver_owner_or_admin(current_user: &CurrentUser, weaver: &Weaver) -> bool {
	current_user.user.is_system_admin()
		|| weaver.owner_user_id == current_user.user.id.to_string()
}

#[cfg(test)]
mod tests {
	use super::*;
	use serde_json::json;

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
		let tool = create_weaver::definition();
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
