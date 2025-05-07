//! kuri is a framework to build [Model Context Protocol][mcp-spec] (MCP) servers, focused on
//! developer ergonomics and clarity.
//!
//! # Example
//!
//! The "Hello World" of kuri is:
//!
//! ```rust
//! use kuri::{MCPServiceBuilder, serve, tool, ServiceExt};
//! use kuri::transport::{StdioTransport, TransportError};
//!
//! #[tool]
//! async fn hello_world_tool() -> String {
//!     "Hello World".to_string()
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<(), TransportError> {
//!     let service = MCPServiceBuilder::new("Hello World".to_string())
//!         .with_tool(HelloWorldTool)
//!         .build();
//!
//!     serve(service.into_request_service(), StdioTransport::new()).await
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
//! # Handling notifications
//!
//! To handle notifications, you'll need to define your own function to handle [`Notification`] and
//! provide this to the [`MCPServiceBuilder`] when building your service.
//!
//! ```rust
//! use kuri::{MCPServiceBuilder};
//! use kuri_mcp_protocol::jsonrpc::Notification;
//!
//! async fn my_notification_handler(notification: Notification) {
//!     println!("Notification received: {:?}", notification.method);
//! }
//!
//! let mut service = MCPServiceBuilder::new("Notification server".to_string())
//!     .with_notification_handler(move |_, notification| {
//!         Box::pin(my_notification_handler(notification))
//!     })
//!     .build();
//! ```
//!
//! ## Error handling
//!
//! The MCP protocol supports two types of errors: RPC errors, and logical errors. In tool handlers,
//! both errors are combined within the same struct, [`ToolError`].
//!
//! # Middleware and layers
//!
//! Like axum, kuri does not have its own bespoke middleware system, and instead utilises the tower
//! ecosystem of middleware. This means you can use anything from [`tower`], [`axum`], or [`tonic`]
//! (gRPC). Middleware can be used to implement functionality like authorisation and logging. More
//! generally, anything that needs to happen before, after, or intercepts a request to a tool, prompt,
//! or resource, can be implemented using tower layers with kuri.
//!
//! We provide [an example][middleware example] of integrating tracing using a layer. Tower also
//! provides [a guide][tower guide to writing middleware] to get started writing middleware.
//!
//! ## Global middleware
//!
//! If your middleware needs to run on all invocations, you can apply the `.layer` using tower's
//! [`ServiceBuilder`]:
//! ```rust
//! use kuri::{MCPServiceBuilder, middleware::tracing::TracingLayer};
//! use tower::ServiceBuilder;
//! # use kuri::tool;
//! # #[tool]
//! # async fn hello_world_tool() -> String {
//! #     "Hello World".to_string()
//! # }
//!
//! let service = MCPServiceBuilder::new("Hello World".to_string())
//!     .with_tool(HelloWorldTool)
//!     .build();
//!
//! let final_service = ServiceBuilder::new()
//!     // Add tracing middleware
//!     .layer(TracingLayer::new())
//!     // Route to the MCP service
//!     .service(service);
//! ```
//!
//! In this case, the layers are applied in order of declaration, before finally routing the request
//! to the MCP service. On return, the handlers are called in reverse order. So the first declared
//! layer will be the first to process an incoming request, and the last to process an outgoing
//! response.
//!
//! ## Per-[tool/prompt/resource] middleware
//!
//! For now, you will need to add the code to your handler to invoke your middleware. We're still
//! working on making this more ergonomic within kuri.
//!
//! ## `.into_request_service()`
//!
//! [`MCPService`] is a service that processes a single JSON-RPC message (represented by [`SendableMessage`]).
//! However, a JSON-RPC request (represented by [`Request`]) may contain a batch of messages as well.
//! [`MCPRequestService`] is a tower service that processes these JSON-RPC requests. On the transport,
//! you'll want to serve a service that handles the JSON-RPC requests. To turn an [`MCPService`] into a
//! [`MCPRequestService`], you can use the `.into_request_service()` method.
//!
//! This has a few implications for middleware. For tracing for instance, you may want this to apply
//! at the request level. In that case, you can use `.into_request_service()` on the service before
//! applying your tracing middleware. Other middleware may prefer to be applied at the message level,
//! and can be applied on [`MCPServer`] instead.
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
//!     .with_state(Inject::new(my_state))
//!     .build();
//! ```
//!
//! You can then access the state within your handlers using by wrapping your type in `Inject`:
//!
//! ```rust
//! # use kuri::context::Inject;
//! # use std::sync::atomic::{AtomicI32, Ordering};
//! # use std::sync::Arc;
//! # struct Counter {
//! #     inner: Arc<AtomicI32>,
//! # }
//!
//! async fn increment(counter: Inject<Counter>, quantity: u32) {
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
//! over some transport, as in the Hello World example above.
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
//! [middleware example]: https://github.com/itsaphel/kuri/tree/main/examples/04_hyper_middleware.rs
//! [`ServiceBuilder`]: https://TODO
//! [tower guide to writing middleware]: https://TODO

pub mod context;
pub mod errors;
mod handler;
pub mod id;
pub mod middleware;
pub mod response;
mod serve;
mod service;
mod service_ext;
pub mod transport;

// aliases
pub use handler::{PromptHandler, ToolHandler};
pub use serve::serve;
pub use service::{MCPRequestService, MCPService, MCPServiceBuilder};
pub use service_ext::ServiceExt;

// re-export certain MCP protocol types
pub use kuri_mcp_protocol::{
    messages::CallToolResult, prompt::PromptArgument, prompt::PromptError, resource::ResourceError,
    tool::generate_tool_schema, tool::ToolError,
};

// re-export macros
pub use kuri_macros::prompt;
pub use kuri_macros::tool;
