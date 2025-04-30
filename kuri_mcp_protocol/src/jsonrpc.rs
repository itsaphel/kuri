// The protocol defined here is largely spec compliant.
//
// Deviations:
// * No batching support
// * Requests and responses are assumed to be client-generated, not bi-directional.
use serde::{de, Deserialize, Serialize};
use serde_json::Value;
use valuable::Valuable;

const JSONRPC_VERSION: &str = "2.0";

/// This type represents messages that can be sent over the transport.
/// This can be used to ensure we don't initiate communication with a response type.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(untagged)]
pub enum SendableMessage {
    Request(JsonRpcRequest),
    Notification(JsonRpcNotification),
}

impl From<JsonRpcRequest> for SendableMessage {
    fn from(request: JsonRpcRequest) -> Self {
        SendableMessage::Request(request)
    }
}

impl From<JsonRpcNotification> for SendableMessage {
    fn from(notification: JsonRpcNotification) -> Self {
        SendableMessage::Notification(notification)
    }
}

/// Message ID, which according to the MCP spec must be either a number or a string.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash, Valuable)]
#[serde(untagged)]
pub enum RequestId {
    Num(u64),
    Str(String),
    /// No id (used for request errors and notifications)
    Null,
}

impl RequestId {
    #[inline]
    pub const fn is_null(&self) -> bool {
        matches!(self, RequestId::Null)
    }
}

/// Structured parameters which may be included in a request or notification.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(untagged)]
pub enum Params {
    /// Array of JSON values
    Array(Vec<Value>),
    /// Map of key-value pairs
    Map(serde_json::Map<String, Value>),
}

impl TryFrom<serde_json::Value> for Params {
    type Error = serde_json::Error;

    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        match value {
            Value::Array(vec) => Ok(Params::Array(vec)),
            Value::Object(map) => Ok(Params::Map(map)),
            _ => Err(de::Error::custom(format!(
                "JSON-RPC params must be either an array or object, got {:?}",
                value
            ))),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct JsonRpcRequest {
    jsonrpc: String,
    /// JSON-RPC spec: if `id` is ommitted, the request is assumed to be a notification.
    /// JSON-RPC permits this to be a "null" value, but MCP spec does not.
    pub id: RequestId,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Params>,
}

impl JsonRpcRequest {
    pub fn new(id: RequestId, method: String, params: Option<Params>) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_owned(),
            id,
            method,
            params,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct JsonRpcNotification {
    jsonrpc: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Params>,
}

impl JsonRpcNotification {
    pub fn new(method: String, params: Option<Params>) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_owned(),
            method,
            params,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
#[serde(untagged)]
pub enum JsonRpcResponse {
    Success {
        jsonrpc: String,
        id: RequestId,
        result: Value,
    },
    Error {
        jsonrpc: String,
        id: RequestId,
        error: ErrorData,
    },
}

impl JsonRpcResponse {
    pub fn success(id: RequestId, result: Value) -> Self {
        Self::Success {
            jsonrpc: JSONRPC_VERSION.to_owned(),
            id,
            result,
        }
    }

    pub fn error(id: RequestId, error: ErrorData) -> Self {
        Self::Error {
            jsonrpc: JSONRPC_VERSION.to_owned(),
            id,
            error,
        }
    }
}

// Standard JSON-RPC error codes
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorCode {
    /// Invalid JSON was received by the server.
    ParseError,
    /// The JSON sent is not a valid Request object.
    InvalidRequest,
    /// The method does not exist / is not available.
    MethodNotFound,
    /// Invalid method parameters.
    InvalidParams,
    /// Internal JSON-RPC error.
    InternalError,
    /// Custom, implementation-defined server errors.
    Custom(i32),
}

impl ErrorCode {
    pub const fn code(&self) -> i32 {
        match self {
            Self::ParseError => -32700,
            Self::InvalidRequest => -32600,
            Self::MethodNotFound => -32601,
            Self::InvalidParams => -32602,
            Self::InternalError => -32603,
            Self::Custom(code) => *code,
        }
    }
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.code())
    }
}

impl Serialize for ErrorCode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_i32(self.code())
    }
}

impl<'de> Deserialize<'de> for ErrorCode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let code = i32::deserialize(deserializer)?;
        Ok(match code {
            -32700 => Self::ParseError,
            -32600 => Self::InvalidRequest,
            -32601 => Self::MethodNotFound,
            -32602 => Self::InvalidParams,
            -32603 => Self::InternalError,
            other => Self::Custom(other),
        })
    }
}

/// Error information for JSON-RPC error responses.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ErrorData {
    /// The error type that occurred.
    pub code: ErrorCode,

    /// A short description of the error. The message SHOULD be limited to a concise single sentence.
    pub message: String,

    /// Additional information about the error. The value of this member is defined by the
    /// sender (e.g. detailed error information, nested errors etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl ErrorData {
    /// Create a new error data instance, with no additional data.
    pub fn new(code: ErrorCode, message: String) -> Self {
        Self {
            code,
            message,
            data: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{self, json};

    #[test]
    fn id_deserialization() {
        let s = r#""2""#;
        let deserialized: RequestId = serde_json::from_str(s).unwrap();
        assert_eq!(deserialized, RequestId::Str("2".into()));

        let s = r#"2"#;
        let deserialized: RequestId = serde_json::from_str(s).unwrap();
        assert_eq!(deserialized, RequestId::Num(2));

        let s = r#""2x""#;
        let deserialized: RequestId = serde_json::from_str(s).unwrap();
        assert_eq!(deserialized, RequestId::Str("2x".to_owned()));

        // UUID v4
        let s = r#""4a54203b-20c0-4367-a15b-938ec6d92bf2""#;
        let deserialized: RequestId = serde_json::from_str(s).unwrap();
        assert_eq!(
            deserialized,
            RequestId::Str("4a54203b-20c0-4367-a15b-938ec6d92bf2".to_owned())
        );

        let s = r#"[null, 0, 2, "3"]"#;
        let deserialized: Vec<RequestId> = serde_json::from_str(s).unwrap();
        assert_eq!(
            deserialized,
            vec![
                RequestId::Null,
                RequestId::Num(0),
                RequestId::Num(2),
                RequestId::Str("3".into())
            ]
        );
    }

    #[test]
    fn id_serialization() {
        let d = vec![
            RequestId::Num(0),
            RequestId::Num(2),
            RequestId::Num(3),
            RequestId::Str("3".to_owned()),
            RequestId::Str("test".to_owned()),
            RequestId::Str("4a54203b-20c0-4367-a15b-938ec6d92bf2".to_owned()),
        ];
        let serialized = serde_json::to_string(&d).unwrap();
        assert_eq!(
            serialized,
            r#"[0,2,3,"3","test","4a54203b-20c0-4367-a15b-938ec6d92bf2"]"#
        );
    }

    #[test]
    fn request_serialization() {
        // without params
        let request = JsonRpcRequest::new(RequestId::Num(1), "test".to_string(), None);
        let serialized = serde_json::to_string(&request).unwrap();
        assert_eq!(serialized, r#"{"jsonrpc":"2.0","id":1,"method":"test"}"#);

        // with array params
        let request = JsonRpcRequest::new(
            RequestId::Num(1),
            "test".to_string(),
            Some(Params::try_from(serde_json::json!([1, 2, 3])).unwrap()),
        );
        let serialized = serde_json::to_string(&request).unwrap();
        assert_eq!(
            serialized,
            r#"{"jsonrpc":"2.0","id":1,"method":"test","params":[1,2,3]}"#
        );

        // with map params
        let request = JsonRpcRequest::new(
            RequestId::Num(1),
            "test".to_string(),
            Some(
                Params::try_from(serde_json::json!({ "key": "value", "key2": "value2" })).unwrap(),
            ),
        );
        let serialized = serde_json::to_string(&request).unwrap();
        assert_eq!(
            serialized,
            r#"{"jsonrpc":"2.0","id":1,"method":"test","params":{"key":"value","key2":"value2"}}"#
        );

        let request = JsonRpcRequest::new(RequestId::Null, "test".to_string(), None);
        let serialized = serde_json::to_string(&request).unwrap();
        assert_eq!(serialized, r#"{"jsonrpc":"2.0","id":null,"method":"test"}"#);
    }

    #[test]
    fn request_deserialization() {
        // without params
        let request = r#"{"jsonrpc":"2.0","id":1,"method":"test"}"#;
        let deserialized: JsonRpcRequest = serde_json::from_str(request).unwrap();
        assert_eq!(
            deserialized,
            JsonRpcRequest::new(RequestId::Num(1), "test".to_string(), None)
        );

        // with array params
        let request = r#"{"jsonrpc":"2.0","id":1,"method":"test","params":[1,2,3]}"#;
        let deserialized: JsonRpcRequest = serde_json::from_str(request).unwrap();
        assert_eq!(
            deserialized,
            JsonRpcRequest::new(
                RequestId::Num(1),
                "test".to_string(),
                Some(Params::try_from(serde_json::json!([1, 2, 3])).unwrap())
            )
        );

        // with params
        let request = r#"{"jsonrpc":"2.0","id":1,"method":"test","params":{"key":"value"}}"#;
        let deserialized: JsonRpcRequest = serde_json::from_str(request).unwrap();
        assert_eq!(
            deserialized,
            JsonRpcRequest::new(
                RequestId::Num(1),
                "test".to_string(),
                Some(Params::try_from(serde_json::json!({ "key": "value" })).unwrap())
            )
        );
    }

    #[test]
    fn notification_serialization() {
        // without params
        let notification = JsonRpcNotification::new("test".to_string(), None);
        let serialized = serde_json::to_string(&notification).unwrap();
        assert_eq!(serialized, r#"{"jsonrpc":"2.0","method":"test"}"#);

        // with params
        let notification = JsonRpcNotification::new(
            "test".to_string(),
            Some(Params::Map(serde_json::Map::from_iter([(
                "key".to_string(),
                Value::from("value".to_string()),
            )]))),
        );
        let serialized = serde_json::to_string(&notification).unwrap();
        assert_eq!(
            serialized,
            r#"{"jsonrpc":"2.0","method":"test","params":{"key":"value"}}"#
        );
    }

    #[test]
    fn notification_deserialization() {
        // without params
        let notification = r#"{"jsonrpc":"2.0","method":"test"}"#;
        let deserialized: JsonRpcNotification = serde_json::from_str(notification).unwrap();
        assert_eq!(
            deserialized,
            JsonRpcNotification::new("test".to_string(), None)
        );

        // with params
        let notification = r#"{"jsonrpc":"2.0","method":"test","params":{"key":"value"}}"#;
        let deserialized: JsonRpcNotification = serde_json::from_str(notification).unwrap();
        assert_eq!(
            deserialized,
            JsonRpcNotification::new(
                "test".to_string(),
                Some(Params::Map(serde_json::Map::from_iter([(
                    "key".to_string(),
                    Value::from("value".to_string()),
                )])))
            )
        );
    }

    #[test]
    fn response_serialization() {
        let response = JsonRpcResponse::success(RequestId::Num(1), json!({ "key": "value" }));
        let serialized = serde_json::to_string(&response).unwrap();
        assert_eq!(
            serialized,
            r#"{"jsonrpc":"2.0","id":1,"result":{"key":"value"}}"#
        );
    }

    #[test]
    fn response_deserialization() {
        let response = r#"{"jsonrpc":"2.0","id":1,"result":{"key":"value"}}"#;
        let deserialized: JsonRpcResponse = serde_json::from_str(response).unwrap();
        assert_eq!(
            deserialized,
            JsonRpcResponse::success(RequestId::Num(1), json!({ "key": "value" }))
        );
    }

    #[test]
    fn error_serialization() {
        let error = JsonRpcResponse::error(
            RequestId::Num(42),
            ErrorData {
                code: ErrorCode::ParseError,
                message: "Parse error".to_string(),
                data: None,
            },
        );
        let serialized = serde_json::to_string(&error).unwrap();
        assert_eq!(
            serialized,
            r#"{"jsonrpc":"2.0","id":42,"error":{"code":-32700,"message":"Parse error"}}"#
        );
    }

    #[test]
    fn error_deserialization() {
        let error = r#"{"jsonrpc":"2.0","id":42,"error":{"code":-32700,"message":"Parse error"}}"#;
        let deserialized: JsonRpcResponse = serde_json::from_str(error).unwrap();
        assert_eq!(
            deserialized,
            JsonRpcResponse::error(
                RequestId::Num(42),
                ErrorData {
                    code: ErrorCode::ParseError,
                    message: "Parse error".to_string(),
                    data: None,
                }
            )
        );
    }
}
