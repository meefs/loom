<!--
 Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
 SPDX-License-Identifier: Proprietary
-->

# MCP System

Model Context Protocol (MCP) server endpoint for Loom, enabling MCP clients (like Claude Desktop) to create and manage ephemeral K8s execution environments (weavers), access conversation threads and repositories, and use predefined prompts.

## Overview

The MCP system exposes Loom's capabilities through the standardized Model Context Protocol, allowing AI assistants to programmatically:

- Create and manage sandboxed execution environments (weavers)
- Access conversation threads and repository data
- Use predefined prompt templates for common workflows

### Goals

- Provide MCP-compliant JSON-RPC 2.0 endpoint at `POST /mcp`
- Expose weaver management tools (create, list, get, delete, attach)
- Expose MCP Resources for threads, repositories, and weavers
- Expose MCP Prompts for common workflows
- Reuse existing authentication and authorization infrastructure
- Maintain audit trail for all MCP operations

### Non-Goals

- SSE streaming (server-initiated notifications)
- Sampling capability
- JSON-RPC batch requests

## Protocol Compliance

### MCP Version

Protocol version: `2025-11-25`

### JSON-RPC 2.0

All requests and responses follow JSON-RPC 2.0 specification:

```json
// Request
{
  "jsonrpc": "2.0",
  "method": "tools/list",
  "params": {},
  "id": 1
}

// Success Response
{
  "jsonrpc": "2.0",
  "result": { ... },
  "id": 1
}

// Error Response
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32601,
    "message": "Method not found"
  },
  "id": 1
}
```

### Standard Error Codes

| Code | Name | Description |
|------|------|-------------|
| `-32700` | Parse error | Invalid JSON |
| `-32600` | Invalid request | Missing required fields |
| `-32601` | Method not found | Unknown method |
| `-32602` | Invalid params | Invalid method parameters |
| `-32603` | Internal error | Server error |

### Custom Error Codes

| Code | Name | Description |
|------|------|-------------|
| `-32001` | Tool not found | Unknown tool name |
| `-32002` | Forbidden | Insufficient permissions |
| `-32003` | Unauthorized | Authentication required |

## Transport

### Streamable HTTP

Single endpoint handling all MCP methods:

```
POST /mcp
Content-Type: application/json
Authorization: Bearer <token>
```

### Session Management

Optional session tracking via `Mcp-Session-Id` header:

1. Client sends `initialize` request
2. Server responds with `Mcp-Session-Id` header
3. Client includes header in subsequent requests
4. Sessions expire after 1 hour of inactivity

Sessions are optional - stateless operation is fully supported.

## Authentication

### Bearer Token Authentication

Uses existing Loom authentication infrastructure:

```
Authorization: Bearer <api-key-or-session-token>
```

### 401 Challenge

Unauthenticated requests receive:

```http
HTTP/1.1 401 Unauthorized
WWW-Authenticate: Bearer realm="loom-mcp"
Content-Type: application/json

{
  "jsonrpc": "2.0",
  "error": {
    "code": -32003,
    "message": "Authentication required"
  },
  "id": null
}
```

## Server Capabilities

Declared during `initialize` handshake:

```json
{
  "protocolVersion": "2025-11-25",
  "capabilities": {
    "tools": {},
    "resources": {},
    "prompts": {}
  },
  "serverInfo": {
    "name": "loom-mcp",
    "version": "0.1.0"
  }
}
```

## MCP Methods

### initialize

Handshake establishing protocol version and capabilities.

**Request:**
```json
{
  "jsonrpc": "2.0",
  "method": "initialize",
  "params": {
    "protocolVersion": "2025-11-25",
    "capabilities": {},
    "clientInfo": {
      "name": "claude-desktop",
      "version": "1.0.0"
    }
  },
  "id": 1
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": {
    "protocolVersion": "2025-11-25",
    "capabilities": {
      "tools": {},
      "resources": {},
      "prompts": {}
    },
    "serverInfo": {
      "name": "loom-mcp",
      "version": "0.1.0"
    }
  },
  "id": 1
}
```

### notifications/initialized

Client notification that initialization is complete. No response expected.

```json
{
  "jsonrpc": "2.0",
  "method": "notifications/initialized"
}
```

### tools/list

List available tools.

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": {
    "tools": [
      {
        "name": "create_weaver",
        "description": "Create an ephemeral Kubernetes pod for code execution",
        "inputSchema": { ... }
      },
      {
        "name": "list_weavers",
        "description": "List weavers owned by the current user",
        "inputSchema": { ... }
      },
      {
        "name": "get_weaver",
        "description": "Get detailed information about a specific weaver",
        "inputSchema": { ... }
      },
      {
        "name": "delete_weaver",
        "description": "Delete a weaver and terminate its Kubernetes pod",
        "inputSchema": { ... }
      },
      {
        "name": "attach_weaver",
        "description": "Get connection information for attaching to a weaver",
        "inputSchema": { ... }
      }
    ]
  },
  "id": 2
}
```

### tools/call

Execute a tool.

**Request:**
```json
{
  "jsonrpc": "2.0",
  "method": "tools/call",
  "params": {
    "name": "create_weaver",
    "arguments": {
      "image": "python:3.12",
      "org_id": "550e8400-e29b-41d4-a716-446655440000"
    }
  },
  "id": 3
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": {
    "content": [
      {
        "type": "text",
        "text": "Created weaver abc123 with image python:3.12"
      }
    ],
    "isError": false
  },
  "id": 3
}
```

### resources/list

List available resources.

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": {
    "resources": [
      {
        "uri": "loom://threads",
        "name": "Threads",
        "description": "List of your conversation threads",
        "mimeType": "application/json"
      },
      {
        "uri": "loom://repos",
        "name": "Repositories",
        "description": "List of your repositories",
        "mimeType": "application/json"
      },
      {
        "uri": "loom://weavers",
        "name": "Weavers",
        "description": "List of your active weavers",
        "mimeType": "application/json"
      }
    ]
  },
  "id": 4
}
```

### resources/read

Read a resource by URI.

**Request:**
```json
{
  "jsonrpc": "2.0",
  "method": "resources/read",
  "params": {
    "uri": "loom://threads"
  },
  "id": 5
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": {
    "contents": [
      {
        "type": "text",
        "text": "{\"threads\": [...], \"count\": 10}",
        "mimeType": "application/json"
      }
    ]
  },
  "id": 5
}
```

### resources/templates/list

List resource URI templates.

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": {
    "resourceTemplates": [
      {
        "uriTemplate": "loom://threads/{thread_id}",
        "name": "Thread",
        "description": "A conversation thread by ID",
        "mimeType": "application/json"
      },
      {
        "uriTemplate": "loom://repos/{repo_id}",
        "name": "Repository",
        "description": "A repository by ID",
        "mimeType": "application/json"
      },
      {
        "uriTemplate": "loom://weavers/{weaver_id}",
        "name": "Weaver",
        "description": "An active weaver by ID",
        "mimeType": "application/json"
      }
    ]
  },
  "id": 6
}
```

### prompts/list

List available prompts.

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": {
    "prompts": [
      {
        "name": "create-weaver",
        "description": "Create a weaver with recommended settings",
        "arguments": [
          { "name": "language", "required": true },
          { "name": "org_id", "required": true }
        ]
      },
      {
        "name": "code-review",
        "description": "Get help reviewing code changes"
      },
      {
        "name": "debug-session",
        "description": "Start a debugging session"
      },
      {
        "name": "explain-code",
        "description": "Get a detailed explanation of code"
      },
      {
        "name": "write-tests",
        "description": "Generate test cases for code"
      }
    ]
  },
  "id": 7
}
```

### prompts/get

Get a prompt with filled arguments.

**Request:**
```json
{
  "jsonrpc": "2.0",
  "method": "prompts/get",
  "params": {
    "name": "create-weaver",
    "arguments": {
      "language": "python",
      "org_id": "550e8400-e29b-41d4-a716-446655440000"
    }
  },
  "id": 8
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": {
    "description": "Create a python development environment",
    "messages": [
      {
        "role": "user",
        "content": {
          "type": "text",
          "text": "Create an ephemeral execution environment for python development..."
        }
      }
    ]
  },
  "id": 8
}
```

## Tools

### create_weaver

Create an ephemeral Kubernetes pod for code execution.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "image": {
      "type": "string",
      "description": "Container image (e.g., 'python:3.12', 'node:20')"
    },
    "org_id": {
      "type": "string",
      "description": "Organization ID (UUID) for billing and isolation"
    },
    "env": {
      "type": "object",
      "additionalProperties": { "type": "string" },
      "description": "Environment variables to set in the container"
    },
    "memory_limit": {
      "type": "string",
      "description": "Memory limit (e.g., '8Gi')"
    },
    "cpu_limit": {
      "type": "string",
      "description": "CPU limit (e.g., '4')"
    },
    "lifetime_hours": {
      "type": "integer",
      "description": "TTL in hours (max 48, default from server config)"
    },
    "tags": {
      "type": "object",
      "additionalProperties": { "type": "string" },
      "description": "Custom tags for the weaver"
    }
  },
  "required": ["image", "org_id"]
}
```

### list_weavers

List weavers owned by the current user. System admins can see all weavers.

**Input Schema:**
```json
{
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
}
```

### get_weaver

Get detailed information about a specific weaver.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "weaver_id": {
      "type": "string",
      "description": "The unique identifier of the weaver"
    }
  },
  "required": ["weaver_id"]
}
```

### delete_weaver

Delete a weaver and terminate its Kubernetes pod. Only the owner or system admins can delete a weaver.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "weaver_id": {
      "type": "string",
      "description": "The unique identifier of the weaver to delete"
    }
  },
  "required": ["weaver_id"]
}
```

### attach_weaver

Get connection information for attaching to a weaver's terminal.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "weaver_id": {
      "type": "string",
      "description": "The unique identifier of the weaver to attach to"
    }
  },
  "required": ["weaver_id"]
}
```

## Resources

Resources expose Loom data for reading by MCP clients.

### Resource URIs

| URI | Description |
|-----|-------------|
| `loom://threads` | List of user's conversation threads |
| `loom://threads/{id}` | Full thread with messages |
| `loom://threads/{id}/messages` | Thread messages only |
| `loom://repos` | List of user's repositories |
| `loom://repos/{id}` | Repository details |
| `loom://weavers` | List of user's weavers |
| `loom://weavers/{id}` | Weaver details |

### Authorization

- Users can only access their own threads, repos owned by them, and weavers they own
- System admins can access all resources
- Support users can read weavers (for troubleshooting)

## Prompts

Prompts provide predefined templates for common workflows.

### create-weaver

Create a weaver with recommended settings for a programming language.

**Arguments:**
- `language` (required): Programming language (python, node, rust, go, etc.)
- `org_id` (required): Organization ID for billing

Provides recommended image, memory, and CPU settings based on language.

### code-review

Get help reviewing code changes for bugs, security issues, and best practices.

**Arguments:**
- `focus` (optional): Focus area - security, performance, style, or all

### debug-session

Start a debugging session to investigate an issue in code.

**Arguments:**
- `error_message` (optional): The error message or symptom
- `language` (optional): Programming language of the code

### explain-code

Get a detailed explanation of how code works.

**Arguments:**
- `detail_level` (optional): Level of detail - overview, detailed, or expert

### write-tests

Generate test cases for code.

**Arguments:**
- `test_type` (optional): Type of tests - unit, integration, e2e, or property
- `framework` (optional): Testing framework to use

## Security

### Organization Membership

Tool executions that create resources verify organization membership:

```rust
if !current_user.user.is_system_admin() {
    match org_repo.get_membership(&org_id, &current_user.user.id).await {
        Ok(Some(_)) => { /* allowed */ }
        Ok(None) => { return Err(McpError::Forbidden(...)); }
        Err(e) => { return Err(McpError::Internal(...)); }
    }
}
```

### Weaver Access Control

| Action | Owner | System Admin | Support |
|--------|-------|--------------|---------|
| list_weavers | Own only | All | All |
| get_weaver | Yes | Yes | Yes (read) |
| delete_weaver | Yes | Yes | No |
| attach_weaver | Full | Full | Read-only |

### Audit Logging

All MCP operations are logged:

```rust
audit_service.log(
    AuditLogBuilder::new(AuditEventType::WeaverCreated)
        .actor(user_id)
        .resource("weaver", weaver_id)
        .details(json!({
            "source": "mcp",
            "image": image,
            "org_id": org_id,
        }))
        .build(),
);
```

## Implementation

### Module Structure

```
crates/loom-server/src/routes/mcp/
├── mod.rs        # Module exports, route registration
├── types.rs      # JSON-RPC 2.0 and MCP protocol types
├── handler.rs    # Main POST handler and method routing
├── session.rs    # Session state management
├── tools.rs      # Tool definitions and execution
├── resources.rs  # Resource handling
├── prompts.rs    # Prompt definitions
└── error.rs      # MCP-specific errors
```

### Key Types

```rust
// JSON-RPC request
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: Option<serde_json::Value>,
    pub id: Option<JsonRpcId>,
}

// JSON-RPC response
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
    pub id: Option<JsonRpcId>,
}

// MCP tool definition
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

// MCP resource definition
pub struct Resource {
    pub uri: String,
    pub name: String,
    pub description: Option<String>,
    pub mime_type: Option<String>,
}

// MCP prompt definition
pub struct Prompt {
    pub name: String,
    pub description: Option<String>,
    pub arguments: Option<Vec<PromptArgument>>,
}
```

### Route Registration

```rust
// In api.rs authed routes
.route("/mcp", post(routes::mcp::mcp_handler))
```

## Testing

### Unit Tests

```rust
#[test]
fn test_list_tools() {
    let result = list_tools();
    assert_eq!(result.tools.len(), 5);
}

#[test]
fn test_list_resource_templates() {
    let result = list_resource_templates();
    assert_eq!(result.resource_templates.len(), 4);
}

#[test]
fn test_list_prompts() {
    let result = list_prompts();
    assert_eq!(result.prompts.len(), 5);
}
```

### Manual Testing

```bash
# Initialize
curl -X POST https://loom.ghuntley.com/mcp \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}'

# List tools
curl -X POST https://loom.ghuntley.com/mcp \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"tools/list","id":2}'

# List resources
curl -X POST https://loom.ghuntley.com/mcp \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"resources/list","id":3}'

# List prompts
curl -X POST https://loom.ghuntley.com/mcp \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"prompts/list","id":4}'

# Get a prompt
curl -X POST https://loom.ghuntley.com/mcp \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"prompts/get","params":{"name":"create-weaver","arguments":{"language":"python","org_id":"<org-uuid>"}},"id":5}'
```

## Future Extensions

### SSE Streaming

Add GET endpoint for server-initiated notifications:

```
GET /mcp
Accept: text/event-stream
```

### Additional Prompts

- Repository analysis
- Code migration assistance
- Performance optimization

## References

- [MCP Specification](https://spec.modelcontextprotocol.io/)
- [JSON-RPC 2.0](https://www.jsonrpc.org/specification)
- [Weaver Provisioner](./weaver-provisioner.md)
