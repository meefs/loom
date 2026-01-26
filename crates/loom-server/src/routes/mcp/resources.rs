// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! MCP Resources implementation.
//!
//! Exposes loom data as MCP resources that can be read by clients.

use loom_server_auth::CurrentUser;
use loom_server_scm::{OwnerType, RepoStore};
use serde_json::json;

use crate::api::AppState;

use super::{
	error::McpError,
	types::{
		Resource, ResourceContent, ResourceTemplate, ResourceTemplatesListResult,
		ResourcesListResult, ResourcesReadResult,
	},
};

/// Get the list of available resources for the current user.
pub async fn list_resources(
	state: &AppState,
	current_user: &CurrentUser,
) -> Result<ResourcesListResult, McpError> {
	let mut resources = Vec::new();

	// Add thread listing resource
	resources.push(Resource {
		uri: "loom://threads".to_string(),
		name: "Threads".to_string(),
		description: Some("List of your conversation threads".to_string()),
		mime_type: Some("application/json".to_string()),
	});

	// Add repos listing resource
	resources.push(Resource {
		uri: "loom://repos".to_string(),
		name: "Repositories".to_string(),
		description: Some("List of your repositories".to_string()),
		mime_type: Some("application/json".to_string()),
	});

	// Add weavers listing resource
	resources.push(Resource {
		uri: "loom://weavers".to_string(),
		name: "Weavers".to_string(),
		description: Some("List of your active weavers (execution environments)".to_string()),
		mime_type: Some("application/json".to_string()),
	});

	// Add individual thread resources based on user's threads
	let owner_user_id = current_user.user.id.to_string();
	if let Ok(threads) = state
		.repo
		.list_for_owner(&owner_user_id, None, 50, 0)
		.await
	{
		for thread in threads.iter() {
			resources.push(Resource {
				uri: format!("loom://threads/{}", thread.id),
				name: thread
					.title
					.clone()
					.unwrap_or_else(|| format!("Thread {}", thread.id)),
				description: Some("Conversation thread".to_string()),
				mime_type: Some("application/json".to_string()),
			});
		}
	}

	Ok(ResourcesListResult { resources })
}

/// Get the list of resource templates.
pub fn list_resource_templates() -> ResourceTemplatesListResult {
	ResourceTemplatesListResult {
		resource_templates: vec![
			ResourceTemplate {
				uri_template: "loom://threads/{thread_id}".to_string(),
				name: "Thread".to_string(),
				description: Some("A conversation thread by ID".to_string()),
				mime_type: Some("application/json".to_string()),
			},
			ResourceTemplate {
				uri_template: "loom://threads/{thread_id}/messages".to_string(),
				name: "Thread Messages".to_string(),
				description: Some("Messages in a conversation thread".to_string()),
				mime_type: Some("application/json".to_string()),
			},
			ResourceTemplate {
				uri_template: "loom://repos/{repo_id}".to_string(),
				name: "Repository".to_string(),
				description: Some("A repository by ID".to_string()),
				mime_type: Some("application/json".to_string()),
			},
			ResourceTemplate {
				uri_template: "loom://weavers/{weaver_id}".to_string(),
				name: "Weaver".to_string(),
				description: Some("An active weaver by ID".to_string()),
				mime_type: Some("application/json".to_string()),
			},
		],
	}
}

/// Read a resource by URI.
pub async fn read_resource(
	state: &AppState,
	current_user: &CurrentUser,
	uri: &str,
) -> Result<ResourcesReadResult, McpError> {
	// Parse the loom:// URI
	let path = uri
		.strip_prefix("loom://")
		.ok_or_else(|| McpError::InvalidParams(format!("Invalid resource URI: {uri}")))?;

	let parts: Vec<&str> = path.split('/').collect();

	match parts.as_slice() {
		["threads"] => read_threads_list(state, current_user).await,
		["threads", thread_id] => read_thread(state, current_user, thread_id).await,
		["threads", thread_id, "messages"] => {
			read_thread_messages(state, current_user, thread_id).await
		}
		["repos"] => read_repos_list(state, current_user).await,
		["repos", repo_id] => read_repo(state, current_user, repo_id).await,
		["weavers"] => read_weavers_list(state, current_user).await,
		["weavers", weaver_id] => read_weaver(state, current_user, weaver_id).await,
		_ => Err(McpError::InvalidParams(format!(
			"Unknown resource path: {path}"
		))),
	}
}

/// Read the list of threads.
async fn read_threads_list(
	state: &AppState,
	current_user: &CurrentUser,
) -> Result<ResourcesReadResult, McpError> {
	let owner_user_id = current_user.user.id.to_string();
	let threads = state
		.repo
		.list_for_owner(&owner_user_id, None, 100, 0)
		.await
		.map_err(|e| McpError::Internal(format!("Failed to list threads: {e}")))?;

	let thread_list: Vec<_> = threads
		.iter()
		.map(|t| {
			json!({
				"id": t.id.to_string(),
				"title": t.title,
				"created_at": t.created_at,
				"updated_at": t.updated_at,
				"message_count": t.message_count,
				"workspace_root": t.workspace_root,
			})
		})
		.collect();

	let content = json!({
		"threads": thread_list,
		"count": threads.len(),
	});

	Ok(ResourcesReadResult {
		contents: vec![ResourceContent::Text {
			text: serde_json::to_string_pretty(&content)
				.map_err(|e| McpError::Internal(format!("JSON serialization error: {e}")))?,
			mime_type: Some("application/json".to_string()),
		}],
	})
}

/// Read a specific thread.
async fn read_thread(
	state: &AppState,
	current_user: &CurrentUser,
	thread_id: &str,
) -> Result<ResourcesReadResult, McpError> {
	let thread_id_parsed = thread_id.parse().map_err(|_| {
		McpError::InvalidParams(format!("Invalid thread_id format: {thread_id}"))
	})?;

	let thread = state
		.repo
		.get(&thread_id_parsed)
		.await
		.map_err(|e| McpError::Internal(format!("Failed to get thread: {e}")))?
		.ok_or_else(|| McpError::InvalidParams(format!("Thread not found: {thread_id}")))?;

	// Check ownership via the owner_user_id stored in thread metadata
	let owner_user_id = state
		.repo
		.get_thread_owner_user_id(thread_id)
		.await
		.map_err(|e| McpError::Internal(format!("Failed to get thread owner: {e}")))?;

	let current_user_id = current_user.user.id.to_string();
	if owner_user_id.as_ref() != Some(&current_user_id) && !current_user.user.is_system_admin() {
		return Err(McpError::Forbidden(
			"You don't have access to this thread".to_string(),
		));
	}

	let content = json!({
		"id": thread.id.to_string(),
		"title": thread.metadata.title,
		"created_at": thread.created_at,
		"updated_at": thread.updated_at,
		"workspace_root": thread.workspace_root,
		"message_count": thread.conversation.messages.len(),
		"messages": thread.conversation.messages,
	});

	Ok(ResourcesReadResult {
		contents: vec![ResourceContent::Text {
			text: serde_json::to_string_pretty(&content)
				.map_err(|e| McpError::Internal(format!("JSON serialization error: {e}")))?,
			mime_type: Some("application/json".to_string()),
		}],
	})
}

/// Read messages from a specific thread.
async fn read_thread_messages(
	state: &AppState,
	current_user: &CurrentUser,
	thread_id: &str,
) -> Result<ResourcesReadResult, McpError> {
	let thread_id_parsed = thread_id.parse().map_err(|_| {
		McpError::InvalidParams(format!("Invalid thread_id format: {thread_id}"))
	})?;

	let thread = state
		.repo
		.get(&thread_id_parsed)
		.await
		.map_err(|e| McpError::Internal(format!("Failed to get thread: {e}")))?
		.ok_or_else(|| McpError::InvalidParams(format!("Thread not found: {thread_id}")))?;

	// Check ownership
	let owner_user_id = state
		.repo
		.get_thread_owner_user_id(thread_id)
		.await
		.map_err(|e| McpError::Internal(format!("Failed to get thread owner: {e}")))?;

	let current_user_id = current_user.user.id.to_string();
	if owner_user_id.as_ref() != Some(&current_user_id) && !current_user.user.is_system_admin() {
		return Err(McpError::Forbidden(
			"You don't have access to this thread".to_string(),
		));
	}

	let content = json!({
		"thread_id": thread.id.to_string(),
		"messages": thread.conversation.messages,
		"count": thread.conversation.messages.len(),
	});

	Ok(ResourcesReadResult {
		contents: vec![ResourceContent::Text {
			text: serde_json::to_string_pretty(&content)
				.map_err(|e| McpError::Internal(format!("JSON serialization error: {e}")))?,
			mime_type: Some("application/json".to_string()),
		}],
	})
}

/// Read the list of repositories.
async fn read_repos_list(
	state: &AppState,
	current_user: &CurrentUser,
) -> Result<ResourcesReadResult, McpError> {
	let scm_repo_store = state.scm_repo_store.as_ref().ok_or_else(|| {
		McpError::Internal("SCM repository store not configured".to_string())
	})?;

	let user_uuid = current_user.user.id.into_inner();
	let repos = scm_repo_store
		.list_by_owner(OwnerType::User, user_uuid)
		.await
		.map_err(|e| McpError::Internal(format!("Failed to list repositories: {e}")))?;

	let repo_list: Vec<_> = repos
		.iter()
		.map(|r| {
			json!({
				"id": r.id.to_string(),
				"name": r.name,
				"owner_type": format!("{:?}", r.owner_type),
				"owner_id": r.owner_id.to_string(),
				"visibility": format!("{:?}", r.visibility),
				"default_branch": r.default_branch,
				"created_at": r.created_at.to_rfc3339(),
			})
		})
		.collect();

	let content = json!({
		"repositories": repo_list,
		"count": repos.len(),
	});

	Ok(ResourcesReadResult {
		contents: vec![ResourceContent::Text {
			text: serde_json::to_string_pretty(&content)
				.map_err(|e| McpError::Internal(format!("JSON serialization error: {e}")))?,
			mime_type: Some("application/json".to_string()),
		}],
	})
}

/// Read a specific repository.
async fn read_repo(
	state: &AppState,
	current_user: &CurrentUser,
	repo_id: &str,
) -> Result<ResourcesReadResult, McpError> {
	let scm_repo_store = state.scm_repo_store.as_ref().ok_or_else(|| {
		McpError::Internal("SCM repository store not configured".to_string())
	})?;

	let repo_uuid = repo_id
		.parse()
		.map_err(|_| McpError::InvalidParams(format!("Invalid repo_id: {repo_id}")))?;

	let repo = scm_repo_store
		.get_by_id(repo_uuid)
		.await
		.map_err(|e| McpError::Internal(format!("Failed to get repository: {e}")))?
		.ok_or_else(|| McpError::InvalidParams(format!("Repository not found: {repo_id}")))?;

	// Check ownership: user must own the repo or be admin
	let user_uuid = current_user.user.id.into_inner();
	let is_owner = repo.owner_type == OwnerType::User && repo.owner_id == user_uuid;

	if !is_owner && !current_user.user.is_system_admin() {
		return Err(McpError::Forbidden(
			"You don't have access to this repository".to_string(),
		));
	}

	let content = json!({
		"id": repo.id.to_string(),
		"name": repo.name,
		"owner_type": format!("{:?}", repo.owner_type),
		"owner_id": repo.owner_id.to_string(),
		"visibility": format!("{:?}", repo.visibility),
		"default_branch": repo.default_branch,
		"created_at": repo.created_at.to_rfc3339(),
		"updated_at": repo.updated_at.to_rfc3339(),
	});

	Ok(ResourcesReadResult {
		contents: vec![ResourceContent::Text {
			text: serde_json::to_string_pretty(&content)
				.map_err(|e| McpError::Internal(format!("JSON serialization error: {e}")))?,
			mime_type: Some("application/json".to_string()),
		}],
	})
}

/// Read the list of weavers.
async fn read_weavers_list(
	state: &AppState,
	current_user: &CurrentUser,
) -> Result<ResourcesReadResult, McpError> {
	let provisioner = state.provisioner.as_ref().ok_or_else(|| {
		McpError::Internal("Weaver provisioner not configured on this server".to_string())
	})?;

	let weavers = if current_user.user.is_system_admin() || current_user.user.is_support() {
		provisioner.list_weavers(None).await?
	} else {
		let user_id = current_user.user.id.to_string();
		provisioner.list_weavers_for_user(&user_id).await?
	};

	let weaver_list: Vec<_> = weavers
		.iter()
		.map(|w| {
			json!({
				"id": w.id.to_string(),
				"image": w.image,
				"status": format!("{:?}", w.status),
				"pod_name": w.pod_name,
				"age_hours": w.age_hours,
				"lifetime_hours": w.lifetime_hours,
			})
		})
		.collect();

	let content = json!({
		"weavers": weaver_list,
		"count": weavers.len(),
	});

	Ok(ResourcesReadResult {
		contents: vec![ResourceContent::Text {
			text: serde_json::to_string_pretty(&content)
				.map_err(|e| McpError::Internal(format!("JSON serialization error: {e}")))?,
			mime_type: Some("application/json".to_string()),
		}],
	})
}

/// Read a specific weaver.
async fn read_weaver(
	state: &AppState,
	current_user: &CurrentUser,
	weaver_id: &str,
) -> Result<ResourcesReadResult, McpError> {
	let provisioner = state.provisioner.as_ref().ok_or_else(|| {
		McpError::Internal("Weaver provisioner not configured on this server".to_string())
	})?;

	let weaver_id_parsed = weaver_id
		.parse()
		.map_err(|_| McpError::InvalidParams(format!("Invalid weaver_id: {weaver_id}")))?;

	let weaver = provisioner.get_weaver(&weaver_id_parsed).await?;

	// Check access
	let can_read = current_user.user.is_system_admin()
		|| current_user.user.is_support()
		|| weaver.owner_user_id == current_user.user.id.to_string();

	if !can_read {
		return Err(McpError::Forbidden(
			"You don't have access to this weaver".to_string(),
		));
	}

	let content = json!({
		"id": weaver.id.to_string(),
		"image": weaver.image,
		"status": format!("{:?}", weaver.status),
		"pod_name": weaver.pod_name,
		"owner_user_id": weaver.owner_user_id,
		"age_hours": weaver.age_hours,
		"lifetime_hours": weaver.lifetime_hours,
		"tags": weaver.tags,
		"created_at": weaver.created_at.to_rfc3339(),
	});

	Ok(ResourcesReadResult {
		contents: vec![ResourceContent::Text {
			text: serde_json::to_string_pretty(&content)
				.map_err(|e| McpError::Internal(format!("JSON serialization error: {e}")))?,
			mime_type: Some("application/json".to_string()),
		}],
	})
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_list_resource_templates() {
		let result = list_resource_templates();
		assert_eq!(result.resource_templates.len(), 4);

		let templates: Vec<&str> = result
			.resource_templates
			.iter()
			.map(|t| t.uri_template.as_str())
			.collect();
		assert!(templates.contains(&"loom://threads/{thread_id}"));
		assert!(templates.contains(&"loom://threads/{thread_id}/messages"));
		assert!(templates.contains(&"loom://repos/{repo_id}"));
		assert!(templates.contains(&"loom://weavers/{weaver_id}"));
	}
}
