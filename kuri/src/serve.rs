use crate::{errors::ServerError, transport::TransportError};
use futures::{SinkExt, StreamExt};
use kuri_mcp_protocol::jsonrpc::{JsonRpcResponse, SendableMessage};
use std::convert::Infallible;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::codec::{Framed, LinesCodec, LinesCodecError};
use tower::Service;

/// Ensure the JSON value is a valid JSON-RPC 2.0 message.
fn validate_jsonrpc_message(value: &serde_json::Value) -> Result<(), TransportError> {
    if !value.is_object() {
        return Err(TransportError::InvalidMessage(
            "Message must be a JSON object".to_string(),
        ));
    }
    // Safe due to check above
    let obj = value.as_object().unwrap();

    // Check JSON-RPC version field
    if !obj.contains_key("jsonrpc") || obj["jsonrpc"] != "2.0" {
        return Err(TransportError::InvalidMessage(
            "Missing or invalid JSON-RPC version".to_string(),
        ));
    }

    Ok(())
}

// Parse a line into a SendableMessage
async fn parse_message(
    line: Result<String, LinesCodecError>,
) -> Result<SendableMessage, TransportError> {
    let line = line?;
    let value =
        serde_json::from_str::<serde_json::Value>(&line).map_err(TransportError::Serialisation)?;

    validate_jsonrpc_message(&value)?;

    serde_json::from_value::<SendableMessage>(value).map_err(TransportError::Serialisation)
}

/// Write a JSON-RPC response on the transport.
async fn write_message<T>(
    frame: &mut Framed<T, LinesCodec>,
    msg: JsonRpcResponse,
) -> Result<(), TransportError>
where
    T: AsyncWrite + Unpin,
{
    let json = serde_json::to_string(&msg)?;
    frame.send(json).await?;
    Ok(())
}

/// Process a single message, calling the service and handling responses.
///
/// An error can only occur when we're writing a response to the transport (assuming a response is needed).
async fn process_message<S, T>(
    service: &mut S,
    frame: &mut Framed<T, LinesCodec>,
    message: SendableMessage,
) -> Result<(), TransportError>
where
    S: Service<SendableMessage, Response = Option<JsonRpcResponse>, Error = Infallible>,
    T: AsyncWrite + Unpin,
{
    match message {
        SendableMessage::Request(request) => {
            let response = service
                .call(SendableMessage::from(request))
                .await
                .expect("MCPService cannot return an error");

            // Send response if available
            if let Some(response) = response {
                write_message(frame, response).await?;
            }
        }
        SendableMessage::Notification(notification) => {
            service
                .call(SendableMessage::from(notification))
                .await
                .expect("MCPService cannot return an error");
        }
    }
    Ok(())
}

async fn handle_connection<S, T>(mut service: S, transport: T) -> Result<(), ServerError>
where
    S: Service<SendableMessage, Response = Option<JsonRpcResponse>, Error = Infallible>,
    T: AsyncRead + AsyncWrite + Unpin,
{
    // nb: buffer is 8kb (tokio internals)
    // TODO: consider a max length for lines
    let mut frame = Framed::new(transport, LinesCodec::new());

    // Process the stream in lines indefinitely, until the connection closes
    while let Some(line) = frame.next().await {
        match parse_message(line).await {
            Ok(message) => {
                if let Err(e) = process_message(&mut service, &mut frame, message).await {
                    tracing::error!(error = ?e, "Error processing message");
                }
            }
            Err(e) => {
                // log an error, but don't terminate the connection; we continue looping
                if matches!(e, TransportError::Serialisation(_)) {
                    tracing::debug!(error = ?e, "Transport error (serialization)");
                } else {
                    tracing::error!(error = ?e, "Transport error");
                }
            }
        }
    }

    Ok(())
}

/// Serve a MCP Service over a transport layer.
pub async fn serve<S, T>(service: S, transport: T) -> Result<(), ServerError>
where
    S: Service<SendableMessage, Response = Option<JsonRpcResponse>, Error = Infallible>,
    T: AsyncRead + AsyncWrite + Unpin,
{
    // TODO: Currently no ability to handle multiple connections.
    handle_connection(service, transport).await
}
