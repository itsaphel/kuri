use crate::transport::{MessageParseError, TransportError};
use futures::{SinkExt, StreamExt};
use kuri_mcp_protocol::jsonrpc::{
    ErrorCode, ErrorData, Request, RequestId, Response, ResponseItem,
};
use std::convert::Infallible;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::codec::{Framed, LinesCodec, LinesCodecError};
use tower::Service;

#[inline]
fn parse_message(line: Result<String, LinesCodecError>) -> Result<Request, MessageParseError> {
    let line = line?;
    serde_json::from_str::<Request>(&line).map_err(MessageParseError::Deserialisation)
}

/// Write a JSON-RPC response on the transport.
#[inline]
async fn write_message<T>(
    frame: &mut Framed<T, LinesCodec>,
    msg: Response,
) -> Result<(), TransportError>
where
    T: AsyncWrite + Unpin,
{
    let json = serde_json::to_string(&msg)?;
    frame.send(json).await?;
    Ok(())
}

async fn handle_connection<S, T>(mut service: S, transport: T) -> Result<(), TransportError>
where
    S: Service<Request, Response = Response, Error = Infallible>,
    T: AsyncRead + AsyncWrite + Unpin,
{
    // nb: buffer is 8kb (tokio internals)
    // TODO: consider a max length for lines
    let mut frame = Framed::new(transport, LinesCodec::new());

    // Process the stream in lines indefinitely, until the connection closes
    while let Some(line) = frame.next().await {
        match parse_message(line) {
            Ok(message) => {
                // Process the message
                let response = service
                    .call(message)
                    .await
                    .expect("MCPService is infallible");
                if !response.is_empty() {
                    // Write the response, if needed
                    if let Err(e) = write_message(&mut frame, response).await {
                        tracing::error!(error = ?e, "Error writing response over transport");
                    }
                }
            }
            Err(e) => {
                // per JSON-RPC spec, we should respond with an "Invalid Request" error
                // see: https://www.jsonrpc.org/specification#examples
                match e {
                    MessageParseError::Deserialisation(_) => {
                        let error_data = ErrorData::new(
                            ErrorCode::ParseError,
                            "JSON parsing error when deserialising the message".to_string(),
                        );
                        let msg = ResponseItem::error(RequestId::Null, error_data);
                        write_message(&mut frame, Response::Single(Some(msg))).await?;
                        tracing::debug!(error = ?e, "Transport error (deserialisation)");
                    }
                    MessageParseError::LinesCodecError(_) => {
                        // Transport error. But don't terminate the connection: we continue looping
                        tracing::error!(error = ?e, "Transport error");
                    }
                }
            }
        }
    }

    Ok(())
}

/// Serve a MCP Service over a transport layer.
pub async fn serve<S, T>(service: S, transport: T) -> Result<(), TransportError>
where
    S: Service<Request, Response = Response, Error = Infallible> + Clone + 'static,
    T: AsyncRead + AsyncWrite + Unpin,
{
    // TODO: Currently no ability to handle multiple connections.
    handle_connection(service, transport).await
}
