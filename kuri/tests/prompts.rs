mod common;

use common::call_server;
use kuri::{prompt, MCPService, MCPServiceBuilder};
use kuri_mcp_protocol::{
    jsonrpc::{ErrorCode, RequestId, ResponseItem},
    messages::{GetPromptResult, ListPromptsResult},
};
use tracing_subscriber::EnvFilter;

// Prompt tests
// Spec: https://spec.modelcontextprotocol.io/specification/2025-03-26/server/prompts/

#[tokio::test]
async fn test_prompts_list() {
    let mut server = init_prompt_server();

    let response = call_server(&mut server, "prompts/list", serde_json::json!({}))
        .await
        .unwrap();

    match response {
        ResponseItem::Success { id, result, .. } => {
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
        ResponseItem::Error { .. } => {
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
        ResponseItem::Success { id, result, .. } => {
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
        ResponseItem::Error { .. } => {
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
        ResponseItem::Error { id, error, .. } => {
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
        ResponseItem::Success { id, result, .. } => {
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
        ResponseItem::Error { .. } => {
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
        ResponseItem::Success { id, result, .. } => {
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
        ResponseItem::Error { .. } => {
            panic!("Expected success response");
        }
    }
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
