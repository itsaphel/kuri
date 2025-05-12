use kuri_mcp_protocol::{messages::CallToolResult, tool::ToolError, Content};
use std::fmt;

/// Trait for generating tool responses.
///
/// Ultimately, the MCP protocol requires `call_tool` invocations to return a list of [`Content`]s.
/// However, various types of tool responses that match some kind of [`Content`] are possible. This
/// traits improves the ergonomics of generating different kinds of tool responses, and provides
/// type-safety when doing so.
///
/// Tool handlers must return a value that implements this trait.
///
/// You may implement this trait for your own types, defining the conversion to [`Content`]s. This
/// allows for ergonomic returning of things like images and audio, without needing to do base64
/// conversion within the handler.
///
/// The Err variant (`ToolError`) handles mappings to JSON-RPC error responses (which are used
/// for cases like invalid parameters or schema errors). Execution errors are part of the Ok
/// variant, and represented within the `CallToolResult`. For more, see the specification on
/// [tool error handling].
///
/// [tool error handling]: https://modelcontextprotocol.io/specification/2025-03-26/server/tools#error-handling
pub trait IntoCallToolResult {
    /// Create a `CallToolResult` from the current type.
    fn into_call_tool_result(self) -> Result<CallToolResult, ToolError>;
}

/// Helper function to create a successful CallToolResult with a single text content
fn successful_text_response<S: Into<String>>(text: S) -> Result<CallToolResult, ToolError> {
    Ok(CallToolResult {
        content: vec![Content::text(text)],
        is_error: false,
    })
}

// Support `IntoCallToolResult` for various primitive types that can be converted to a String
// Use a macro as this would otherwise be hundreds of lines.
macro_rules! impl_into_call_tool_result_for_to_string {
    ($($t:ty),*) => {
        $(
            impl IntoCallToolResult for $t {
                fn into_call_tool_result(self) -> Result<CallToolResult, ToolError> {
                    successful_text_response(self.to_string())
                }
            }
        )*
    };
}

impl_into_call_tool_result_for_to_string!(
    String, i8, u8, i16, u16, i32, u32, i64, u64, f32, f64, bool
);

impl IntoCallToolResult for Vec<Content> {
    fn into_call_tool_result(self) -> Result<CallToolResult, ToolError> {
        Ok(CallToolResult {
            content: self,
            is_error: false,
        })
    }
}

impl IntoCallToolResult for () {
    fn into_call_tool_result(self) -> Result<CallToolResult, ToolError> {
        Ok(CallToolResult {
            content: vec![],
            is_error: false,
        })
    }
}

/// Handler returns a Result<T, ToolError>
///
/// ExecutionErrors are converted into an error `CallToolResponse`, while other cases are
/// propagated up to the transport.
impl<T> IntoCallToolResult for Result<T, ToolError>
where
    T: IntoCallToolResult,
{
    fn into_call_tool_result(self) -> Result<CallToolResult, ToolError> {
        match self {
            Ok(value) => value.into_call_tool_result(),
            Err(err) => match err {
                // Map ExecutionError to Ok result with error content
                ToolError::ExecutionError(msg) => Ok(CallToolResult {
                    content: vec![Content::text(format!("Error: {}", msg))],
                    is_error: true,
                }),
                // Propagate other ToolError variants directly
                other_err => Err(other_err),
            },
        }
    }
}

/// Handler returns a Result<T, S>, where S implements Display
/// We treat these as logical errors.
impl<T> IntoCallToolResult for Result<T, DisplayableError>
where
    T: IntoCallToolResult,
{
    fn into_call_tool_result(self) -> Result<CallToolResult, ToolError> {
        match self {
            Ok(value) => value.into_call_tool_result(),
            Err(err) => Ok(CallToolResult {
                content: vec![Content::text(err.to_string())],
                is_error: true,
            }),
        }
    }
}

#[derive(Debug)]
pub struct DisplayableError(String);

impl fmt::Display for DisplayableError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Allow converting anything `Into<String>` into a `DisplayableError`
impl<E: Into<String>> From<E> for DisplayableError {
    fn from(err: E) -> Self {
        DisplayableError(err.into())
    }
}
