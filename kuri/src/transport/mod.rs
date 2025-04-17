use thiserror::Error;

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

    #[error("LinesCodecError error: {0}")]
    LinesCodecError(#[from] tokio_util::codec::LinesCodecError),
}

mod stdio;
pub use stdio::StdioTransport;
