use crate::context::Context;
use async_trait::async_trait;
use kuri_mcp_protocol::{
    messages::CallToolResult,
    prompt::{PromptArgument, PromptError},
    tool::ToolError,
};
use serde_json::Value;
use std::collections::HashMap;

#[async_trait(?Send)]
pub trait ToolHandler: 'static {
    /// The name of the tool
    fn name(&self) -> &'static str;

    /// A description of what the tool does
    fn description(&self) -> &'static str;

    /// JSON schema describing the tool's parameters
    fn schema(&self) -> Value;

    /// Execute the tool with the given parameters
    async fn call(&self, context: &Context, params: Value) -> Result<CallToolResult, ToolError>;
}

#[async_trait(?Send)]
pub trait PromptHandler: 'static {
    fn name(&self) -> &'static str;
    fn description(&self) -> Option<&'static str>;
    fn arguments(&self) -> Option<Vec<PromptArgument>>;
    async fn call(
        &self,
        context: &Context,
        params: HashMap<String, serde_json::Value>,
    ) -> Result<String, PromptError>;
}

#[cfg(test)]
mod tests {
    use crate::response::IntoCallToolResult;
    use kuri_mcp_protocol::Content;

    use super::*;

    #[tokio::test]
    async fn test_echo_tool() {
        let tool = EchoTool;
        let result = tool
            .call(&Context::default(), serde_json::json!({"input": "hello"}))
            .await
            .unwrap();
        let content = result.content[0].clone();
        assert_eq!(content, Content::text("hello"));
    }

    // --- Test tool implementation ---
    struct EchoTool;

    async fn echo_tool(input: String) -> String {
        input
    }

    #[async_trait(?Send)]
    impl ToolHandler for EchoTool {
        fn name(&self) -> &'static str {
            "echo"
        }

        fn description(&self) -> &'static str {
            "Echo the input"
        }

        fn schema(&self) -> Value {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "input": {
                        "type": "string",
                        "description": "The text to echo back"
                    }
                },
                "required": ["input"]
            })
        }

        #[allow(unused_variables)]
        async fn call(
            &self,
            context: &Context,
            params: Value,
        ) -> Result<CallToolResult, ToolError> {
            let input = params.get("input").unwrap().as_str().unwrap();
            echo_tool(input.to_string()).await.into_call_tool_result()
        }
    }
}
