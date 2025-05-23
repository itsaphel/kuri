use anyhow::Result;
use kuri::{
    MCPServiceBuilder, ServiceExt, ToolError, middleware::tracing::TracingLayer, serve, tool,
    transport::StdioTransport,
};
use tower::ServiceBuilder;
use tracing_subscriber::{self, EnvFilter};

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
                Err(ToolError::ExecutionError("Division by zero".to_string()))
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

#[tokio::main]
async fn main() -> Result<()> {
    // Logging. We need to reroute logs to file, see `docs/LOGGING.md` for more information.
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

    tracing::info!("Starting MCP server");

    // Create the MCP service and add our tools
    let service = MCPServiceBuilder::new("Calculator".to_string())
        .with_tool(Calculator)
        .build();

    tracing::info!(
        "Starting server over stdin. Logging to {}",
        log_dir.path().display()
    );

    let final_service = ServiceBuilder::new()
        // Add tracing middleware
        .layer(TracingLayer::new())
        // Route to the MCP service
        .service(service);

    // Serve over the stdio transport
    serve(final_service.into_request_service(), StdioTransport::new()).await?;

    Ok(())
}
