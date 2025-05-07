use kuri::MCPService;
use kuri_mcp_protocol::jsonrpc::{MethodCall, Params, RequestId, ResponseItem, SendableMessage};
use tower::Service;

pub async fn call_server(
    server: &mut MCPService,
    method: &str,
    params: serde_json::Value,
) -> Option<ResponseItem> {
    let params = match params {
        serde_json::Value::Object(map) => Some(Params::Map(map)),
        serde_json::Value::Array(array) => Some(Params::Array(array)),
        _ => None,
    };

    let request = MethodCall::new(RequestId::Num(1), method.to_string(), params);
    let future = server.call(SendableMessage::from(request));

    future.await.unwrap()
}
