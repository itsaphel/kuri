use kuri_mcp_protocol::jsonrpc::RequestId;
use uuid::Uuid;

/// All JSON-RPC requests that expect a response need a unique request ID.
/// This trait can be implemented to provide new formats for generating request IDs.
///
/// The default generator is a UUID generator.
pub trait RequestIdGenerator: Send + Sync + 'static {
    fn next_id(&self) -> RequestId;
}

/// Generate request IDs using UUID v7.
///
/// UUIDs have a few advantages over atomic integers. They don't need to take a lock, so no
/// synchronisation between threads/servers is required. No state needs to be stored, so it's more
/// suitable for serverless environments. UUID v7 is nicer than UUID v4 for database locality
/// reasons, assuming the MCP server wants to persist requests.
pub struct Uuidv7RequestIdGenerator;

impl RequestIdGenerator for Uuidv7RequestIdGenerator {
    fn next_id(&self) -> RequestId {
        RequestId::Str(Uuid::now_v7().to_string())
    }
}
