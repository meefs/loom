// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! MCP Prompts implementation.
//!
//! Provides predefined prompt templates for common workflows.

use super::{
	error::McpError,
	types::{Prompt, PromptArgument, PromptContent, PromptMessage, PromptRole, PromptsGetResult, PromptsListResult},
};
use std::collections::HashMap;

/// Get the list of available prompts.
pub fn list_prompts() -> PromptsListResult {
	PromptsListResult {
		prompts: vec![
			create_weaver_prompt(),
			code_review_prompt(),
			debug_session_prompt(),
			explain_code_prompt(),
			write_tests_prompt(),
		],
	}
}

/// Prompt for creating a weaver with recommended settings.
fn create_weaver_prompt() -> Prompt {
	Prompt {
		name: "create-weaver".to_string(),
		description: Some(
			"Create a weaver (ephemeral execution environment) with recommended settings for a \
			 specific programming language or use case."
				.to_string(),
		),
		arguments: Some(vec![
			PromptArgument {
				name: "language".to_string(),
				description: Some("Programming language (python, node, rust, go, etc.)".to_string()),
				required: Some(true),
			},
			PromptArgument {
				name: "org_id".to_string(),
				description: Some("Organization ID for billing".to_string()),
				required: Some(true),
			},
		]),
	}
}

/// Prompt for code review assistance.
fn code_review_prompt() -> Prompt {
	Prompt {
		name: "code-review".to_string(),
		description: Some(
			"Get help reviewing code changes for bugs, security issues, and best practices."
				.to_string(),
		),
		arguments: Some(vec![
			PromptArgument {
				name: "focus".to_string(),
				description: Some(
					"Focus area: security, performance, style, or all (default: all)".to_string(),
				),
				required: Some(false),
			},
		]),
	}
}

/// Prompt for debugging assistance.
fn debug_session_prompt() -> Prompt {
	Prompt {
		name: "debug-session".to_string(),
		description: Some(
			"Start a debugging session to investigate an issue in your code.".to_string(),
		),
		arguments: Some(vec![
			PromptArgument {
				name: "error_message".to_string(),
				description: Some("The error message or symptom you're seeing".to_string()),
				required: Some(false),
			},
			PromptArgument {
				name: "language".to_string(),
				description: Some("Programming language of the code".to_string()),
				required: Some(false),
			},
		]),
	}
}

/// Prompt for explaining code.
fn explain_code_prompt() -> Prompt {
	Prompt {
		name: "explain-code".to_string(),
		description: Some(
			"Get a detailed explanation of how a piece of code works.".to_string(),
		),
		arguments: Some(vec![
			PromptArgument {
				name: "detail_level".to_string(),
				description: Some(
					"Level of detail: overview, detailed, or expert (default: detailed)".to_string(),
				),
				required: Some(false),
			},
		]),
	}
}

/// Prompt for writing tests.
fn write_tests_prompt() -> Prompt {
	Prompt {
		name: "write-tests".to_string(),
		description: Some(
			"Generate test cases for your code.".to_string(),
		),
		arguments: Some(vec![
			PromptArgument {
				name: "test_type".to_string(),
				description: Some(
					"Type of tests: unit, integration, e2e, or property (default: unit)".to_string(),
				),
				required: Some(false),
			},
			PromptArgument {
				name: "framework".to_string(),
				description: Some("Testing framework to use (e.g., pytest, jest, etc.)".to_string()),
				required: Some(false),
			},
		]),
	}
}

/// Get a prompt by name with filled arguments.
pub fn get_prompt(name: &str, arguments: &HashMap<String, String>) -> Result<PromptsGetResult, McpError> {
	match name {
		"create-weaver" => get_create_weaver_prompt(arguments),
		"code-review" => get_code_review_prompt(arguments),
		"debug-session" => get_debug_session_prompt(arguments),
		"explain-code" => get_explain_code_prompt(arguments),
		"write-tests" => get_write_tests_prompt(arguments),
		_ => Err(McpError::InvalidParams(format!("Unknown prompt: {name}"))),
	}
}

fn get_create_weaver_prompt(arguments: &HashMap<String, String>) -> Result<PromptsGetResult, McpError> {
	let language = arguments
		.get("language")
		.ok_or_else(|| McpError::InvalidParams("Missing required argument: language".to_string()))?;
	let org_id = arguments
		.get("org_id")
		.ok_or_else(|| McpError::InvalidParams("Missing required argument: org_id".to_string()))?;

	let (image, memory, cpu) = match language.to_lowercase().as_str() {
		"python" | "py" => ("python:3.12-slim", "2Gi", "1"),
		"node" | "nodejs" | "javascript" | "js" | "typescript" | "ts" => ("node:20-slim", "2Gi", "1"),
		"rust" => ("rust:1.75-slim", "4Gi", "2"),
		"go" | "golang" => ("golang:1.22-alpine", "2Gi", "1"),
		"ruby" => ("ruby:3.3-slim", "2Gi", "1"),
		"java" => ("eclipse-temurin:21-jdk", "4Gi", "2"),
		"dotnet" | "csharp" | "c#" => ("mcr.microsoft.com/dotnet/sdk:8.0", "4Gi", "2"),
		_ => ("ubuntu:22.04", "2Gi", "1"),
	};

	let prompt_text = format!(
		"Create an ephemeral execution environment for {} development.\n\n\
		Recommended configuration:\n\
		- Image: {}\n\
		- Memory: {}\n\
		- CPU: {} core(s)\n\
		- Organization: {}\n\n\
		Use the create_weaver tool with these settings to spin up the environment. \
		The environment will be automatically cleaned up after 24 hours.\n\n\
		Example tool call:\n\
		```json\n\
		{{\n\
		  \"name\": \"create_weaver\",\n\
		  \"arguments\": {{\n\
		    \"image\": \"{}\",\n\
		    \"org_id\": \"{}\",\n\
		    \"memory_limit\": \"{}\",\n\
		    \"cpu_limit\": \"{}\",\n\
		    \"lifetime_hours\": 24\n\
		  }}\n\
		}}\n\
		```",
		language, image, memory, cpu, org_id, image, org_id, memory, cpu
	);

	Ok(PromptsGetResult {
		description: Some(format!("Create a {} development environment", language)),
		messages: vec![PromptMessage {
			role: PromptRole::User,
			content: PromptContent::Text { text: prompt_text },
		}],
	})
}

fn get_code_review_prompt(arguments: &HashMap<String, String>) -> Result<PromptsGetResult, McpError> {
	let focus = arguments.get("focus").map(|s| s.as_str()).unwrap_or("all");

	let focus_instructions = match focus.to_lowercase().as_str() {
		"security" => {
			"Focus specifically on:\n\
			- Input validation and sanitization\n\
			- Authentication and authorization\n\
			- SQL injection, XSS, CSRF vulnerabilities\n\
			- Secrets handling and exposure\n\
			- Cryptographic best practices"
		}
		"performance" => {
			"Focus specifically on:\n\
			- Algorithm complexity (Big O)\n\
			- Memory allocation patterns\n\
			- Database query efficiency\n\
			- Caching opportunities\n\
			- Unnecessary computations"
		}
		"style" => {
			"Focus specifically on:\n\
			- Code readability and clarity\n\
			- Naming conventions\n\
			- Code organization and structure\n\
			- Comments and documentation\n\
			- Consistency with project patterns"
		}
		_ => {
			"Review for:\n\
			1. **Correctness**: Logic errors, edge cases, null handling\n\
			2. **Security**: Input validation, authentication, data exposure\n\
			3. **Performance**: Efficiency, resource usage, scalability\n\
			4. **Maintainability**: Readability, structure, documentation"
		}
	};

	let prompt_text = format!(
		"I need you to review my code changes.\n\n\
		{}\n\n\
		Please provide:\n\
		- A summary of what the code does\n\
		- Specific issues found with line numbers\n\
		- Suggestions for improvement\n\
		- Any questions about intent or requirements\n\n\
		I'll share the code in my next message.",
		focus_instructions
	);

	Ok(PromptsGetResult {
		description: Some(format!("Code review with {} focus", focus)),
		messages: vec![PromptMessage {
			role: PromptRole::User,
			content: PromptContent::Text { text: prompt_text },
		}],
	})
}

fn get_debug_session_prompt(arguments: &HashMap<String, String>) -> Result<PromptsGetResult, McpError> {
	let error_context = arguments
		.get("error_message")
		.map(|e| format!("The error I'm seeing:\n```\n{}\n```\n\n", e))
		.unwrap_or_default();

	let language_context = arguments
		.get("language")
		.map(|l| format!("Programming language: {}\n\n", l))
		.unwrap_or_default();

	let prompt_text = format!(
		"I need help debugging an issue in my code.\n\n\
		{}{}\
		Please help me:\n\
		1. Understand what might be causing this issue\n\
		2. Identify potential root causes\n\
		3. Suggest debugging steps or diagnostic code\n\
		4. Propose fixes once we identify the problem\n\n\
		I'll share the relevant code and any additional context.",
		language_context, error_context
	);

	Ok(PromptsGetResult {
		description: Some("Start a debugging session".to_string()),
		messages: vec![PromptMessage {
			role: PromptRole::User,
			content: PromptContent::Text { text: prompt_text },
		}],
	})
}

fn get_explain_code_prompt(arguments: &HashMap<String, String>) -> Result<PromptsGetResult, McpError> {
	let detail_level = arguments
		.get("detail_level")
		.map(|s| s.as_str())
		.unwrap_or("detailed");

	let detail_instructions = match detail_level.to_lowercase().as_str() {
		"overview" => {
			"Provide a high-level overview:\n\
			- What the code does in 2-3 sentences\n\
			- The main purpose and functionality\n\
			- Key inputs and outputs"
		}
		"expert" => {
			"Provide an expert-level analysis:\n\
			- Detailed algorithm explanation with complexity analysis\n\
			- Design patterns and architectural decisions\n\
			- Memory and performance characteristics\n\
			- Trade-offs and alternative approaches\n\
			- Edge cases and potential issues"
		}
		_ => {
			"Provide a detailed explanation:\n\
			- What the code does, step by step\n\
			- The purpose of each major section\n\
			- Important variables and data structures\n\
			- Any complex logic or algorithms\n\
			- How different parts interact"
		}
	};

	let prompt_text = format!(
		"I'd like you to explain some code to me.\n\n\
		{}\n\n\
		I'll share the code in my next message.",
		detail_instructions
	);

	Ok(PromptsGetResult {
		description: Some(format!("{} code explanation", detail_level)),
		messages: vec![PromptMessage {
			role: PromptRole::User,
			content: PromptContent::Text { text: prompt_text },
		}],
	})
}

fn get_write_tests_prompt(arguments: &HashMap<String, String>) -> Result<PromptsGetResult, McpError> {
	let test_type = arguments
		.get("test_type")
		.map(|s| s.as_str())
		.unwrap_or("unit");

	let framework = arguments.get("framework").map(|s| s.as_str());

	let type_instructions = match test_type.to_lowercase().as_str() {
		"integration" => {
			"Write integration tests that:\n\
			- Test interactions between components\n\
			- Verify data flow through the system\n\
			- Test with real (or realistic mock) dependencies\n\
			- Cover happy paths and error scenarios"
		}
		"e2e" | "end-to-end" => {
			"Write end-to-end tests that:\n\
			- Test the full user workflow\n\
			- Simulate real user interactions\n\
			- Verify the complete system behavior\n\
			- Test critical user journeys"
		}
		"property" | "property-based" => {
			"Write property-based tests that:\n\
			- Define properties the code should satisfy\n\
			- Generate random inputs to test those properties\n\
			- Find edge cases automatically\n\
			- Test invariants that should always hold"
		}
		_ => {
			"Write unit tests that:\n\
			- Test individual functions/methods in isolation\n\
			- Cover normal cases, edge cases, and error cases\n\
			- Use mocks/stubs for dependencies\n\
			- Are fast and deterministic"
		}
	};

	let framework_note = framework
		.map(|f| format!("\n\nUse the {} testing framework.", f))
		.unwrap_or_default();

	let prompt_text = format!(
		"I need you to write tests for my code.\n\n\
		{}{}\n\n\
		For each test:\n\
		- Give it a descriptive name\n\
		- Arrange the test data\n\
		- Act on the system under test\n\
		- Assert the expected outcomes\n\n\
		I'll share the code to test in my next message.",
		type_instructions, framework_note
	);

	Ok(PromptsGetResult {
		description: Some(format!("Generate {} tests", test_type)),
		messages: vec![PromptMessage {
			role: PromptRole::User,
			content: PromptContent::Text { text: prompt_text },
		}],
	})
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_list_prompts() {
		let result = list_prompts();
		assert_eq!(result.prompts.len(), 5);

		let prompt_names: Vec<&str> = result.prompts.iter().map(|p| p.name.as_str()).collect();
		assert!(prompt_names.contains(&"create-weaver"));
		assert!(prompt_names.contains(&"code-review"));
		assert!(prompt_names.contains(&"debug-session"));
		assert!(prompt_names.contains(&"explain-code"));
		assert!(prompt_names.contains(&"write-tests"));
	}

	#[test]
	fn test_get_create_weaver_prompt() {
		let mut args = HashMap::new();
		args.insert("language".to_string(), "python".to_string());
		args.insert("org_id".to_string(), "test-org-123".to_string());

		let result = get_prompt("create-weaver", &args).unwrap();
		assert!(result.description.is_some());
		assert_eq!(result.messages.len(), 1);
	}

	#[test]
	fn test_get_create_weaver_prompt_missing_args() {
		let args = HashMap::new();
		let result = get_prompt("create-weaver", &args);
		assert!(result.is_err());
	}

	#[test]
	fn test_get_code_review_prompt() {
		let args = HashMap::new();
		let result = get_prompt("code-review", &args).unwrap();
		assert_eq!(result.messages.len(), 1);
	}

	#[test]
	fn test_get_unknown_prompt() {
		let args = HashMap::new();
		let result = get_prompt("unknown-prompt", &args);
		assert!(result.is_err());
	}
}
