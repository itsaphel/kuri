use async_trait::async_trait;
use thiserror::Error;

use kuri_mcp_protocol::jsonrpc::{JsonRpcResponse, SendableMessage};

/// A generic asynchronous transport trait, used to abstract over the underlying transport mechanism.
///
/// The transport can be started and closed. Starting the transport returns a handle, which can be
/// used to send messages over the transport.
///
/// Any logic needed to start or initialise the transport should be done by the user.
#[async_trait]
pub trait Transport {
    /// Send a message over the transport.
    ///
    /// The SendableMessage may be either a JSON-RPC request or a notification.
    /// For requests, a `JsonRpcResponse` (or error) is returned. For notifications, there is no
    /// response if the request is successful.
    async fn send(
        &self,
        message: SendableMessage,
    ) -> Result<Option<JsonRpcResponse>, TransportError>;

    // /// Receive a message from the transport. This will block the task until a message is available,
    // /// or the transport is closed.
    // // TODO: Integrate with StreamExt
    // async fn receive(&self) -> Result<Option<SendableMessage>, TransportError>;
}

/// Errors raised by a transport
#[derive(Error, Debug)]
pub enum TransportError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON serialisation error: {0}")]
    Serialisation(#[from] serde_json::Error),

    #[error("Invalid UTF-8 sequence: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),

    #[error("Invalid message format: {0}")]
    InvalidMessage(String),

    #[error("Channel closed")]
    ChannelClosed,

    #[error("Stdio process error: {0}")]
    StdioProcessError(String),

    #[error("Transport unavailable (either closed or not started)")]
    Unavailable,
}

mod byte_transport;
pub use byte_transport::ByteTransport;
