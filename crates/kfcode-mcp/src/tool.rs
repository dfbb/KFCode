//! Tool metadata and in-memory registry for MCP tools.
//!
//! `McpTool` holds the metadata for a single tool exposed by an MCP server.
//! `McpToolRegistry` stores all tools from all connected servers and provides
//! lookup and batch-update operations.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;

/// Metadata for a single tool exposed by an MCP server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    pub server_name: String,
    pub name: String,
    pub full_name: String,
    pub description: Option<String>,
    pub input_schema: serde_json::Value,
}

impl McpTool {
    /// Create a new tool entry, deriving `full_name` as `{server_name}_{name}`.
    pub fn new(
        server_name: &str,
        name: &str,
        description: Option<String>,
        input_schema: serde_json::Value,
    ) -> Self {
        Self {
            server_name: server_name.to_string(),
            name: name.to_string(),
            full_name: format!("{}_{}", server_name, name),
            description,
            input_schema,
        }
    }
}

/// Thread-safe in-memory store of all tools from all connected MCP servers.
pub struct McpToolRegistry {
    tools: RwLock<HashMap<String, McpTool>>,
}

impl McpToolRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            tools: RwLock::new(HashMap::new()),
        }
    }

    /// Insert or replace a single tool entry.
    pub async fn register(&self, tool: McpTool) {
        let mut tools = self.tools.write().await;
        tools.insert(tool.full_name.clone(), tool);
    }

    /// Insert or replace a batch of tool definitions from a single server.
    pub async fn register_batch(
        &self,
        server_name: &str,
        tools: Vec<crate::protocol::ToolDefinition>,
    ) {
        let mut registry = self.tools.write().await;
        for tool_def in tools {
            let mcp_tool = McpTool::new(
                server_name,
                &tool_def.name,
                tool_def.description,
                tool_def.input_schema,
            );
            registry.insert(mcp_tool.full_name.clone(), mcp_tool);
        }
    }

    /// Look up a tool by its `full_name` (`{server}_{tool}`).
    pub async fn get(&self, full_name: &str) -> Option<McpTool> {
        let tools = self.tools.read().await;
        tools.get(full_name).cloned()
    }

    /// Return all registered tools across all servers.
    pub async fn list(&self) -> Vec<McpTool> {
        let tools = self.tools.read().await;
        tools.values().cloned().collect()
    }

    /// Return all tools registered for a specific server.
    pub async fn list_for_server(&self, server_name: &str) -> Vec<McpTool> {
        let tools = self.tools.read().await;
        tools
            .values()
            .filter(|t| t.server_name == server_name)
            .cloned()
            .collect()
    }

    /// Remove all tools belonging to a specific server.
    pub async fn clear_server(&self, server_name: &str) {
        let mut tools = self.tools.write().await;
        tools.retain(|_, t| t.server_name != server_name);
    }

    /// Remove all tools from all servers.
    pub async fn clear(&self) {
        let mut tools = self.tools.write().await;
        tools.clear();
    }
}

impl Default for McpToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}
