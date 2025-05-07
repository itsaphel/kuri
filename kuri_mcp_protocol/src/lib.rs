/*!
`kuri_mcp_protocol` provides Rust types for the [Model Context Protocol (MCP)](https://spec.modelcontextprotocol.io).

This crate is intended to be independent of Kuri's implementation and usage of these types, so you
may use `kuri_mcp_protocol` by itself in your project, if you only want the protocol types and not the
`kuri` server framework.

# Organisation

The crate is organised into several modules:

- [`content`](content/index.html) - Content types for communication (text, images, etc.)
- [`jsonrpc`](jsonrpc/index.html) - JSON-RPC protocol implementation
- [`messages`](messages/index.html) - MCP message types
- [`prompt`](prompt/index.html) - Prompt types
- [`resource`](resource/index.html) - Resource types
- [`tool`](tool/index.html) - Tool types

# Basic Usage

## Prompts

```rust
use kuri_mcp_protocol::prompt::{Prompt, PromptMessage, PromptMessageRole};

// Describe a prompt
let prompt = Prompt::new(
    "simple_greeting",
    Some("A simple greeting prompt"),
    None,
);

// Describe a prompt message
let message = PromptMessage::new_text(
    PromptMessageRole::User,
    "Hello, how are you today?",
);
```

## Resources

```rust
use kuri_mcp_protocol::resource::Resource;

// Describe a resource
let resource = Resource::new(
    "file:///example.txt",
    Some("text/plain".to_string()),
    Some("Example File".to_string()),
    None,
).expect("Failed to create resource");
```

## Tools

```rust
use kuri_mcp_protocol::tool::{Tool, generate_tool_schema};
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Deserialize, JsonSchema)]
struct CalculatorParameters {
    #[schemars(description = "First number")]
    x: i32,
    #[schemars(description = "Second number")]
    y: i32,
    #[schemars(description = "Operation to perform")]
    operation: String,
}

// Generate schema for the tool parameters
let schema = generate_tool_schema::<CalculatorParameters>()
    .expect("Failed to generate schema");

// Describe a tool
let tool = Tool::new(
    "calculator",
    "Perform basic arithmetic operations",
    schema,
);
```
*/

pub mod content;
pub use content::{Annotations, Content, ImageContent, TextContent};
pub mod jsonrpc;
pub mod messages;
pub mod prompt;
pub mod resource;
pub mod tool;
