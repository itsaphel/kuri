use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

/// Metadata to describe a tool exposed by the MCP server.
/// A tool is like an RPC method and can be called by a model, either to fetch data, or perform
/// side effects.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tool {
    /// The name of the tool
    pub name: String,
    /// A description of what the tool does
    pub description: String,
    /// A JSON Schema object defining the expected parameters and the return format
    pub input_schema: Value,
}

impl Tool {
    /// Create a new tool with the given name and description
    pub fn new<N, D>(name: N, description: D, input_schema: Value) -> Self
    where
        N: Into<String>,
        D: Into<String>,
    {
        Tool {
            name: name.into(),
            description: description.into(),
            input_schema,
        }
    }
}

pub type ToolResult<T> = Result<T, ToolError>;

/// Errors that can be raised by a tool handler.
#[derive(Error, Debug, Clone, Deserialize, Serialize, PartialEq)]
pub enum ToolError {
    #[error("Invalid parameters: {0}")]
    InvalidParameters(String),
    #[error("Execution failed: {0}")]
    ExecutionError(String),
    #[error("Schema error: {0}")]
    SchemaError(String),
    #[error("Tool not found: {0}")]
    NotFound(String),
}

/// Helper function to generate JSON schema for a type
pub fn generate_tool_schema<T: JsonSchema>() -> ToolResult<Value> {
    let schema = schemars::schema_for!(T);
    serde_json::to_value(schema).map_err(|e| ToolError::SchemaError(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(dead_code)]
    #[derive(serde::Deserialize, schemars::JsonSchema)]
    struct CalculatorParameters {
        #[schemars(description = "First number in the calculation")]
        x: i32,
        #[schemars(description = "Second number in the calculation")]
        y: i32,
        #[schemars(description = "The operation to perform (add, subtract, multiply, divide)")]
        operation: String,
    }

    #[test]
    fn test_generate_schema() {
        let schema = generate_tool_schema::<CalculatorParameters>();
        assert!(schema.is_ok());
        let actual = schema.unwrap();
        // TODO: Unfortunate that `title` is included.
        let expected = serde_json::json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "properties": {
                "operation": { "type": "string", "description": "The operation to perform (add, subtract, multiply, divide)" },
                "x": { "format": "int32", "type": "integer", "description": "First number in the calculation" },
                "y": { "format": "int32", "type": "integer", "description": "Second number in the calculation" },
            },
            "required": ["operation", "x", "y"],
            "title": "CalculatorParameters",
        });
        assert_eq!(actual, expected);
    }
}
