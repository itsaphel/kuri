use anyhow::Result;
use kuri::{MCPServiceBuilder, Server, ToolError, context::Inject, tool, transport::ByteTransport};
use serde::Deserialize;
use std::sync::atomic::{AtomicI32, Ordering};
use tokio::io::{stdin, stdout};
use tracing_subscriber::EnvFilter;

#[derive(Default, Deserialize)]
struct Counter {
    inner: AtomicI32,
}

#[tool(
    description = "Increment the counter by a specified quantity",
    params(quantity = "How much to increment the counter by")
)]
async fn increment(counter: Inject<Counter>, quantity: u32) -> Result<(), ToolError> {
    counter.inner.fetch_add(quantity as i32, Ordering::SeqCst);
    Ok(())
}

#[tool(
    description = "Decrement the counter by a specified quantity",
    params(quantity = "How much to decrement the counter by")
)]
async fn decrement(counter: Inject<Counter>, quantity: u32) -> Result<(), ToolError> {
    counter.inner.fetch_sub(quantity as i32, Ordering::SeqCst);
    Ok(())
}

#[tool(description = "Get current value of counter")]
async fn get_value(counter: Inject<Counter>) -> Result<i32, ToolError> {
    Ok(counter.inner.load(Ordering::SeqCst))
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    // Logging
    let log_dir = tempfile::tempdir()?;
    let file_appender = tracing_appender::rolling::daily(log_dir.path(), "server.log");
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(file_appender)
        .with_target(false)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    // Create the MCP service and add our tools
    let service = MCPServiceBuilder::new(
        "Counter".to_string(),
        "This server provides a counter tool that can increment and decrement a counter. You can also get the current value of the counter.".to_string()
    )
    .with_tool(Increment)
    .with_tool(Decrement)
    .with_tool(GetValue)
    .with_state(Inject::new(Counter::default()))
    .build();

    // Create and run the server over the stdio transport
    let server = Server::new(service);
    let transport = ByteTransport::new(stdin(), stdout());

    tracing::info!(
        "Server started over stdin/stdout. Logging to {}. Ready to accept requests",
        log_dir.path().display()
    );
    Ok(server.run(transport).await?)
}
