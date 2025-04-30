use thiserror::Error;

/// Errors raised when parsing a message
#[derive(Error, Debug)]
pub enum MessageParseError {
    #[error("Message is not a JSON-RPC 2.0 message")]
    NotJsonRpc2Message,

    #[error("Error deserialising message: {0}")]
    Deserialisation(#[from] serde_json::Error),

    #[error("Error decoding line: {0}")]
    LinesCodecError(#[from] tokio_util::codec::LinesCodecError),
}

/// Errors raised by a transport.
///
/// The most common case is when reading from, or writing to, a connection.
#[derive(Error, Debug)]
pub enum TransportError {
    #[error("JSON serialisation error: {0}")]
    Serialisation(#[from] serde_json::Error),

    #[error("Error sending/receiving bytes: {0}")]
    LinesCodecError(#[from] tokio_util::codec::LinesCodecError),
}

mod stdio;
pub use stdio::StdioTransport;
