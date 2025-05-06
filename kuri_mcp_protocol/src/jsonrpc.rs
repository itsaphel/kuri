// The protocol defined here is largely spec compliant.
//
// Deviations:
// * No batching support
// * Requests and responses are assumed to be client-generated, not bi-directional.
use serde::{de, Deserialize, Serialize};
use serde_json::Value;
use valuable::Valuable;

/// A single JSON-RPC message.
#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(untagged)]
pub enum SendableMessage {
    Request(MethodCall),
    Notification(Notification),
    Invalid {
        /// call ID (if known)
        #[serde(default = "RequestId::null")]
        id: RequestId,
    },
}

impl From<MethodCall> for SendableMessage {
    fn from(request: MethodCall) -> Self {
        SendableMessage::Request(request)
    }
}

impl From<Notification> for SendableMessage {
    fn from(notification: Notification) -> Self {
        SendableMessage::Notification(notification)
    }
}

impl<'de> serde::Deserialize<'de> for SendableMessage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        if let Ok(req) = MethodCall::deserialize(&value) {
            return Ok(SendableMessage::Request(req));
        }
        if let Ok(note) = Notification::deserialize(&value) {
            return Ok(SendableMessage::Notification(note));
        }

        // Invalid message. Extract ID if possible.
        let id = match &value {
            serde_json::Value::Object(map) => map
                .get("id")
                .and_then(|id_val| RequestId::deserialize(id_val).ok())
                .unwrap_or_else(RequestId::null),
            _ => RequestId::Null,
        };
        Ok(SendableMessage::Invalid { id })
    }
}

/// A single JSON-RPC request, which is sent over the transport, and may contain multiple messages
/// (per JSON-RPC's batch specification).
#[derive(Debug, Serialize, Clone, PartialEq)]
pub enum Request {
    /// Single message
    Single(SendableMessage),
    /// Batch of messages
    Batch(Vec<SendableMessage>),
}

impl<'de> serde::Deserialize<'de> for Request {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, MapAccess, SeqAccess, Visitor};
        use std::fmt;

        struct RequestVisitor;

        impl<'de> Visitor<'de> for RequestVisitor {
            type Value = Request;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a JSON-RPC request or batch of requests")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Request, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut messages = Vec::new();
                while let Some(msg) = seq.next_element::<SendableMessage>()? {
                    messages.push(msg);
                }
                Ok(Request::Batch(messages))
            }

            fn visit_map<M>(self, map: M) -> Result<Request, M::Error>
            where
                M: MapAccess<'de>,
            {
                let value = serde_json::Value::deserialize(
                    serde::de::value::MapAccessDeserializer::new(map),
                )?;
                let msg = SendableMessage::deserialize(value).map_err(de::Error::custom)?;
                Ok(Request::Single(msg))
            }
        }

        deserializer.deserialize_any(RequestVisitor)
    }
}

/// A single JSON-RPC response, which is sent over the transport.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(untagged)]
pub enum Response {
    /// Single message
    Single(Option<ResponseItem>),
    /// Batch of messages
    Batch(Vec<ResponseItem>),
}

impl Response {
    pub fn is_empty(&self) -> bool {
        match self {
            Response::Single(opt) => opt.is_none(),
            Response::Batch(responses) => responses.is_empty(),
        }
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
    pub const fn null() -> Self {
        RequestId::Null
    }

    #[inline]
    pub const fn is_null(&self) -> bool {
        matches!(self, RequestId::Null)
    }
}

/// Protocol version
#[derive(Debug, PartialEq, Clone, Copy, Hash, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum JsonRpcVersion {
    V2,
}

impl TryFrom<String> for JsonRpcVersion {
    type Error = serde::de::value::Error;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "2.0" => Ok(JsonRpcVersion::V2),
            _ => Err(serde::de::Error::custom("not a valid JSON-RPC 2.0 message")),
        }
    }
}

impl From<JsonRpcVersion> for String {
    fn from(version: JsonRpcVersion) -> Self {
        match version {
            JsonRpcVersion::V2 => "2.0".to_string(),
        }
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

/// An RPC method call (known in the JSON-RPC spec as a "request").
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct MethodCall {
    jsonrpc: JsonRpcVersion,
    /// JSON-RPC spec: if `id` is ommitted, the request is assumed to be a notification.
    /// JSON-RPC permits this to be a "null" value, but MCP spec does not.
    pub id: RequestId,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Params>,
}

impl MethodCall {
    pub fn new(id: RequestId, method: String, params: Option<Params>) -> Self {
        Self {
            jsonrpc: JsonRpcVersion::V2,
            id,
            method,
            params,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Notification {
    jsonrpc: JsonRpcVersion,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Params>,
}

impl Notification {
    pub fn new(method: String, params: Option<Params>) -> Self {
        Self {
            jsonrpc: JsonRpcVersion::V2,
            method,
            params,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
#[serde(untagged)]
pub enum ResponseItem {
    Success {
        jsonrpc: JsonRpcVersion,
        id: RequestId,
        result: Value,
    },
    Error {
        jsonrpc: JsonRpcVersion,
        id: RequestId,
        error: ErrorData,
    },
}

impl ResponseItem {
    pub fn success(id: RequestId, result: Value) -> Self {
        Self::Success {
            jsonrpc: JsonRpcVersion::V2,
            id,
            result,
        }
    }

    pub fn error(id: RequestId, error: ErrorData) -> Self {
        Self::Error {
            jsonrpc: JsonRpcVersion::V2,
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
        let request = MethodCall::new(RequestId::Num(1), "test".to_string(), None);
        let serialized = serde_json::to_string(&request).unwrap();
        assert_eq!(serialized, r#"{"jsonrpc":"2.0","id":1,"method":"test"}"#);

        // with array params
        let request = MethodCall::new(
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
        let request = MethodCall::new(
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

        let request = MethodCall::new(RequestId::Null, "test".to_string(), None);
        let serialized = serde_json::to_string(&request).unwrap();
        assert_eq!(serialized, r#"{"jsonrpc":"2.0","id":null,"method":"test"}"#);
    }

    #[test]
    fn request_deserialization() {
        // without params
        let request = r#"{"jsonrpc":"2.0","id":1,"method":"test"}"#;
        let deserialized: MethodCall = serde_json::from_str(request).unwrap();
        assert_eq!(
            deserialized,
            MethodCall::new(RequestId::Num(1), "test".to_string(), None)
        );

        // with array params
        let request = r#"{"jsonrpc":"2.0","id":1,"method":"test","params":[1,2,3]}"#;
        let deserialized: MethodCall = serde_json::from_str(request).unwrap();
        assert_eq!(
            deserialized,
            MethodCall::new(
                RequestId::Num(1),
                "test".to_string(),
                Some(Params::try_from(serde_json::json!([1, 2, 3])).unwrap())
            )
        );

        // with params
        let request = r#"{"jsonrpc":"2.0","id":1,"method":"test","params":{"key":"value"}}"#;
        let deserialized: MethodCall = serde_json::from_str(request).unwrap();
        assert_eq!(
            deserialized,
            MethodCall::new(
                RequestId::Num(1),
                "test".to_string(),
                Some(Params::try_from(serde_json::json!({ "key": "value" })).unwrap())
            )
        );
    }

    #[test]
    fn notification_serialization() {
        // without params
        let notification = Notification::new("test".to_string(), None);
        let serialized = serde_json::to_string(&notification).unwrap();
        assert_eq!(serialized, r#"{"jsonrpc":"2.0","method":"test"}"#);

        // with params
        let notification = Notification::new(
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
        let deserialized: Notification = serde_json::from_str(notification).unwrap();
        assert_eq!(deserialized, Notification::new("test".to_string(), None));

        // with params
        let notification = r#"{"jsonrpc":"2.0","method":"test","params":{"key":"value"}}"#;
        let deserialized: Notification = serde_json::from_str(notification).unwrap();
        assert_eq!(
            deserialized,
            Notification::new(
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
        let response = ResponseItem::success(RequestId::Num(1), json!({ "key": "value" }));
        let serialized = serde_json::to_string(&response).unwrap();
        assert_eq!(
            serialized,
            r#"{"jsonrpc":"2.0","id":1,"result":{"key":"value"}}"#
        );
    }

    #[test]
    fn response_deserialization() {
        let response = r#"{"jsonrpc":"2.0","id":1,"result":{"key":"value"}}"#;
        let deserialized: ResponseItem = serde_json::from_str(response).unwrap();
        assert_eq!(
            deserialized,
            ResponseItem::success(RequestId::Num(1), json!({ "key": "value" }))
        );
    }

    #[test]
    fn error_serialization() {
        let error = ResponseItem::error(
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
        let deserialized: ResponseItem = serde_json::from_str(error).unwrap();
        assert_eq!(
            deserialized,
            ResponseItem::error(
                RequestId::Num(42),
                ErrorData {
                    code: ErrorCode::ParseError,
                    message: "Parse error".to_string(),
                    data: None,
                }
            )
        );
    }

    #[test]
    fn jsonrpc_version_error() {
        // v1.0
        let request = r#"{"jsonrpc":"1.0","id":1,"method":"test"}"#;
        let deserialised = serde_json::from_str::<MethodCall>(request);
        assert!(deserialised.is_err());
        assert_eq!(
            deserialised.err().unwrap().to_string(),
            "not a valid JSON-RPC 2.0 message at line 1 column 16"
        );

        // no version
        let request = r#"{"id":1,"method":"test"}"#;
        let deserialised = serde_json::from_str::<MethodCall>(request);
        assert!(deserialised.is_err());
        assert_eq!(
            deserialised.err().unwrap().to_string(),
            "missing field `jsonrpc` at line 1 column 24"
        );
    }

    #[test]
    fn request_batch_deserialisation() {
        // all ok
        let request = r#"[{"jsonrpc":"2.0","id":1,"method":"tools/calc/add","params":[1,2,3]}, {"jsonrpc":"2.0","id":2,"method":"tools/list"}, {"jsonrpc": "2.0", "method": "notify_sum", "params": [1,2,4]}]"#;
        let deserialised = serde_json::from_str::<Request>(request);
        assert!(deserialised.is_ok());
        let request = deserialised.unwrap();

        match request {
            Request::Batch(messages) => {
                assert_eq!(messages.len(), 3);
                assert!(matches!(messages[0], SendableMessage::Request(_)));
                assert!(matches!(messages[1], SendableMessage::Request(_)));
                assert!(matches!(messages[2], SendableMessage::Notification(_)));
            }
            _ => panic!("expected a batch"),
        }
    }

    #[test]
    fn request_batch_deserialisation_with_errors() {
        // all invalid
        let request = r#"[1]"#;
        let deserialised = serde_json::from_str::<Request>(request);
        assert!(deserialised.is_ok());
        let request = deserialised.unwrap();

        match request {
            Request::Batch(messages) => {
                assert_eq!(messages.len(), 1);
                assert!(matches!(
                    messages[0],
                    SendableMessage::Invalid {
                        id: RequestId::Null
                    }
                ));
            }
            _ => panic!("expected a batch"),
        }

        // one valid, two invalid
        let request = r#"[{"jsonrpc":"2.0","id":1,"method":"test"}, {"jsonrpc":"1.0","id":1,"method":"test"}, {"foo":"bar"}]"#;
        let deserialised = serde_json::from_str::<Request>(request);
        assert!(deserialised.is_ok());
        let request = deserialised.unwrap();

        match request {
            Request::Batch(messages) => {
                assert_eq!(messages.len(), 3);
                assert!(matches!(messages[0], SendableMessage::Request(_)));
                assert!(matches!(
                    messages[1],
                    SendableMessage::Invalid {
                        id: RequestId::Num(1),
                        ..
                    }
                ));
                assert!(matches!(
                    messages[2],
                    SendableMessage::Invalid {
                        id: RequestId::Null,
                        ..
                    }
                ));
            }
            _ => panic!("expected a batch"),
        }
    }

    #[test]
    fn response_serialisation() {
        let response = Response::Single(Some(ResponseItem::success(
            RequestId::Num(1),
            json!({ "key": "value" }),
        )));
        let serialized = serde_json::to_string(&response).unwrap();
        assert_eq!(
            serialized,
            r#"{"jsonrpc":"2.0","id":1,"result":{"key":"value"}}"#
        );
    }
}
