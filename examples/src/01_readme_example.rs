use kuri::{
    MCPServiceBuilder, ServiceExt, ToolError, prompt, serve, tool, transport::StdioTransport,
};
use schemars::JsonSchema;
use serde::Deserialize;
use tracing_subscriber::{self, EnvFilter};

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Operation {
    Add,
    Subtract,
    Multiply,
    Divide,
}

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
async fn calculator(x: i32, y: i32, operation: Operation) -> Result<i32, ToolError> {
    match operation {
        Operation::Add => Ok(x + y),
        Operation::Subtract => Ok(x - y),
        Operation::Multiply => Ok(x * y),
        Operation::Divide => {
            if y == 0 {
                Err(ToolError::ExecutionError("Division by zero".to_string()))
            } else {
                Ok(x / y)
            }
        }
    }
}

// Returns a prompt. The application provides the text to summarise, and (optionally) a format.
// The format argument uses Rust's standard `Option` type. Behind the scenes, kuri uses this fact to tell the model it may omit `format`
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
async fn main() -> anyhow::Result<()> {
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
    let service = MCPServiceBuilder::new("kuri's test server".to_string())
        .with_tool(Calculator)
        .with_prompt(SummariseText)
        .build();

    tracing::info!(
        "Starting server over stdin. Logging to {}",
        log_dir.path().display()
    );

    // Serve over the stdio transport
    serve(service.into_request_service(), StdioTransport::new()).await?;
    Ok(())
}
