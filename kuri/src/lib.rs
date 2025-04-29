//! kuri is a framework to build [Model Context Protocol][mcp-spec] (MCP) servers, focused on
//! developer ergonomics and clarity.
//!
//! # Example
//!
//! The "Hello World" of kuri is:
//!
//! ```rust,ignore
//! use kuri::{MCPServiceBuilder, serve};
//! use kuri::transport::StdioTransport;
//! use kuri::errors::ServerError;
//!
//! async fn hello_world_tool() -> String {
//!     "Hello World".to_string()
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<(), ServerError> {
//!     let service = MCPServiceBuilder::new("Hello World", "A server with a 'hello world' tool")
//!         .with_tool(HelloWorldTool)
//!         .build();
//!
//!     serve(service, StdioTransport::new()).await
//! }
//! ```
//!
//! There are more [`examples`] in the repository.
//!
//! # Getting started
//!
//! You'll need to add these dependencies to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! kuri = "0.1"
//! tokio = { version = "1", features = ["full"] }
//! serde = { version = "1.0", features = ["derive"] }
//! serde_json = "1.0"
//! schemars = "0.8"
//! async-trait = "0.1"
//! ```
//!
//! The `full` feature of `tokio` isn't necessary, but is the easiest way to get started.
//!
//! # Defining tools and prompts
//!
//! Handlers are called when a tool or prompt is invoked, and define the behaviour of that tool or
//! prompt. They're just normal Rust functions, and can return any type that implements
//! [`IntoCallToolResult`]. Since handlers are just Rust functions, you can use them as normal.
//! Testing is also straightforward; just call the function directly.
//!
//! ## Error handling
//!
//! The MCP protocol supports two types of errors: RPC errors, and logical errors. In tool handlers,
//! both errors are combined within the same struct, [`ToolError`].
//!
//! # Middleware and layers
//!
//! You can re-use anything from the [`tower`] ecosystem.
//!
//! ...
//!
//! # Sharing state with handlers
//!
//! Handlers can share state with each other, and persist state across invocations, through types
//! saved within the MCPService's [`Context`]. As in the [counter example], when creating your
//! service, provide state to the builder:
//!
//! ```rust,ignore
//! let my_state = Counter::default();
//! let service = MCPServiceBuilder::new(...)
//! .with_state(Inject::new(my_state))
//! .build();
//! ```
//!
//! You can then access the state within your handlers using by wrapping your type in `Inject`:
//!
//! ```rust,ignore
//! async fn increment(counter: Inject<Counter>, quantity: u32) -> () {
//!     counter.inner.fetch_add(quantity as i32, Ordering::SeqCst);
//! }
//! ```
//!
//! You don't need to use `Inject`, but it's the easiest way to get started. If you have more
//! specific needs, see the [`FromContext`] trait, which you may implement for your own types.
//!
//! # Transports
//!
//! Once you instantiate a [`MCPService`], you can use the [`serve`] function to start the server
//! over some transport:
//!
//! ```rust,ignore
//! use kuri::serve;
//! use kuri::transport::StdioTransport;
//! use kuri::MCPService;
//!
//! let service = MCPServiceBuilder::new(...).build();
//!
//! serve(service, StdioTransport::new()).await?;
//! ```
//!
//! # Logging
//!
//! kuri uses tokio's tracing throughout for log messages. Typically, applications might consume
//! these messages to stdout, however when using the stdin transport to communicate with the client,
//! we are unable to log messages to stdout, as discussed in [the MCP docs](https://modelcontextprotocol.io/docs/tools/debugging#server-side-logging)
//!
//! You can change the tokio_subscriber writer to any other output stream, for example file logging:
//! ```rust,ignore
//! let file_appender = tracing_appender::rolling::daily(tempfile::tempdir()?, "server.log");
//! tracing_subscriber::fmt()
//!     .with_env_filter(EnvFilter::from_default_env())
//!     .with_writer(file_appender)
//!     .with_target(false)
//!     .with_thread_ids(true)
//!     .with_file(true)
//!     .with_line_number(true)
//!     .init();
//! ```
//!
//! [mcp-spec]: https://modelcontextprotocol.io/specification/2025-03-26/
//! [examples]: https://github.com/itsaphel/kuri/tree/main/examples
//! [tower]: https://github.com/tokio-rs/tower
//! [counter example]: https://github.com/itsaphel/kuri/tree/main/examples/02_stateful_counter_tool_server.rs

pub mod context;
pub mod errors;
mod handler;
pub mod id;
pub mod middleware;
pub mod response;
mod serve;
mod service;
pub mod transport;

// aliases
pub use handler::{PromptHandler, ToolHandler};
pub use serve::serve;
pub use service::{MCPService, MCPServiceBuilder};

// re-export certain MCP protocol types
pub use kuri_mcp_protocol::{
    messages::CallToolResult, prompt::PromptArgument, prompt::PromptError, resource::ResourceError,
    tool::generate_tool_schema, tool::ToolError,
};

// re-export macros
pub use kuri_macros::prompt;
pub use kuri_macros::tool;
