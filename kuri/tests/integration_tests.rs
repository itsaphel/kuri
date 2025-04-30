#[allow(unused)]
mod common;

use common::*;
use kuri::MCPService;
use kuri_mcp_protocol::{
    jsonrpc::{ErrorCode, JsonRpcRequest, JsonRpcResponse, Params, RequestId, SendableMessage},
    messages::{
        CallToolResult, GetPromptResult, Implementation, InitializeResult, ListPromptsResult,
        ListToolsResult, ServerCapabilities, ToolsCapability,
    },
    Content, TextContent,
};
use tower::Service;

async fn call_server(
    server: &mut MCPService,
    method: &str,
    params: serde_json::Value,
) -> Option<JsonRpcResponse> {
    let params = match params {
        serde_json::Value::Object(map) => Some(Params::Map(map)),
        serde_json::Value::Array(array) => Some(Params::Array(array)),
        _ => None,
    };

    let request = JsonRpcRequest::new(RequestId::Num(1), method.to_string(), params);
    let future = server.call(SendableMessage::from(request));

    future.await.unwrap()
}

// Utility (ping): https://spec.modelcontextprotocol.io/specification/2025-03-26/basic/utilities/ping/
#[tokio::test]
async fn test_ping() {
    let mut server = init_tool_server_simple();

    let response = call_server(&mut server, "ping", serde_json::json!({}))
        .await
        .unwrap();

    match response {
        JsonRpcResponse::Success { result, .. } => {
            assert_eq!(result, serde_json::json!({}));
        }
        JsonRpcResponse::Error { .. } => {
            panic!("Expected success response");
        }
    }
}

// Client initialisation
// Spec: https://spec.modelcontextprotocol.io/specification/2025-03-26/basic/lifecycle/#initialization
#[tokio::test]
async fn test_initialize() {
    let mut server = init_tool_server_simple();

    let response = call_server(
        &mut server,
        "initialize",
        serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
              "name": "ExampleClient",
              "version": "1.0.0"
            }
        }),
    )
    .await
    .unwrap();

    match response {
        JsonRpcResponse::Success { id, result, .. } => {
            assert_eq!(id, RequestId::Num(1));

            let actual: InitializeResult = serde_json::from_value(result).unwrap();
            let expected = InitializeResult {
                protocol_version: "2024-11-05".to_string(),
                capabilities: ServerCapabilities {
                    prompts: None,
                    resources: None,
                    tools: Some(ToolsCapability {
                        list_changed: Some(false),
                    }),
                },
                server_info: Implementation {
                    name: "Calculator".to_string(),
                    version: "0.1.0".to_string(),
                },
                instructions: Some("Test calculator server".to_string()),
            };
            assert_eq!(actual, expected);
        }
        JsonRpcResponse::Error { .. } => {
            panic!("Expected success response");
        }
    }
}

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

fn validate_tools_list(response: JsonRpcResponse) {
    match response {
        JsonRpcResponse::Success { id, result, .. } => {
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
        JsonRpcResponse::Error { .. } => {
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
        JsonRpcResponse::Success { id, result, .. } => {
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
        JsonRpcResponse::Error { .. } => {
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
        JsonRpcResponse::Error { id, error, .. } => {
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
        JsonRpcResponse::Success { .. } => {
            panic!("Expected error response");
        }
        JsonRpcResponse::Error { id, error, .. } => {
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
        JsonRpcResponse::Success { .. } => {
            panic!("Expected error response");
        }
        JsonRpcResponse::Error { id, error, .. } => {
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
        JsonRpcResponse::Success { id, result, .. } => {
            assert_eq!(id, RequestId::Num(1));
            let actual: CallToolResult = serde_json::from_value(result).unwrap();
            let expected = CallToolResult {
                content: vec![],
                is_error: false,
            };
            assert_eq!(actual.content, expected.content);
            assert_eq!(actual.is_error, expected.is_error);
        }
        JsonRpcResponse::Error { .. } => {
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
        JsonRpcResponse::Success { id, result, .. } => {
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
        JsonRpcResponse::Error { .. } => {
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
        JsonRpcResponse::Success { id, result, .. } => {
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
        JsonRpcResponse::Error { .. } => {
            panic!("Expected success response when no arguments are provided, if none needed by the tool");
        }
    }
}

// Prompt tests
// Spec: https://spec.modelcontextprotocol.io/specification/2025-03-26/server/prompts/

#[tokio::test]
async fn test_prompts_list() {
    let mut server = init_prompt_server();

    let response = call_server(&mut server, "prompts/list", serde_json::json!({}))
        .await
        .unwrap();

    match response {
        JsonRpcResponse::Success { id, result, .. } => {
            assert_eq!(id, RequestId::Num(1));

            let actual: ListPromptsResult = serde_json::from_value(result).unwrap();
            let expected = ListPromptsResult {
                prompts: vec![
                    kuri_mcp_protocol::prompt::Prompt {
                        name: "review_code".to_string(),
                        description: Some(
                            "Generates a code review prompt for the provided code".to_string(),
                        ),
                        arguments: Some(vec![kuri_mcp_protocol::prompt::PromptArgument {
                            name: "code".to_string(),
                            description: Some("The code to review".to_string()),
                            required: Some(true),
                        }]),
                    },
                    kuri_mcp_protocol::prompt::Prompt {
                        name: "summarise_text".to_string(),
                        description: Some("Generates a prompt for summarising text".to_string()),
                        arguments: Some(vec![
                            kuri_mcp_protocol::prompt::PromptArgument {
                                name: "text".to_string(),
                                description: Some("The text to summarise".to_string()),
                                required: Some(true),
                            },
                            kuri_mcp_protocol::prompt::PromptArgument {
                                name: "format".to_string(),
                                description: Some("Optional format for the summary (e.g., 'bullet points', 'paragraph')".to_string()),
                                required: Some(false),
                            },
                        ]),
                    },
                ],
            };

            // Order doesn't matter in the listing
            assert_eq!(actual.prompts.len(), expected.prompts.len());
            for prompt in actual.prompts {
                assert!(expected.prompts.contains(&prompt));
                let expected_prompt = expected
                    .prompts
                    .iter()
                    .find(|p| p.name == prompt.name)
                    .unwrap();
                assert_eq!(prompt.description, expected_prompt.description);
                assert_eq!(prompt.arguments, expected_prompt.arguments);
            }
        }
        JsonRpcResponse::Error { .. } => {
            panic!("Expected success response");
        }
    }
}

#[tokio::test]
async fn test_prompts_get_simple() {
    let mut server = init_prompt_server();

    let response = call_server(
        &mut server,
        "prompts/get",
        serde_json::json!({
            "name": "review_code",
            "arguments": {
                "code": "123"
            }
        }),
    )
    .await
    .unwrap();

    match response {
        JsonRpcResponse::Success { id, result, .. } => {
            assert_eq!(id, RequestId::Num(1));
            let actual: GetPromptResult = serde_json::from_value(result).unwrap();
            let expected = GetPromptResult {
                description: None,
                messages: vec![kuri_mcp_protocol::prompt::PromptMessage {
                    role: kuri_mcp_protocol::prompt::PromptMessageRole::User,
                    content: kuri_mcp_protocol::prompt::PromptMessageContent::Text {
                        text: "Please review this code:\n\n123".to_string(),
                    },
                }],
            };
            assert_eq!(actual, expected);
        }
        JsonRpcResponse::Error { .. } => {
            panic!("Expected success response");
        }
    }
}

#[tokio::test]
async fn test_prompts_get_invalid_prompt() {
    let mut server = init_prompt_server();

    let response = call_server(
        &mut server,
        "prompts/get",
        serde_json::json!({
            "name": "some_invalid_prompt",
            "arguments": {}
        }),
    )
    .await
    .unwrap();

    match response {
        JsonRpcResponse::Error { id, error, .. } => {
            assert_eq!(id, RequestId::Num(1));
            assert_eq!(error.code, ErrorCode::InvalidParams);
            assert_eq!(
                error.message,
                "Invalid parameters: Prompt not found: some_invalid_prompt"
            );
        }
        _ => {
            panic!("Expected error response");
        }
    }
}

#[tokio::test]
async fn test_prompts_get_optional_params() {
    let mut server = init_prompt_server();

    // Include the optional format parameter
    let response = call_server(
        &mut server,
        "prompts/get",
        serde_json::json!({
            "name": "summarise_text",
            "arguments": {
                "text": "123",
                "format": "bullet points"
            }
        }),
    )
    .await
    .unwrap();

    match response {
        JsonRpcResponse::Success { id, result, .. } => {
            assert_eq!(id, RequestId::Num(1));
            let actual: GetPromptResult = serde_json::from_value(result).unwrap();
            let expected = GetPromptResult {
                description: None,
                messages: vec![kuri_mcp_protocol::prompt::PromptMessage {
                    role: kuri_mcp_protocol::prompt::PromptMessageRole::User,
                    content: kuri_mcp_protocol::prompt::PromptMessageContent::Text {
                        text: "Please summarize the following text in the format of bullet points:\n\n123".to_string(),
                    },
                }],
            };
            assert_eq!(actual, expected);
        }
        JsonRpcResponse::Error { .. } => {
            panic!("Expected success response");
        }
    }

    // Don't include the format parameter
    let response = call_server(
        &mut server,
        "prompts/get",
        serde_json::json!({
            "name": "summarise_text",
            "arguments": {
                "text": "123"
            }
        }),
    )
    .await
    .unwrap();

    match response {
        JsonRpcResponse::Success { id, result, .. } => {
            assert_eq!(id, RequestId::Num(1));
            let actual: GetPromptResult = serde_json::from_value(result).unwrap();
            let expected = GetPromptResult {
                description: None,
                messages: vec![kuri_mcp_protocol::prompt::PromptMessage {
                    role: kuri_mcp_protocol::prompt::PromptMessageRole::User,
                    content: kuri_mcp_protocol::prompt::PromptMessageContent::Text {
                        text: "Please summarize the following text:\n\n123".to_string(),
                    },
                }],
            };
            assert_eq!(actual, expected);
        }
        JsonRpcResponse::Error { .. } => {
            panic!("Expected success response");
        }
    }
}

// General server and JSON-RPC tests

#[tokio::test]
async fn test_unknown_method() {
    let mut server = init_tool_server_simple();

    let response = call_server(&mut server, "unknown_method", serde_json::json!({}))
        .await
        .unwrap();

    match response {
        JsonRpcResponse::Success { .. } => {
            panic!("Expected error response");
        }
        JsonRpcResponse::Error { error, .. } => {
            assert_eq!(error.code.code(), -32601);
        }
    }
}
