use std::sync::atomic::{AtomicI32, Ordering};

use kuri::context::Inject;
use kuri::MCPService;
use kuri::MCPServiceBuilder;
use kuri::ToolError;
use kuri_macros::{prompt, tool};
use kuri_mcp_protocol::jsonrpc::MethodCall;
use kuri_mcp_protocol::jsonrpc::Params;
use kuri_mcp_protocol::jsonrpc::RequestId;
use kuri_mcp_protocol::jsonrpc::ResponseItem;
use kuri_mcp_protocol::jsonrpc::SendableMessage;
use serde::Deserialize;
use tower::Service;
use tracing_subscriber::EnvFilter;

pub async fn call_server(
    server: &mut MCPService,
    method: &str,
    params: serde_json::Value,
) -> Option<ResponseItem> {
    let params = match params {
        serde_json::Value::Object(map) => Some(Params::Map(map)),
        serde_json::Value::Array(array) => Some(Params::Array(array)),
        _ => None,
    };

    let request = MethodCall::new(RequestId::Num(1), method.to_string(), params);
    let future = server.call(SendableMessage::from(request));

    future.await.unwrap()
}

#[tool(
    description = "Perform basic arithmetic operations",
    params(
        x = "First number in the calculation",
        y = "Second number in the calculation",
        operation = "The operation to perform (add, subtract, multiply, divide)"
    )
)]
pub async fn calculator(x: i32, y: i32, operation: String) -> Result<i32, ToolError> {
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

#[tool]
pub async fn calculator_no_desc(x: i32, y: i32, operation: String) -> Result<i32, ToolError> {
    calculator(x, y, operation).await
}

#[derive(Default, Deserialize)]
struct Counter {
    inner: AtomicI32,
}

#[tool(
    description = "Increment the counter by a specified quantity",
    params(quantity = "How much to increment the counter by")
)]
async fn increment(counter: Inject<Counter>, quantity: u32) {
    counter.inner.fetch_add(quantity as i32, Ordering::SeqCst);
}

#[tool(
    description = "Decrement the counter by a specified quantity",
    params(quantity = "How much to decrement the counter by")
)]
async fn decrement(counter: Inject<Counter>, quantity: u32) {
    counter.inner.fetch_sub(quantity as i32, Ordering::SeqCst);
}

#[tool(description = "Get current value of counter")]
async fn get_value(counter: Inject<Counter>) -> i32 {
    counter.inner.load(Ordering::SeqCst)
}

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

pub fn init_tool_server_simple() -> MCPService {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_test_writer()
        .try_init();

    MCPServiceBuilder::new("Calculator".to_string())
        .with_tool(Calculator)
        .build()
}

pub fn init_tool_server_no_desc() -> MCPService {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_test_writer()
        .try_init();

    MCPServiceBuilder::new("Calculator".to_string())
        .with_tool(CalculatorNoDesc)
        .build()
}

pub fn init_tool_server_with_ctx() -> MCPService {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_test_writer()
        .try_init();

    MCPServiceBuilder::new("Counter".to_string())
        .with_tool(Increment)
        .with_tool(Decrement)
        .with_tool(GetValue)
        .with_state(Inject::new(Counter::default()))
        .build()
}

pub fn init_prompt_server() -> MCPService {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_test_writer()
        .try_init();

    MCPServiceBuilder::new("Prompt Server".to_string())
        .with_prompt(ReviewCode)
        .with_prompt(SummariseText)
        .build()
}
