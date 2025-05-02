use crate::{
    context::{Context, Inject},
    errors::RequestError,
    handler::{PromptHandler, ToolHandler},
};
use kuri_mcp_protocol::{
    jsonrpc::{ErrorData, JsonRpcRequest, JsonRpcResponse, Params, SendableMessage},
    messages::{
        CallToolResult, GetPromptResult, Implementation, InitializeResult, ListPromptsResult,
        ListResourcesResult, ListToolsResult, PromptsCapability, ReadResourceResult,
        ResourcesCapability, ServerCapabilities, ToolsCapability,
    },
    prompt::{Prompt as PromptMeta, PromptError, PromptMessage, PromptMessageRole},
    resource::{Resource as ResourceMeta, ResourceContents, ResourceError},
    tool::{Tool as ToolMeta, ToolError},
};
use serde_json::json;
use serde_json::Value;
use std::task::Poll;
use std::{collections::HashMap, future::Future, pin::Pin};
use std::{convert::Infallible, rc::Rc};
use tower::Service;

type Tools = HashMap<String, Rc<dyn ToolHandler>>;
type Prompts = HashMap<String, Rc<dyn PromptHandler>>;

/// A service that handles MCP requests.
///
/// The `MCPService` is responsible for handling `JsonRpcRequest`s, whatever their origin (including
/// as library calls), and returning `JsonRpcResponse`s. It also maintains internal state with the
/// tools, prompts, and context, as well as the capabilities of the server. This is in contrast to
/// `server.rs`, which runs continuously in a loop handling requests (passing them to `MCPService`)
/// and middlemanning communication with the transport layer.
#[derive(Clone)]
pub struct MCPService {
    name: String,
    description: String,
    tools: Rc<Tools>,
    prompts: Rc<Prompts>,
    ctx: Rc<Context>,
}

/// Build an MCPService. Tools and structs are defined when the MCPService is built. They cannot be
/// modified after that time.
pub struct MCPServiceBuilder {
    name: String,
    description: String,
    tools: Tools,
    prompts: Prompts,
    ctx: Context,
}

impl MCPServiceBuilder {
    pub fn new(name: String, description: String) -> Self {
        Self {
            name,
            description,
            tools: HashMap::new(),
            prompts: HashMap::new(),
            ctx: Context::default(),
        }
    }

    pub fn with_tool(mut self, tool: impl ToolHandler) -> Self {
        self.tools.insert(tool.name().to_string(), Rc::new(tool));
        self
    }

    pub fn with_prompt(mut self, prompt: impl PromptHandler) -> Self {
        self.prompts
            .insert(prompt.name().to_string(), Rc::new(prompt));
        self
    }

    pub fn with_state<T: 'static>(mut self, state: Inject<T>) -> Self {
        self.ctx.insert(state);
        self
    }

    pub fn build(self) -> MCPService {
        MCPService {
            name: self.name,
            description: self.description,
            tools: Rc::new(self.tools),
            prompts: Rc::new(self.prompts),
            ctx: Rc::new(self.ctx),
        }
    }
}

/// Builder for configuring and constructing capabilities
pub struct CapabilitiesBuilder {
    tools: Option<ToolsCapability>,
    prompts: Option<PromptsCapability>,
    resources: Option<ResourcesCapability>,
}

impl Default for CapabilitiesBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl CapabilitiesBuilder {
    pub fn new() -> Self {
        Self {
            tools: None,
            prompts: None,
            resources: None,
        }
    }

    /// Add multiple tools to the router
    pub fn with_tools(mut self, list_changed: bool) -> Self {
        self.tools = Some(ToolsCapability {
            list_changed: Some(list_changed),
        });
        self
    }

    /// Enable prompts capability
    pub fn with_prompts(mut self, list_changed: bool) -> Self {
        self.prompts = Some(PromptsCapability {
            list_changed: Some(list_changed),
        });
        self
    }

    /// Enable resources capability
    #[allow(dead_code)]
    pub fn with_resources(mut self, subscribe: bool, list_changed: bool) -> Self {
        self.resources = Some(ResourcesCapability {
            subscribe: Some(subscribe),
            list_changed: Some(list_changed),
        });
        self
    }

    /// Build the router with automatic capability inference
    pub fn build(self) -> ServerCapabilities {
        // Create capabilities based on what's configured
        ServerCapabilities {
            tools: self.tools,
            prompts: self.prompts,
            resources: self.resources,
        }
    }
}

trait MCPServiceTrait: 'static {
    fn name(&self) -> String;
    // in the protocol, instructions are optional but we make it required
    fn instructions(&self) -> String;
    fn capabilities(&self) -> ServerCapabilities;

    fn list_tools(&self) -> Vec<ToolMeta>;
    fn call_tool(
        &self,
        tool_name: &str,
        arguments: Value,
    ) -> Pin<Box<dyn Future<Output = Result<CallToolResult, ToolError>> + '_>>;
    fn list_resources(&self) -> Vec<ResourceMeta>;
    fn read_resource(
        &self,
        uri: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, ResourceError>> + 'static>>;
    fn list_prompts(&self) -> Vec<PromptMeta>;
    fn get_prompt(
        &self,
        prompt_name: &str,
        arguments: HashMap<String, serde_json::Value>,
    ) -> Pin<Box<dyn Future<Output = Result<String, PromptError>> + '_>>;
}

impl MCPServiceTrait for MCPService {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn instructions(&self) -> String {
        self.description.clone()
    }

    fn capabilities(&self) -> kuri_mcp_protocol::messages::ServerCapabilities {
        // MCPService only allows tools and prompts to be registered at build time, after which they
        // cannot be changed. Consequently, we set `list_changed` to false, though "true" would be
        // equally correct.

        let mut builder = CapabilitiesBuilder::new();
        if !self.tools.is_empty() {
            builder = builder.with_tools(false);
        }
        if !self.prompts.is_empty() {
            builder = builder.with_prompts(false);
        }
        // if self.resources.len() > 0 {
        //     builder.with_resources(true, true);
        // }

        builder.build()
    }

    /// List tool schema for all tools registered with this MCP server.
    fn list_tools(&self) -> Vec<ToolMeta> {
        self.tools
            .iter()
            .map(|(name, tool)| ToolMeta::new(name.clone(), tool.description(), tool.schema()))
            .collect()
    }

    /// Call a tool.
    ///
    /// Guarantees:
    /// * `tool_name` is *not* guaranteed to be a valid tool.
    /// * `arguments` may not contain all arguments required by the tool handler. Also, it may
    ///   contain arguments not used by the tool handler.
    fn call_tool(
        &self,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> Pin<Box<dyn Future<Output = Result<CallToolResult, ToolError>> + '_>> {
        let tool = match self.tools.get(tool_name) {
            Some(tool) => tool,
            None => {
                return Box::pin(futures::future::ready(Err(ToolError::NotFound(
                    tool_name.to_string(),
                ))))
            }
        };
        Box::pin(async move { tool.call(&self.ctx, arguments).await })
    }

    fn list_resources(&self) -> Vec<ResourceMeta> {
        // TODO implement
        vec![]
    }

    fn read_resource(
        &self,
        _uri: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, ResourceError>> + 'static>> {
        // TODO implement
        Box::pin(futures::future::ready(Err(ResourceError::ExecutionError(
            "Reading resources is not yet implemented".into(),
        ))))
    }

    /// List prompt schema for all prompts registered with this MCP server.
    fn list_prompts(&self) -> Vec<PromptMeta> {
        self.prompts
            .values()
            .map(|prompt| PromptMeta::new(prompt.name(), prompt.description(), prompt.arguments()))
            .collect()
    }

    /// Call a prompt with the given name and arguments.
    ///
    /// Guarantees:
    /// * `prompt_name` is *not* guaranteed to be a valid prompt.
    /// * `arguments` may not contain all arguments required by the prompt handler. Also, it may
    ///   contain arguments not used by the prompt handler.
    fn get_prompt(
        &self,
        prompt_name: &str,
        arguments: HashMap<String, serde_json::Value>,
    ) -> Pin<Box<dyn Future<Output = Result<String, PromptError>> + '_>> {
        // TODO: Write more idiomatic, and ideally move into the async block.
        let prompt = match self.prompts.get(prompt_name) {
            Some(prompt) => prompt,
            None => {
                return Box::pin(futures::future::ready(Err(PromptError::NotFound(
                    prompt_name.to_string(),
                ))));
            }
        };
        Box::pin(async move {
            let result = prompt.call(&self.ctx, arguments).await?;
            Ok(result)
        })
    }
}

/// Validate and return request parameters
fn get_request_params(
    params: Option<Params>,
) -> Result<serde_json::Map<String, Value>, RequestError> {
    match params {
        Some(Params::Map(map)) => Ok(map),
        Some(_) => Err(RequestError::InvalidParams(
            "Parameters must be a map-like object".to_string(),
        )),
        None => Err(RequestError::InvalidParams(
            "The request was empty".to_string(),
        )),
    }
}

/// Note: Handlers only perform *syntactic* validation. For instance, that required arguments are
/// provided, or that they're (immediately) of the correct type. The methods on `MCPServiceTrait`
/// are ultimately responsible for verifying the *semantic* correctness of the arguments, including
/// whether the tool/prompt exists, etc.
///
/// The above may change, as the distinction may be unnecessary.
#[allow(clippy::manual_async_fn)]
impl MCPService {
    fn handle_ping(
        &self,
        req: JsonRpcRequest,
    ) -> impl Future<Output = Result<JsonRpcResponse, RequestError>> {
        async move { Ok(JsonRpcResponse::success(req.id, json!({}))) }
    }

    fn handle_initialize(
        &self,
        req: JsonRpcRequest,
    ) -> impl Future<Output = Result<JsonRpcResponse, RequestError>> + '_ {
        async move {
            // Build response content
            let result = InitializeResult {
                protocol_version: "2024-11-05".to_string(),
                capabilities: self.capabilities(),
                server_info: Implementation {
                    name: self.name(),
                    version: env!("CARGO_PKG_VERSION").to_string(),
                },
                instructions: Some(self.instructions()),
            };

            // Serialise response
            let result = serde_json::to_value(result)
                .map_err(|e| RequestError::Internal(format!("JSON serialization error: {}", e)))?;
            let response = JsonRpcResponse::success(req.id, result);
            Ok(response)
        }
    }

    fn handle_tools_list(
        &self,
        req: JsonRpcRequest,
    ) -> impl Future<Output = Result<JsonRpcResponse, RequestError>> + '_ {
        async move {
            // No request arguments required.

            // Build response content
            let tools = self.list_tools();
            let result = ListToolsResult { tools };

            // Serialise response
            let result = serde_json::to_value(result)
                .map_err(|e| RequestError::Internal(format!("JSON serialization error: {}", e)))?;
            let response = JsonRpcResponse::success(req.id, result);
            Ok(response)
        }
    }

    fn handle_tools_call(
        &self,
        req: JsonRpcRequest,
    ) -> impl Future<Output = Result<JsonRpcResponse, RequestError>> + '_ {
        async move {
            // Get and validate request parameters
            let params = get_request_params(req.params)?;

            let name = params
                .get("name")
                .and_then(Value::as_str)
                .ok_or_else(|| RequestError::InvalidParams("No tool name was provided".into()))?;

            let arguments = params.get("arguments").cloned().unwrap_or(Value::Null);

            // Call tool and build response content
            let result = self.call_tool(name, arguments).await?;

            // Serialise response
            let result = serde_json::to_value(result)
                .map_err(|e| RequestError::Internal(format!("JSON serialization error: {}", e)))?;
            let response = JsonRpcResponse::success(req.id, result);
            Ok(response)
        }
    }

    fn handle_resources_list(
        &self,
        req: JsonRpcRequest,
    ) -> impl Future<Output = Result<JsonRpcResponse, RequestError>> + '_ {
        async move {
            // No request arguments required.

            // Build response content
            let resources = self.list_resources();
            let result = ListResourcesResult { resources };

            // Serialise response
            let result = serde_json::to_value(result)
                .map_err(|e| RequestError::Internal(format!("JSON serialization error: {}", e)))?;
            let response = JsonRpcResponse::success(req.id, result);
            Ok(response)
        }
    }

    fn handle_resources_read(
        &self,
        req: JsonRpcRequest,
    ) -> impl Future<Output = Result<JsonRpcResponse, RequestError>> + '_ {
        async move {
            // Get and validate request parameters
            let params = get_request_params(req.params)?;

            let uri = params
                .get("uri")
                .and_then(Value::as_str)
                .ok_or_else(|| RequestError::InvalidParams("Missing resource URI".into()))?;

            // Read resource and build response content
            let contents = self.read_resource(uri).await.map_err(RequestError::from)?;
            let result = ReadResourceResult {
                contents: vec![ResourceContents::TextResourceContents {
                    uri: uri.to_string(),
                    mime_type: Some("text/plain".to_string()),
                    text: contents,
                }],
            };

            let result = serde_json::to_value(result)
                .map_err(|e| RequestError::Internal(format!("JSON serialization error: {}", e)))?;
            let response = JsonRpcResponse::success(req.id, result);
            Ok(response)
        }
    }

    fn handle_prompts_list(
        &self,
        req: JsonRpcRequest,
    ) -> impl Future<Output = Result<JsonRpcResponse, RequestError>> + '_ {
        async move {
            // No request arguments required.

            // Build response content
            let prompts = self.list_prompts();
            let result = ListPromptsResult { prompts };

            // Serialise response
            let result = serde_json::to_value(result)
                .map_err(|e| RequestError::Internal(format!("JSON serialization error: {}", e)))?;
            let response = JsonRpcResponse::success(req.id, result);
            Ok(response)
        }
    }

    fn handle_prompts_get(
        &self,
        req: JsonRpcRequest,
    ) -> impl Future<Output = Result<JsonRpcResponse, RequestError>> + '_ {
        async move {
            // Get and validate request parameters
            let params = get_request_params(req.params)?;

            let prompt_name = params
                .get("name")
                .and_then(Value::as_str)
                .ok_or_else(|| RequestError::InvalidParams("Missing prompt name".into()))?;

            // Ensure arguments are provided,
            // TODO: Only error if arguments are required.
            let arguments = params
                .get("arguments")
                .and_then(Value::as_object)
                .ok_or_else(|| RequestError::InvalidParams("Missing arguments object".into()))?;
            // then convert from serde_json::Map<String, Value> to HashMap<String, Value>
            let arguments: HashMap<String, serde_json::Value> = arguments
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect();

            // Call prompt handler and build response content
            let prompt_message =
                self.get_prompt(prompt_name, arguments)
                    .await
                    .map_err(|e| match e {
                        PromptError::InvalidParameters(_) => {
                            RequestError::InvalidParams(e.to_string())
                        }
                        PromptError::NotFound(_) => RequestError::InvalidParams(e.to_string()),
                        PromptError::InternalError(_) => RequestError::Internal(e.to_string()),
                    })?;

            let messages = vec![PromptMessage::new_text(
                // TODO: Unclear role correctness.
                PromptMessageRole::User,
                prompt_message.to_string(),
            )];

            // Build final response and serialise
            let result = serde_json::to_value(GetPromptResult {
                // TODO: Unclear if we need `description` here.
                description: None,
                messages,
            })
            .map_err(|e| RequestError::Internal(format!("JSON serialization error: {}", e)))?;
            let response = JsonRpcResponse::success(req.id, result);
            Ok(response)
        }
    }
}

impl Service<SendableMessage> for MCPService {
    type Response = Option<JsonRpcResponse>;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&mut self, _cx: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    /// Returns a future that handles the request and resolves to an (optional) JSON-RPC response.
    /// If no response is to be emitted (eg notifications or unsupported requests without an id),
    /// then returns Ok(None).
    fn call(&mut self, req: SendableMessage) -> Self::Future {
        let this = self.clone();
        Box::pin(async move {
            if let SendableMessage::Request(req) = req {
                let id = req.id.clone();
                let result = match req.method.as_str() {
                    "ping" => this.handle_ping(req).await,
                    "initialize" => this.handle_initialize(req).await,
                    "tools/list" => this.handle_tools_list(req).await,
                    "tools/call" => this.handle_tools_call(req).await,
                    "resources/list" => this.handle_resources_list(req).await,
                    "resources/read" => this.handle_resources_read(req).await,
                    "prompts/list" => this.handle_prompts_list(req).await,
                    "prompts/get" => this.handle_prompts_get(req).await,
                    _ => Err(RequestError::MethodNotFound(req.method)),
                };

                let response = match result {
                    Ok(response) => response,
                    Err(e) => {
                        let error = ErrorData::from(e);
                        JsonRpcResponse::error(id, error)
                    }
                };
                Ok(Some(response))
            } else {
                Ok(None)
            }
        })
    }
}
