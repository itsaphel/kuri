use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf, Stdin, Stdout};

/// A transport that combines stdin and stdout.
///
/// The implementations rely on the fact that Stdin and Stdout are Unpin.
#[derive(Debug)]
pub struct StdioTransport {
    input: Stdin,
    output: Stdout,
}

impl StdioTransport {
    /// Creates a new StdioTransport using tokio's stdin and stdout.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        StdioTransport {
            input: tokio::io::stdin(),
            output: tokio::io::stdout(),
        }
    }
}

impl AsyncRead for StdioTransport {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.input).poll_read(cx, buf)
    }
}

impl AsyncWrite for StdioTransport {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        Pin::new(&mut self.output).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        Pin::new(&mut self.output).poll_flush(cx)
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), io::Error>> {
        Pin::new(&mut self.output).poll_shutdown(cx)
    }
}

// allows converting Pin<&mut Self> to &mut Self via get_mut()
// can be implemented because Stdin and Stdout are Unpin
impl Unpin for StdioTransport {}
