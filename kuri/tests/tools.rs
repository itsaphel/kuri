mod common;

use std::sync::atomic::{AtomicI32, Ordering};

use common::*;
use kuri::{context::Inject, tool, MCPService, MCPServiceBuilder, ToolError};
use kuri_mcp_protocol::{
    jsonrpc::{ErrorCode, RequestId, ResponseItem},
    messages::{CallToolResult, ListToolsResult},
    Content, TextContent,
};
use serde::Deserialize;
use tracing_subscriber::EnvFilter;

// Tool tests
// Spec: https://spec.modelcontextprotocol.io/specification/2025-03-26/server/tools/

#[tokio::test]
async fn test_tools_list() {
    let mut server = init_tool_server_simple();

    // Basic list
    let response = call_server(&mut server, "tools/list", serde_json::json!({}))
        .await
        .unwrap();
    validate_tools_list(response);

    // List with unnecessary params
    let response = call_server(
        &mut server,
        "tools/list",
        serde_json::json!({
            "cursor": "optional-cursor-value",
            "random_param": "some-value",
        }),
    )
    .await
    .unwrap();
    validate_tools_list(response);
}

fn validate_tools_list(response: ResponseItem) {
    match response {
        ResponseItem::Success { id, result, .. } => {
            assert_eq!(id, RequestId::Num(1));

            let actual: ListToolsResult = serde_json::from_value(result).unwrap();
            let expected = ListToolsResult {
                tools: vec![kuri_mcp_protocol::tool::Tool {
                    name: "calculator".to_string(),
                    description: "Perform basic arithmetic operations".to_string(),
                    input_schema: serde_json::json!({
                        "$schema": "http://json-schema.org/draft-07/schema#",
                        "properties": {
                            "operation": {
                                "description": "The operation to perform (add, subtract, multiply, divide)",
                                "type": "string"
                            },
                            "x": {
                                "description": "First number in the calculation",
                                "format": "int32",
                                "type": "integer"
                            },
                            "y": {
                                "description": "Second number in the calculation",
                                "format": "int32",
                                "type": "integer"
                            }
                        },
                        "required": ["operation", "x", "y"],
                        "title": "CalculatorParameters",
                        "type": "object"
                    }),
                }],
            };
            assert_eq!(actual, expected);
        }
        ResponseItem::Error { .. } => {
            panic!("Expected success response");
        }
    }
}

#[tokio::test]
async fn test_tools_call_simple_text() {
    let mut server = init_tool_server_simple();
    verify_calculator(&mut server, "calculator").await;
}

#[tokio::test]
async fn test_tools_call_no_tool_descriptions() {
    let mut server = init_tool_server_no_desc();
    verify_calculator(&mut server, "calculator_no_desc").await;
}

async fn verify_calculator(server: &mut MCPService, tool_name: &str) {
    let response = call_server(
        server,
        "tools/call",
        serde_json::json!({
            "name": tool_name,
            "arguments": {
                "x": 1,
                "y": 2,
                "operation": "add"
            }
        }),
    )
    .await
    .unwrap();

    match response {
        ResponseItem::Success { id, result, .. } => {
            assert_eq!(id, RequestId::Num(1));

            let actual: CallToolResult = serde_json::from_value(result).unwrap();
            let expected = CallToolResult {
                content: vec![Content::Text(TextContent {
                    text: "3".to_string(),
                    annotations: None,
                })],
                is_error: false,
            };
            assert_eq!(actual.content[0], expected.content[0]);
            assert_eq!(actual.is_error, expected.is_error);
        }
        ResponseItem::Error { .. } => {
            panic!("Expected success response");
        }
    }
}

#[tokio::test]
async fn test_tools_call_with_invalid_parameters() {
    // TODO: more descriptive error msg, e.g. "Invalid tool args: missing `operation`"

    let mut server = init_tool_server_simple();

    // Parameters required by tool, but not given in request
    let response = call_server(
        &mut server,
        "tools/call",
        serde_json::json!({
            "name": "calculator",
        }),
    )
    .await
    .unwrap();

    match response {
        ResponseItem::Error { id, error, .. } => {
            assert_eq!(id, RequestId::Num(1));
            assert_eq!(error.code, ErrorCode::InvalidParams);
            assert_eq!(
                error.message,
                "Invalid parameters: Missing or incorrect tool arguments"
            );
        }
        _ => {
            panic!("Expected error response");
        }
    }

    // Not all required params were given
    let response = call_server(
        &mut server,
        "tools/call",
        serde_json::json!({
            "name": "calculator",
            "arguments": {
                "x": 1,
                "y": 2,
            }
        }),
    )
    .await
    .unwrap();

    match response {
        ResponseItem::Success { .. } => {
            panic!("Expected error response");
        }
        ResponseItem::Error { id, error, .. } => {
            assert_eq!(id, RequestId::Num(1));
            assert_eq!(error.code, ErrorCode::InvalidParams);
            assert_eq!(
                error.message,
                "Invalid parameters: Missing or incorrect tool arguments"
            );
        }
    }
}

#[tokio::test]
async fn test_tools_call_invalid_tool() {
    let mut server = init_tool_server_simple();

    let response = call_server(
        &mut server,
        "tools/call",
        serde_json::json!({
            "name": "some_invalid_tool",
            "arguments": {}
        }),
    )
    .await
    .unwrap();

    match response {
        ResponseItem::Success { .. } => {
            panic!("Expected error response");
        }
        ResponseItem::Error { id, error, .. } => {
            assert_eq!(id, RequestId::Num(1));
            assert_eq!(error.code, ErrorCode::InvalidParams);
            assert_eq!(error.message, "Tool not found: some_invalid_tool");
        }
    }
}

#[tokio::test]
async fn test_tools_call_with_context() {
    let mut server = init_tool_server_with_ctx();

    // First call the increment tool
    let response = call_server(
        &mut server,
        "tools/call",
        serde_json::json!({
            "name": "increment",
            "arguments": {
                "quantity": 1
            }
        }),
    )
    .await
    .unwrap();

    match response {
        ResponseItem::Success { id, result, .. } => {
            assert_eq!(id, RequestId::Num(1));
            let actual: CallToolResult = serde_json::from_value(result).unwrap();
            let expected = CallToolResult {
                content: vec![],
                is_error: false,
            };
            assert_eq!(actual.content, expected.content);
            assert_eq!(actual.is_error, expected.is_error);
        }
        ResponseItem::Error { .. } => {
            panic!("Expected success response");
        }
    }

    // Then get the value
    let response = call_server(
        &mut server,
        "tools/call",
        serde_json::json!({
            "name": "get_value",
            "arguments": {}
        }),
    )
    .await
    .unwrap();

    match response {
        ResponseItem::Success { id, result, .. } => {
            assert_eq!(id, RequestId::Num(1));
            let actual: CallToolResult = serde_json::from_value(result).unwrap();
            let expected = CallToolResult {
                content: vec![Content::Text(TextContent {
                    text: "1".to_string(),
                    annotations: None,
                })],
                is_error: false,
            };
            assert_eq!(actual.content, expected.content);
            assert_eq!(actual.is_error, expected.is_error);
        }
        ResponseItem::Error { .. } => {
            panic!("Expected success response");
        }
    }

    // Request may provide no arguments, if no arguments are needed by the tool
    let response = call_server(
        &mut server,
        "tools/call",
        serde_json::json!({ "name": "get_value" }),
    )
    .await
    .unwrap();

    match response {
        ResponseItem::Success { id, result, .. } => {
            assert_eq!(id, RequestId::Num(1));
            let actual: CallToolResult = serde_json::from_value(result).unwrap();
            let expected = CallToolResult {
                content: vec![Content::Text(TextContent {
                    text: "1".to_string(),
                    annotations: None,
                })],
                is_error: false,
            };
            assert_eq!(actual.content, expected.content);
            assert_eq!(actual.is_error, expected.is_error);
        }
        ResponseItem::Error { .. } => {
            panic!("Expected success response when no arguments are provided, if none needed by the tool");
        }
    }
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
