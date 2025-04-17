# kuri (栗 or くり)

`kuri` is a framework to build [Model Context Protocol][mcp-docs] (MCP) servers, focused on developer ergonomics and clarity.

## Design philosophy

- Ergonomic developer experience. Tools and prompts are just plain Rust functions.
- Minimal use of macros: only for attaching tool metadata (description and param descriptions)
- Minimal boilerplate
- Take advantage of the [`tower`] ecosystem of middleware, services and utilities. Get timeouts, tracing, panic-handling, and more for free. Re-use the same logic you use for [`axum`], [`tonic`] and [`hyper`].

We're inspired by [`axum`] and [`actix`].

## Usage

```rust
#[tool(
    description = "Perform basic arithmetic operations",
    params(
        x = "First number in the calculation",
        y = "Second number in the calculation",
        operation = "The operation to perform (add, subtract, multiply, divide)"
    )
)]
async fn calculator(x: i32, y: i32, operation: String) -> Result<i32, ToolError> {
    match operation.as_str() {
        "add" => Ok(x + y),
        "subtract" => Ok(x - y),
        "multiply" => Ok(x * y),
        "divide" => {
            if y == 0 {
                Err(ToolError::ExecutionError("Division by zero".into()))
            } else {
                Ok(x / y)
            }
        }
        _ => Err(ToolError::InvalidParameters(format!(
            "Unknown operation: {}",
            operation
        ))),
    }
}

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
async fn main() -> Result<()> {
    let service = MCPServiceBuilder::new(
        "kuri's test server".to_string(),
        "This server provides a `calculator` tool that can perform basic arithmetic operations, and a prompt to summarise text.".to_string()
    )
    .with_tool(Calculator)
    .with_prompt(SummariseText)
    .build();

    // Create and run the server over the stdio transport
    let server = Server::new(service);
    let transport = ByteTransport::new(stdin(), stdout());
    Ok(server.run(transport).await?)
}
```

More in [the examples]

## Features

- [x] Tools: Feature complete with tests
- [x] Prompts: Mostly complete with tests
- [ ] Resources
- Transports
  - [x] stdin/stdout
  - [ ] Streaming HTTP ([`2025-03-26` protocol])
- Extra (optional) features
  - [ ] [Completions][mcp-completions]
  - [ ] [Pagination][mcp-pagination]

Our priorities are HTTP transport support, stabilising the API, and ensuring full support of the core specification.

## Contributing

The goal of this project is to build a pleasant, ergonomic, idiomatic Rust library for building MCP Servers. It's in an early phase, so the structure can still change. If you enjoy any of Rust, MCP, building crates, or network protocols (maybe you're into [protohackers](https://protohackers.com/)!), we'd love to have you! Get involved in the repo, or [reach out by email](mailto:aphel@indices.io) if you want to chat!

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
