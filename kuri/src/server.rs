use crate::{
    errors::ServerError,
    transport::{ByteTransport, TransportError},
};
use kuri_mcp_protocol::jsonrpc::{JsonRpcResponse, SendableMessage};
use std::{convert::Infallible, pin::Pin};
use tokio::io::{AsyncRead, AsyncWrite};
use tower::Service;

/// The main server type that processes incoming requests in a loop, and middlemans communication
/// with the transport layer.
pub struct Server<S> {
    /// A Tower Service that can handle/process MCP messages, and return MCP responses. This Service
    /// may be enhanced using tower layers (for middleware).
    service: S,
}

fn trace_response(response: &Option<JsonRpcResponse>) {
    let response_json = serde_json::to_string(&response)
        .unwrap_or_else(|_| "Failed to serialize response".to_string());
    tracing::debug!(
        json = %response_json,
        "Sending response"
    );
}

impl<S> Server<S>
where
    S: Service<SendableMessage, Response = Option<JsonRpcResponse>, Error = Infallible>,
{
    pub fn new(service: S) -> Self {
        Self { service }
    }

    // TODO: Consider pushing tracing into middleware, eg https://docs.rs/tower-http/latest/tower_http/trace/index.html
    /// Process a JSON-RPC message received by the transport layer.
    #[tracing::instrument(level = "trace", fields(request_id, method), skip(self, transport))]
    async fn process_message<R, W>(
        &mut self,
        transport: &mut Pin<&mut ByteTransport<R, W>>,
        msg_result: Result<SendableMessage, TransportError>,
    ) -> Result<(), ServerError>
    where
        R: AsyncRead + Unpin,
        W: AsyncWrite + Unpin,
    {
        use valuable::Valuable;

        match msg_result {
            Ok(SendableMessage::Request(request)) => {
                let id = request.id.clone();
                tracing::Span::current().record("request_id", id.as_value());
                tracing::Span::current().record("method", &request.method);

                // Process the request
                let response = self
                    .service
                    .call(SendableMessage::from(request))
                    .await
                    .expect("MCPService cannot return an error.");

                trace_response(&response);

                // If there is a response, send it over the transport
                if let Some(response) = response {
                    transport
                        .write_message(response)
                        .await
                        .map_err(|e| ServerError::Transport(TransportError::Io(e)))?;
                }
            }
            Ok(SendableMessage::Notification(notification)) => {
                tracing::Span::current().record("method", &notification.method);

                // Process the notification
                self.service
                    .call(SendableMessage::from(notification))
                    .await
                    .expect("MCPService cannot return an error.");
            }
            Err(e) => {
                // Transport errors are just logged. No response is sent to the client.
                // TODO: Not all transport errors problematic (eg serialisation), so maybe reduce log level.
                tracing::error!(error = ?e, "Transport error");
            }
        }
        Ok(())
    }

    /// Run the server.
    ///
    /// Accepts a transport layer over which the JSON-RPC messages are received and written.
    pub async fn run<R, W>(mut self, mut transport: ByteTransport<R, W>) -> Result<(), ServerError>
    where
        R: AsyncRead + Unpin,
        W: AsyncWrite + Unpin,
    {
        use futures::StreamExt;
        let mut transport = Pin::new(&mut transport);

        tracing::info!("Server started");

        // Loop until the transport is closed. The transport returns Ok(None) _iff_ it closes
        while let Some(msg_result) = transport.next().await {
            // TODO: Perhaps spawn a tokio task to process the message?
            self.process_message(&mut transport, msg_result).await?;
        }

        Ok(())
    }
}

// todo: consider `axum::serve`
