use std::convert::Infallible;

use kuri_mcp_protocol::jsonrpc::{ResponseItem, SendableMessage};
use tower::Service;

use crate::MCPRequestService;

/// Extension trait that adds additional methods to any [`Service`] that processes MCP messages.
pub trait ServiceExt<R>: Service<R> + Sized {
    /// Convert this service into a [`MCPRequestService`], which processes a single MCP request.
    fn into_request_service(self) -> MCPRequestService<Self>;
}

impl<S> ServiceExt<SendableMessage> for S
where
    S: Service<SendableMessage, Response = Option<ResponseItem>, Error = Infallible>
        + Sized
        + Clone
        + 'static,
{
    fn into_request_service(self) -> MCPRequestService<Self> {
        MCPRequestService::new(self)
    }
}
