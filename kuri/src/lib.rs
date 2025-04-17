pub mod context;
pub mod errors;
mod handler;
pub mod id;
mod server;
mod service;
pub mod transport;

// aliases
pub use handler::{PromptHandler, ToolHandler};
pub use server::Server;
pub use service::{MCPService, MCPServiceBuilder};

// re-export certain MCP protocol types
pub use kuri_mcp_protocol::{
    prompt::PromptArgument, prompt::PromptError, resource::ResourceError,
    tool::generate_tool_schema, tool::ToolError,
};

// re-export macros
pub use kuri_macros::prompt;
pub use kuri_macros::tool;
