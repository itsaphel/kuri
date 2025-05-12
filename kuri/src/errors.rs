use thiserror::Error;

/// Errors raised while *processing* a request.
/// These errors assume that the request is valid and was successfully parsed. Errors for invalid
/// requests are handled at the transport level, within [`MessageParseError`].
///
/// [`MessageParseError`]: crate::transport::MessageParseError
#[derive(Error, Debug)]
pub enum RequestError {
    #[error("Method not found: {0}")]
    MethodNotFound(String),

    #[error("Invalid parameters: {0}")]
    InvalidParams(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    #[error("Resource not found: {0}")]
    ResourceNotFound(String),

    #[error("Not found: {0}")]
    PromptNotFound(String),

    #[error("This implementation doesn't support message type: {0}")]
    Unsupported(String),
}

/// Request errors can be returned as a `JsonRpcResponse` with the error type.
/// This trait implementation aids conversion of the `RequestError` to an `ErrorData` which can be
/// provided in the `JsonRpcResponse`.
impl From<RequestError> for kuri_mcp_protocol::jsonrpc::ErrorData {
    fn from(err: RequestError) -> Self {
        use kuri_mcp_protocol::jsonrpc::{ErrorCode, ErrorData};

        let code = match err {
            RequestError::MethodNotFound(_) => ErrorCode::MethodNotFound,
            RequestError::InvalidParams(_) => ErrorCode::InvalidParams,
            RequestError::Internal(_) => ErrorCode::InternalError,
            RequestError::ToolNotFound(_) => ErrorCode::InvalidParams,
            RequestError::ResourceNotFound(_) => ErrorCode::InvalidParams,
            RequestError::PromptNotFound(_) => ErrorCode::InvalidParams,
            RequestError::Unsupported(_) => ErrorCode::InvalidRequest,
        };

        ErrorData::new(code, err.to_string())
    }
}

impl From<kuri_mcp_protocol::resource::ResourceError> for RequestError {
    fn from(err: kuri_mcp_protocol::resource::ResourceError) -> Self {
        match err {
            kuri_mcp_protocol::resource::ResourceError::NotFound(msg) => {
                RequestError::ResourceNotFound(msg)
            }
            _ => RequestError::Internal(format!("Unknown resource error: {}", err)),
        }
    }
}

impl From<kuri_mcp_protocol::tool::ToolError> for RequestError {
    fn from(err: kuri_mcp_protocol::tool::ToolError) -> Self {
        match err {
            kuri_mcp_protocol::tool::ToolError::NotFound(msg) => RequestError::ToolNotFound(msg),
            kuri_mcp_protocol::tool::ToolError::InvalidParameters(msg) => {
                RequestError::InvalidParams(msg)
            }
            kuri_mcp_protocol::tool::ToolError::SchemaError(msg) => {
                RequestError::InvalidParams(msg)
            }
            kuri_mcp_protocol::tool::ToolError::ExecutionError(_) => {
                // This case should've been mapped to a successful result.
                unreachable!()
            }
        }
    }
}
