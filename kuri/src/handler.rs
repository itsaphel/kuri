use std::collections::HashMap;

use crate::context::Context;
use async_trait::async_trait;
use kuri_mcp_protocol::{
    prompt::{PromptArgument, PromptError},
    tool::ToolResult,
};
use serde_json::Value;

#[async_trait(?Send)]
pub trait ToolHandler: 'static {
    /// The name of the tool
    fn name(&self) -> &'static str;
    /// A description of what the tool does
    fn description(&self) -> &'static str;
    /// JSON schema describing the tool's parameters
    fn schema(&self) -> Value;
    /// Execute the tool with the given parameters
    async fn call(&self, context: &Context, params: Value) -> ToolResult<Value>;
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
