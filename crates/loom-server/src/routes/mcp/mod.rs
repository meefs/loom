// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! MCP (Model Context Protocol) server endpoint.
//!
//! This module implements the MCP protocol, enabling MCP clients (like Claude Desktop)
//! to create and manage ephemeral K8s execution environments (weavers).
//!
//! ## Protocol
//!
//! - Transport: Streamable HTTP (POST /mcp)
//! - Format: JSON-RPC 2.0
//! - Version: MCP 2025-11-25
//!
//! ## Capabilities
//!
//! ### Tools
//!
//! - `create_weaver`: Create an ephemeral Kubernetes pod
//! - `list_weavers`: List weavers owned by the user
//! - `get_weaver`: Get details of a specific weaver
//! - `delete_weaver`: Delete a weaver
//! - `attach_weaver`: Get connection info for a weaver
//!
//! ### Resources
//!
//! - `loom://threads` - List conversation threads
//! - `loom://threads/{id}` - Thread details with messages
//! - `loom://repos` - List connected repositories
//! - `loom://repos/{id}` - Repository details
//! - `loom://weavers` - List active weavers
//! - `loom://weavers/{id}` - Weaver details
//!
//! ### Prompts
//!
//! - `create-weaver`: Create weaver with recommended settings
//! - `code-review`: Code review assistance
//! - `debug-session`: Debugging session
//! - `explain-code`: Code explanation
//! - `write-tests`: Test generation
//!
//! ## Example Usage
//!
//! ```json
//! // Initialize
//! POST /mcp
//! {
//!   "jsonrpc": "2.0",
//!   "method": "initialize",
//!   "params": {
//!     "protocolVersion": "2025-11-25",
//!     "capabilities": {},
//!     "clientInfo": { "name": "test", "version": "1.0" }
//!   },
//!   "id": 1
//! }
//!
//! // List tools
//! POST /mcp
//! { "jsonrpc": "2.0", "method": "tools/list", "id": 2 }
//!
//! // List resources
//! POST /mcp
//! { "jsonrpc": "2.0", "method": "resources/list", "id": 3 }
//!
//! // List prompts
//! POST /mcp
//! { "jsonrpc": "2.0", "method": "prompts/list", "id": 4 }
//! ```

mod error;
mod handler;
mod prompts;
mod resources;
pub mod session;
mod tools;
mod types;

pub use error::McpError;
pub use handler::{mcp_handler, MCP_SESSION_HEADER};
pub use session::{create_session_store, McpSession, McpSessionStore};
pub use types::{
	JsonRpcError, JsonRpcId, JsonRpcRequest, JsonRpcResponse, MCP_PROTOCOL_VERSION, MCP_SERVER_NAME,
	MCP_SERVER_VERSION,
};
