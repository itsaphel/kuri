mod common;

use common::call_server;
use kuri::{tool, MCPService, MCPServiceBuilder};
use kuri_mcp_protocol::{
    jsonrpc::{RequestId, ResponseItem},
    messages::{Implementation, InitializeResult, ServerCapabilities, ToolsCapability},
};
use tracing_subscriber::EnvFilter;

// Utility (ping): https://spec.modelcontextprotocol.io/specification/2025-03-26/basic/utilities/ping/
#[tokio::test]
async fn test_ping() {
    let mut server = init_simple_server();

    let response = call_server(&mut server, "ping", serde_json::json!({}))
        .await
        .unwrap();

    match response {
        ResponseItem::Success { result, .. } => {
            assert_eq!(result, serde_json::json!({}));
        }
        ResponseItem::Error { .. } => {
            panic!("Expected success response");
        }
    }
}

// Client initialisation
// Spec: https://spec.modelcontextprotocol.io/specification/2025-03-26/basic/lifecycle/#initialization
#[tokio::test]
async fn test_initialize() {
    let mut server = init_simple_server();

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
        ResponseItem::Success { id, result, .. } => {
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
                    name: "Simple server".to_string(),
                    version: "0.1.0".to_string(),
                },
                instructions: None,
            };
            assert_eq!(actual, expected);
        }
        ResponseItem::Error { .. } => {
            panic!("Expected success response");
        }
    }
}

// General server and JSON-RPC tests

#[tokio::test]
async fn test_unknown_method() {
    let mut server = init_simple_server();

    let response = call_server(&mut server, "unknown_method", serde_json::json!({}))
        .await
        .unwrap();

    match response {
        ResponseItem::Success { .. } => {
            panic!("Expected error response");
        }
        ResponseItem::Error { error, .. } => {
            assert_eq!(error.code.code(), -32601);
        }
    }
}

#[tool]
async fn hello_world_tool(int: i32) -> String {
    format!("Hello, {}!", int)
}

pub fn init_simple_server() -> MCPService {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_test_writer()
        .try_init();

    MCPServiceBuilder::new("Simple server".to_string())
        .with_tool(HelloWorldTool)
        .build()
}
