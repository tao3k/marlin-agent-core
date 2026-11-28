// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Loopback HTTP/2 `tonic` server lifecycle handle and thin `ToolRuntime`
//! client for `marlin.search.v1.SearchGateway`.

use std::net::SocketAddr;
use std::sync::{Arc, RwLock};

use marlin_agent_protocol::{ModelToolCall, ModelToolName};
use marlin_agent_runtime::{
    ModelToolCapability, ModelToolDeclaration, ModelToolOutcome, RuntimeContext, RuntimeFuture,
    RuntimeTask, TokioAgentRuntime, ToolDeclarationRequest, ToolParameterName, ToolParameterSpec,
    ToolParameterSpecRequest, ToolParameterType, ToolRuntime,
};
use marlin_agent_search_engine::SearchDslSessionId;
use tokio::sync::oneshot;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::transport::{Channel, Endpoint, Server};

use super::proto::search_gateway_client::SearchGatewayClient;
use super::proto::search_gateway_server::SearchGatewayServer;
use super::proto::{
    ResetSessionRequest, SearchStepRequest, ToolParameterEntry, ToolSchemaEntry,
};
use super::service::SearchGatewayService;

/// Loopback HTTP/2 `tonic` gRPC server hosting [`SearchGatewayService`] on an
/// ephemeral `127.0.0.1:0` port supervised by a [`TokioAgentRuntime`] task.
pub struct LoopbackSearchGrpcServer {
    addr: SocketAddr,
    client: SearchGatewayClient<Channel>,
    shutdown_tx: Option<oneshot::Sender<()>>,
    _server_task: RuntimeTask<()>,
}

impl LoopbackSearchGrpcServer {
    /// Bind on `127.0.0.1:0` and spawn the tonic gRPC server onto the current
    /// Tokio runtime, returning a connected lazy-HTTP/2 [`SearchGatewayClient`].
    pub fn spawn(
        service: SearchGatewayService,
        runtime: &TokioAgentRuntime,
    ) -> Result<Self, String> {
        Self::spawn_on(&tokio::runtime::Handle::current(), service, runtime)
    }

    /// Bind on `127.0.0.1:0` and spawn the tonic gRPC server onto an explicit
    /// Tokio `handle` (usable from synchronous startup code such as TUI boot).
    pub fn spawn_on(
        handle: &tokio::runtime::Handle,
        service: SearchGatewayService,
        runtime: &TokioAgentRuntime,
    ) -> Result<Self, String> {
        let _enter = handle.enter();
        let std_listener = std::net::TcpListener::bind("127.0.0.1:0")
            .map_err(|e| format!("bind loopback search gRPC listener: {e}"))?;
        std_listener
            .set_nonblocking(true)
            .map_err(|e| format!("configure nonblocking search gRPC listener: {e}"))?;
        let addr = std_listener
            .local_addr()
            .map_err(|e| format!("read search gRPC listen addr: {e}"))?;
        let listener = tokio::net::TcpListener::from_std(std_listener)
            .map_err(|e| format!("adopt tokio TcpListener for search gRPC: {e}"))?;
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

        let server_task = runtime.spawn(async move {
            let incoming = TcpListenerStream::new(listener);
            let _ = Server::builder()
                .add_service(SearchGatewayServer::new(service))
                .serve_with_incoming_shutdown(incoming, async {
                    let _ = shutdown_rx.await;
                })
                .await;
        });

        let endpoint = Endpoint::from_shared(format!("http://{addr}"))
            .map_err(|e| format!("invalid loopback search gRPC endpoint http://{addr}: {e}"))?;
        let channel = endpoint.connect_lazy();
        let client = SearchGatewayClient::new(channel);
        Ok(Self {
            addr,
            client,
            shutdown_tx: Some(shutdown_tx),
            _server_task: server_task,
        })
    }

    /// Loopback TCP address serving `marlin.search.v1.SearchGateway`.
    #[must_use]
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// Clone a lightweight HTTP/2 [`SearchGatewayClient`] sharing the underlying
    /// multiplexed tonic channel.
    #[must_use]
    pub fn client(&self) -> SearchGatewayClient<Channel> {
        self.client.clone()
    }

    /// Reset a server-side search session over gRPC (`ResetSession`).
    pub async fn reset_session(&self, session_id: &SearchDslSessionId) -> Result<bool, String> {
        let mut client = self.client.clone();
        let resp = client
            .reset_session(ResetSessionRequest {
                session_id: session_id.as_str().to_owned(),
            })
            .await
            .map_err(|e| format!("search gRPC ResetSession failed: {e}"))?;
        Ok(resp.into_inner().cleared)
    }
}

impl Drop for LoopbackSearchGrpcServer {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

/// Convert declared read-only search [`ModelToolDeclaration`]s into protobuf
/// [`ToolSchemaEntry`] records served by `GetToolSchema`, preserving the full
/// multi-parameter typed schema declared in Scheme.
#[must_use]
pub fn tool_schema_entries_from_declarations(
    declarations: &[ModelToolDeclaration],
) -> Vec<ToolSchemaEntry> {
    declarations
        .iter()
        .map(|decl| ToolSchemaEntry {
            name: decl.definition().name.as_str().to_owned(),
            description: decl.definition().description.clone(),
            argument_name: decl.argument().to_owned(),
            timeout_seconds: u32::try_from(decl.timeout().as_secs()).unwrap_or(u32::MAX),
            output_limit: u32::try_from(decl.output_limit()).unwrap_or(u32::MAX),
            read_only: true,
            parameters: decl
                .parameters()
                .iter()
                .map(|param| ToolParameterEntry {
                    name: param.name().as_str().to_owned(),
                    param_type: param.param_type().as_str().to_owned(),
                    description: param.description().to_owned(),
                    required: param.required(),
                    allowed_values: param.allowed_values().to_vec(),
                })
                .collect(),
        })
        .collect()
}

/// Reconstruct a validated read-only search [`ModelToolDeclaration`] from a
/// remote `marlin.search.v1.SearchGateway/GetToolSchema` [`ToolSchemaEntry`],
/// preserving the exact B11 SHA-256 [`marlin_agent_runtime::schema_digest_for_definition`].
pub fn declaration_from_tool_schema_entry(
    entry: &ToolSchemaEntry,
) -> Result<ModelToolDeclaration, &'static str> {
    if !entry.read_only {
        return Err("search gateway schema entry must certify read_only=true");
    }
    let name = ModelToolName::new(entry.name.clone())?;
    let parameters = entry
        .parameters
        .iter()
        .map(|param| {
            ToolParameterSpec::from_request(ToolParameterSpecRequest {
                name: ToolParameterName::new(param.name.clone())?,
                param_type: ToolParameterType::parse_str(&param.param_type)?,
                description: param.description.clone(),
                required: param.required,
                allowed_values: param.allowed_values.clone(),
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    ModelToolDeclaration::from_request(ToolDeclarationRequest {
        name,
        description: entry.description.clone(),
        capability: ModelToolCapability::Search,
        parameters,
        timeout_seconds: u64::from(entry.timeout_seconds),
        output_limit: usize::try_from(entry.output_limit).map_err(|_| "output limit overflow")?,
    })
}

/// Thin `ToolRuntime` client forwarding search tool invocations directly over
/// `marlin.search.v1.SearchGateway/SearchStep` with zero client-side state.
#[derive(Clone)]
pub struct GrpcSearchToolHandler {
    declaration: ModelToolDeclaration,
    client: SearchGatewayClient<Channel>,
    session_id: Arc<RwLock<SearchDslSessionId>>,
}

impl GrpcSearchToolHandler {
    /// Construct a thin gRPC search tool handler bound to `declaration`,
    /// `client`, and a shared logical `session_id` cell.
    #[must_use]
    pub fn new(
        declaration: ModelToolDeclaration,
        client: SearchGatewayClient<Channel>,
        session_id: Arc<RwLock<SearchDslSessionId>>,
    ) -> Self {
        Self {
            declaration,
            client,
            session_id,
        }
    }
}

impl ToolRuntime for GrpcSearchToolHandler {
    type Invocation = ModelToolCall;
    type Output = ModelToolOutcome;

    fn run_tool(
        &self,
        call: ModelToolCall,
        _context: RuntimeContext,
    ) -> RuntimeFuture<Self::Output> {
        let declaration = self.declaration.clone();
        let mut client = self.client.clone();
        let session_id = self
            .session_id
            .read()
            .map(|g| g.as_str().to_owned())
            .unwrap_or_else(|_| "default".to_owned());
        Box::pin(async move {
            if !declaration.admits(&call) {
                return ModelToolOutcome::Failed("invalid search tool arguments".into());
            }
            let query = call.arguments[declaration.argument()]
                .as_str()
                .expect("admitted string query")
                .to_owned();
            let req = SearchStepRequest {
                session_id,
                tool_name: declaration.definition().name.as_str().to_owned(),
                query,
                output_limit: u32::try_from(declaration.output_limit()).unwrap_or(u32::MAX),
            };
            match client.search_step(req).await {
                Ok(resp) => ModelToolOutcome::Completed(resp.into_inner().rendered_text),
                Err(status) => ModelToolOutcome::Failed(status.message().to_owned()),
            }
        })
    }
}
