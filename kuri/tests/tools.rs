#[allow(unused)]
mod common;

use common::*;
use kuri::MCPService;
use kuri_mcp_protocol::{
    jsonrpc::{ErrorCode, RequestId, ResponseItem},
    messages::{CallToolResult, ListToolsResult},
    Content, TextContent,
};

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
