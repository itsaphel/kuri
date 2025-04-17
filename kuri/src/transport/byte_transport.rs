// This file is derived from goose, which is licensed under the MIT license.
// Original: https://github.com/block/goose/blob/66bfcc0e553a84d6e93613140bad3e2fad577486/crates/mcp-server/src/lib.rs

use futures::{Future, Stream};
use kuri_mcp_protocol::jsonrpc::{JsonRpcResponse, SendableMessage};
use pin_project::pin_project;
use std::{
    pin::Pin,
    task::{Context, Poll},
};
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};

use super::TransportError;

/// A transport layer that handles JSON-RPC messages over byte streams.
#[pin_project]
pub struct ByteTransport<R, W> {
    #[pin]
    reader: BufReader<R>,
    #[pin]
    writer: W,
}

impl<R, W> ByteTransport<R, W>
where
    R: AsyncRead,
    W: AsyncWrite,
{
    pub fn new(reader: R, writer: W) -> Self {
        Self {
            // TODO: Rethink capacity
            // Default BufReader capacity is 8 * 1024, increase this to 2MB to the file size limit
            // allows the buffer to have the capacity to read very large calls
            reader: BufReader::with_capacity(2 * 1024 * 1024, reader),
            writer,
        }
    }
}

/// Parse a message from a byte buffer.
///
/// Returns an error if the buffer is not valid UTF-8, or if the message is not a valid JSON-RPC
/// message.
fn parse_message(buf: Vec<u8>) -> Result<SendableMessage, TransportError> {
    // Convert to UTF-8 string
    let line = match String::from_utf8(buf) {
        Ok(s) => s,
        Err(e) => return Err(TransportError::Utf8(e)),
    };
    // Parse JSON and validate message format
    match serde_json::from_str::<serde_json::Value>(&line) {
        Ok(value) => {
            // Validate basic JSON-RPC structure
            if !value.is_object() {
                return Err(TransportError::InvalidMessage(
                    "Message must be a JSON object".into(),
                ));
            }
            let obj = value.as_object().unwrap(); // Safe due to check above

            // Check jsonrpc version field
            if !obj.contains_key("jsonrpc") || obj["jsonrpc"] != "2.0" {
                return Err(TransportError::InvalidMessage(
                    "Missing or invalid jsonrpc version".into(),
                ));
            }

            // Now try to parse as proper message
            match serde_json::from_value::<SendableMessage>(value) {
                Ok(msg) => Ok(msg),
                Err(e) => Err(TransportError::Serialisation(e)),
            }
        }
        Err(e) => Err(TransportError::Serialisation(e)),
    }
}

impl<R, W> Stream for ByteTransport<R, W>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    type Item = Result<SendableMessage, TransportError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        let mut buf = Vec::new();

        let mut reader = this.reader.as_mut();
        let mut read_future = Box::pin(reader.read_until(b'\n', &mut buf));
        match read_future.as_mut().poll(cx) {
            Poll::Ready(Ok(0)) => Poll::Ready(None), // EOF (connection closed)
            Poll::Ready(Ok(_)) => match parse_message(buf) {
                Ok(msg) => Poll::Ready(Some(Ok(msg))),
                Err(e) => Poll::Ready(Some(Err(e))),
            },
            Poll::Ready(Err(e)) => Poll::Ready(Some(Err(TransportError::Io(e)))),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<R, W> ByteTransport<R, W>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    pub async fn write_message(
        self: &mut Pin<&mut Self>,
        msg: JsonRpcResponse,
    ) -> Result<(), std::io::Error> {
        let json = serde_json::to_string(&msg)?;

        let mut this = self.as_mut().project();
        this.writer.write_all(json.as_bytes()).await?;
        this.writer.write_all(b"\n").await?;
        this.writer.flush().await?;
        Ok(())
    }
}
