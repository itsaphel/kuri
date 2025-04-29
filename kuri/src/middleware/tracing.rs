use std::{
    convert::Infallible,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use kuri_mcp_protocol::jsonrpc::{JsonRpcResponse, SendableMessage};
use tower::{Layer, Service};
use tracing::Level;

const DEFAULT_TRACE_LEVEL: Level = Level::DEBUG;

/// A service that logs incoming MCP messages
pub struct TracingService<S> {
    inner: S,
}

impl<S> Service<SendableMessage> for TracingService<S>
where
    S: Service<SendableMessage, Response = Option<JsonRpcResponse>, Error = Infallible>,
    S::Future: 'static,
{
    type Response = Option<JsonRpcResponse>;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: SendableMessage) -> Self::Future {
        let method = match &req {
            SendableMessage::Request(req) => &req.method,
            SendableMessage::Notification(req) => &req.method,
        };
        let params = match &req {
            SendableMessage::Request(req) => &req.params,
            SendableMessage::Notification(req) => &req.params,
        };
        let span = tracing::span!(
            DEFAULT_TRACE_LEVEL,
            "request",
            method = method,
            params = ?params
        );

        let future = {
            let _guard = span.enter();
            self.inner.call(req)
        };

        Box::pin(future)
    }
}

/// A layer that wraps services with tracing functionality
#[derive(Clone, Default)]
pub struct TracingLayer;

impl TracingLayer {
    /// Create a new tracing layer
    pub fn new() -> Self {
        Self
    }
}

impl<S> Layer<S> for TracingLayer
where
    S: Service<SendableMessage, Response = Option<JsonRpcResponse>, Error = Infallible>,
    S::Future: 'static,
{
    type Service = TracingService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        TracingService { inner }
    }
}
