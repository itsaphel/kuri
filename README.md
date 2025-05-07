# kuri (栗 or くり)

`kuri` is a framework to build [Model Context Protocol][mcp-docs] (MCP) servers, focused on developer ergonomics and clarity.

[![Build status](https://github.com/itsaphel/kuri/actions/workflows/ci.yml/badge.svg)](https://github.com/itsaphel/kuri/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/kuri)](https://crates.io/crates/kuri)
[![Documentation](https://docs.rs/kuri/badge.svg)](https://docs.rs/kuri)

MCP allows an LLM to execute predefined functions (called 'tools'), which allow it to fetch data and have side effects (ie: interact with the outside world). The LLM just needs to provide the input arguments of the function, which is then executed and the response is returned to the model. These tools are provided by "MCP servers", which can be ran locally, and a single server can provide multiple tools.

## Design philosophy

Rust is an excellent language to write reliable MCP servers, with its strong type system and correctness guarantees. `kuri` aims to make MCP server programming in Rust extremely pleasant, to facilitate using Rust for building MCP servers. Our design goals are:

- **Ergonomic developer experience:** MCP server programming should feel like normal Rust programming. Tools and prompts are just plain async Rust functions.
- **Minimal use of macros** (`#[tool]`, `#[prompt]`): only for attaching tool and argument descriptions, not complex code generation.
- **Minimal boilerplate:** focus on your application logic, not on serialisation or MCP protocol routing.
- Take advantage of the [`tower`] ecosystem of middleware, services and utilities. Get timeouts, tracing, panic-handling, and more for free, and re-use components from [`axum`], [`tonic`] and [`hyper`].

The above is some of what sets us apart from other MCP server crates. We're focused on doing one thing, and doing it really well. And there's no magic complex macros, your application code remains self-explanatory, and `kuri`'s internals are clean to read and understand. `kuri` also builds on [`tower`], allowing you to re-use a rich ecosystem of middleware and layers.

## Example

```rust
use kuri::transport::{StdioTransport, TransportError};
use kuri::{MCPServiceBuilder, ServiceExt, prompt, serve, tool};

// A pure function that takes three inputs and returns an integer. Descriptions
// for the tool and its parameters help the model decide which tool to use, and
// correctly supply the tool's parameters.
#[tool(
    description = "Perform basic arithmetic operations",
    params(
        x = "First number in the calculation",
        y = "Second number in the calculation",
        operation = "The operation to perform (add, subtract, multiply, divide)"
    )
)]
async fn calculator(x: i32, y: i32, operation: String) -> Result<i32, String> {
    match operation.as_str() {
        "add" => Ok(x + y),
        "subtract" => Ok(x - y),
        "multiply" => Ok(x * y),
        "divide" => {
            if y == 0 {
                Err("Division by zero".to_string())
            } else {
                Ok(x / y)
            }
        }
        _ => Err(format!("Unknown operation: {}", operation)),
    }
}

// Creates a prompt template for text summarisation. The application provides
// the text to summarise, and an optional format parameter (denoted using Rust's
// `Option` type). kuri tells the model that `format` may be omitted.
#[prompt(
    description = "Generates a prompt for summarising text",
    params(
        text = "The text to summarise",
        format = "Optional format for the summary (eg: 'bullet points' or 'Shakespeare')"
    )
)]
async fn summarise_text(text: String, format: Option<String>) -> String {
    let format_instruction = match format {
        Some(f) => format!(" in the format of {}", f),
        None => String::new(),
    };

    format!(
        "Please summarize the following text{}:\n\n{}",
        format_instruction, text
    )
}

#[tokio::main]
async fn main() -> Result<(), TransportError> {
    // Create the MCP service, with the server's name and description
    let service = MCPServiceBuilder::new(
        "kuri's test server".to_string(),
        "This server provides a `calculator` tool that can perform basic arithmetic operations, and a prompt to summarise text.".to_string()
    )
    // Register the tool and prompt
    .with_tool(Calculator)
    .with_prompt(SummariseText)
    .build();

    // Serve over the stdio transport
    serve(service.into_request_service(), StdioTransport::new()).await
}
```

More in [the examples]

To get started, add `kuri` and some necessary dependencies to your `Cargo.toml`:

```toml
[dependencies]
kuri = "0.1"
async-trait = "0.1"
schemars = "0.8"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1", features = ["full"] }
```

## MCP specification support

- [x] Core lifecycle: connection initialisation, capability negotiation, and session control
- [x] Tools: Feature complete with tests
- [x] Prompts: Mostly complete with tests
- [ ] Resources
- Transports
  - [x] stdin/stdout
  - [ ] Streaming HTTP ([`2025-03-26` protocol])
- Extra (optional) features
  - [ ] [Completions][mcp-completions]
  - [ ] [Pagination][mcp-pagination]

Our current priorities are adding HTTP transport support, stabilising the API, and ensuring full support of the core specification.

## Contributing

The goal of this project is to build a pleasant, ergonomic, idiomatic Rust library for building MCP Servers. It's in an early phase, so the structure can still change. If you enjoy any of Rust, MCP, building crates, or network protocols (maybe you're into [protohackers](https://protohackers.com/)!), we'd love to have you! Get involved in the repo, or [reach out by email](mailto:aphel@indices.io) if you want to chat!

If you've used this framework for your project: thanks for trying it out! I'd love to hear about your experience!

[`hyper`]: https://github.com/hyperium/hyper
[`tonic`]: https://github.com/hyperium/tonic
[`tower`]: https://github.com/tower-rs/tower
[`axum`]: https://github.com/tokio-rs/axum
[`actix`]: https://github.com/actix/actix-web
[mcp-docs]: https://modelcontextprotocol.io
[the examples]: examples/
[`2025-03-26` protocol]: https://modelcontextprotocol.io/specification/2025-03-26/basic/transports#streamable-http
[mcp-completions]: https://modelcontextprotocol.io/specification/2025-03-26/server/utilities/completion
[mcp-pagination]: https://spec.modelcontextprotocol.io/specification/2025-03-26/server/utilities/pagination/
