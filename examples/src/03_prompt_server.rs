use anyhow::Result;
use kuri::transport::ByteTransport;
use kuri::{MCPServiceBuilder, Server, prompt};
use tokio::io::{stdin, stdout};
use tracing_subscriber::{self, EnvFilter};

#[prompt(
    description = "Generates a code review prompt for the provided code",
    params(code = "The code to review")
)]
async fn review_code(code: String) -> String {
    format!("Please review this code:\n\n{}", code)
}

#[prompt(
    description = "Generates a prompt for summarising text",
    params(
        text = "The text to summarise",
        format = "Optional format for the summary (e.g., 'bullet points', 'paragraph')"
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

    // Create the MCP service and add our prompt handlers
    let service = MCPServiceBuilder::new(
        "Prompt Server".to_string(),
        "This server provides prompt templates for various tasks. Use the available prompts to generate formatted prompts for specific tasks.".to_string()
    )
    .with_prompt(ReviewCode)
    .with_prompt(SummariseText)
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
