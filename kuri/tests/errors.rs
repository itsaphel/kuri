#[allow(unused)]
mod common;

use common::init_tool_server_simple;
use kuri::{serve, ServiceExt};
use std::{
    io,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll},
};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

macro_rules! assert_json_eq {
    ($actual:expr, $expected:expr) => {
        let actual = serde_json::from_str::<serde_json::Value>($actual)
            .map(|v| v.to_string())
            .unwrap_or_else(|_| $actual.to_string());
        let expected = serde_json::from_str::<serde_json::Value>($expected)
            .map(|v| v.to_string())
            .unwrap_or_else(|_| $expected.to_string());
        assert_eq!(actual, expected);
    };
}

#[tokio::test]
async fn test_invalid_json() {
    let response = request(r#"{"jsonrpc": "2.0", "method": "foobar, "params": "bar", "baz]"#).await;
    assert_json_eq!(
        &response,
        r#"{"jsonrpc": "2.0", "error": {"code": -32700, "message": "JSON parsing error when deserialising the message"}, "id": null}"#
    );
}

#[tokio::test]
async fn test_invalid_request() {
    let response = request(r#"{}"#).await;
    assert_json_eq!(
        &response,
        r#"{"jsonrpc": "2.0", "error": {"code": -32600, "message": "Invalid request"}, "id": null}"#
    );

    let response = request(r#"{"jsonrpc": "2.0", "method": 1, "params": "bar"}"#).await;
    assert_json_eq!(
        &response,
        r#"{"jsonrpc":"2.0", "error": {"code": -32600, "message": "Invalid request"}, "id": null}"#
    );
}

#[tokio::test]
async fn test_method_not_found() {
    let response =
        request(r#"{"jsonrpc": "2.0", "method": "non_existent_method", "params": {}, "id": 1}"#)
            .await;
    assert_json_eq!(
        &response,
        r#"{"jsonrpc": "2.0", "error": {"code": -32601, "message": "Method not found: non_existent_method"}, "id": 1}"#
    );
}

#[tokio::test]
async fn test_logical_param_errors() {
    let response = request(
        r#"{"jsonrpc": "2.0", "method": "tools/call", "params": {"name": "calculator", "arguments": {"x": "not_a_number", "y": 2, "operation": "add"}}, "id": 1}"#,
    ).await;
    assert_json_eq!(
        &response,
        r#"{"jsonrpc": "2.0", "error": {"code": -32602, "message": "Invalid parameters: Missing or incorrect tool arguments"}, "id": 1}"#
    );

    let response = request(
        r#"{"jsonrpc": "2.0", "method": "tools/call", "params": {"name": "calculator", "arguments": {"x": 1, "y": 2, "operation": "invalid_operation"}}, "id": 1}"#,
    ).await;
    assert_json_eq!(
        &response,
        r#"{"jsonrpc": "2.0", "error": {"code": -32602, "message": "Invalid parameters: Unknown operation: invalid_operation"}, "id": 1}"#
    );
}

#[tokio::test]
async fn test_incorrect_jsonrpc_version() {
    // JSON-RPC v1.0
    let response =
        request(r#"{"jsonrpc": "1.0", "method": "initialize", "params": {}, "id": 1}"#).await;
    assert_json_eq!(
        &response,
        r#"{"jsonrpc": "2.0", "error": {"code": -32600, "message": "Invalid request"}, "id": 1}"#
    );

    // missing JSON-RPC version
    let response = request(r#"{"method": "initialize", "params": {}, "id": 1}"#).await;
    assert_json_eq!(
        &response,
        r#"{"jsonrpc": "2.0", "error": {"code": -32600, "message": "Invalid request"}, "id": 1}"#
    );
}

#[tokio::test]
async fn test_batch_no_valid_messages() {
    // Empty array
    let response = request(r#"[]"#).await;
    assert_json_eq!(
        &response,
        r#"{"jsonrpc": "2.0", "error": {"code": -32600, "message": "Invalid request: batch is empty"}, "id": null}"#
    );

    // Non-empty batch (one message), but no valid message
    let response = request(r#"[1]"#).await;
    assert_json_eq!(
        &response,
        r#"[{"jsonrpc": "2.0", "error": {"code": -32600, "message": "Invalid request"}, "id": null}]"#
    );

    // Non-empty batch (multiple messages), but no valid message
    let response = request(r#"[1,2]"#).await;
    assert_json_eq!(
        &response,
        r#"[{"jsonrpc": "2.0", "error": {"code": -32600, "message": "Invalid request"}, "id": null},{"jsonrpc": "2.0", "error": {"code": -32600, "message": "Invalid request"}, "id": null}]"#
    );
}

#[tokio::test]
async fn test_batch_invalid_json() {
    let response = request(
        r#"[{"jsonrpc": "2.0", "method": "sum", "params": [1,2,4], "id": "1"},{"jsonrpc": "2.0", "method"]"#,
    )
    .await;
    assert_json_eq!(
        &response,
        r#"{"jsonrpc": "2.0", "error": {"code": -32700, "message": "JSON parsing error when deserialising the message"}, "id": null}"#
    );
}

#[derive(Debug, Clone)]
struct MockTransport {
    read_buf: Vec<u8>,
    write_buf: Arc<Mutex<Vec<u8>>>,
    read_pos: usize,
}

impl MockTransport {
    fn new() -> Self {
        MockTransport {
            read_buf: Vec::new(),
            write_buf: Arc::new(Mutex::new(Vec::new())),
            read_pos: 0,
        }
    }

    fn set_read_buf(&mut self, data: &[u8]) {
        self.read_buf = data.to_vec();
        self.read_pos = 0;
    }

    fn get_write_buf(&self) -> Vec<u8> {
        self.write_buf.lock().unwrap().clone()
    }
}

impl AsyncRead for MockTransport {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        if self.read_pos >= self.read_buf.len() {
            return Poll::Ready(Ok(()));
        }

        let len = std::cmp::min(buf.remaining(), self.read_buf.len() - self.read_pos);
        if len > 0 {
            buf.put_slice(&self.read_buf[self.read_pos..self.read_pos + len]);
            self.read_pos += len;
        }
        Poll::Ready(Ok(()))
    }
}

impl AsyncWrite for MockTransport {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        self.write_buf.lock().unwrap().extend_from_slice(buf);
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

impl Unpin for MockTransport {}

async fn request(input: &str) -> String {
    let service = init_tool_server_simple();
    let mut transport = MockTransport::new();
    transport.set_read_buf(format!("{}\n", input).as_bytes());

    let _ = serve(service.into_request_service(), transport.clone()).await;

    let response = transport.get_write_buf();
    let response_str = std::str::from_utf8(&response).unwrap();
    let lines: Vec<_> = response_str.lines().collect();

    assert_eq!(lines.len(), 1, "Expected exactly one line of response");

    lines[0].to_string()

    // TODO
    // // Assert it's serialisable (or not)
    // let response = serde_json::from_str::<JsonRpcResponse>(lines[0])
    //     .expect("No valid JSON-RPC response found");
}
