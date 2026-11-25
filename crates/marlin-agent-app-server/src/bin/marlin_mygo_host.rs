// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Production-grade Marlin MyGo Authority Host daemon serving all 6 orthogonal
//! `marlin.{app,ui,gate,plugin,device,projection}.v1` gRPC services over HTTP/2
//! (loopback sidecar plaintext with ephemeral Bearer token, or TLS 1.3 via
//! `MARLIN_BRIDGE_TLS_CERT` + `MARLIN_BRIDGE_TLS_KEY`).
//!
//! Set `MARLIN_BRIDGE_REAL_SESSION=1` to co-host the full durable `AppServer`
//! (`SessionBinding<InMemoryAgentStorage>` + `ConversationJournal` + deterministic
//! local multi-chunk `ModelStreamingGateway`) alongside `MyGoBridgeHost`'s five
//! UI, Gate, Plugin, Device, and Projection services on the same HTTP/2 listener.

use std::{
    collections::BTreeMap,
    env, fs,
    io::{self, Write},
    sync::Arc,
    time::Duration,
};

use marlin_agent_app_server::{
    AppServer, SessionBinding, SessionOpen,
    ui_grpc::MyGoBridgeHost,
    wire::marlin::gate::v1::{GateKind, PendingGate},
};
use marlin_agent_protocol::{
    ModelEndpoint, ModelGatewayCompletionChoice, ModelGatewayCompletionResponse,
    ModelGatewayFuture, ModelGatewayRequest, ModelGatewayResult, ModelGatewayTransport,
    assistant_gateway_message,
};
use marlin_agent_runtime::{
    TokioAgentRuntime, TokioAgentRuntimeBuildRequest,
    plugin::{PublisherId, ReleaseTrust},
};
use marlin_agent_sessions::{AgentSessionContext, ContextVisibility};
use marlin_agent_storage::{AgentId, InMemoryAgentStorage, ProjectId};
use marlin_agent_stream::{
    ModelGatewayEventStream, ModelStreamChunk, ModelStreamEvent, ModelStreamingGateway,
};
use tokio::{net::TcpListener, sync::Mutex};
use tokio_stream::wrappers::TcpListenerStream;
use tonic::{
    Status,
    metadata::MetadataMap,
    transport::{Identity, Server, ServerTlsConfig},
};

/// Deterministic local multi-chunk streaming gateway used when
/// `MARLIN_BRIDGE_REAL_SESSION=1` is enabled without any live external LLM calls.
struct DeterministicLocalGateway {
    chunk_delay: Duration,
}

impl ModelStreamingGateway for DeterministicLocalGateway {
    fn stream(
        &self,
        request: ModelGatewayRequest,
    ) -> ModelGatewayFuture<ModelGatewayResult<ModelGatewayEventStream>> {
        let prompt = request
            .messages()
            .last()
            .map(|msg| msg.content.clone())
            .unwrap_or_default();
        let delay = self.chunk_delay;
        Box::pin(async move {
            let chunk1 = "[marlin-core] ".to_owned();
            let chunk2 = format!("processed \"{prompt}\" ");
            let chunk3 = "via durable ConversationJournal.".to_owned();
            let full_reply = format!("{chunk1}{chunk2}{chunk3}");
            let stream = async_stream::try_stream! {
                yield ModelStreamEvent::Started {
                    transport: ModelGatewayTransport::Sse,
                };
                if !delay.is_zero() {
                    tokio::time::sleep(delay).await;
                }
                yield ModelStreamEvent::Chunk(ModelStreamChunk::new(1, chunk1));
                if !delay.is_zero() {
                    tokio::time::sleep(delay).await;
                }
                yield ModelStreamEvent::Chunk(ModelStreamChunk::new(2, chunk2));
                if !delay.is_zero() {
                    tokio::time::sleep(delay).await;
                }
                yield ModelStreamEvent::Chunk(ModelStreamChunk::new(3, chunk3));
                yield ModelStreamEvent::Completed {
                    response: ModelGatewayCompletionResponse::new(
                        "marlin-local-deterministic",
                        "gpt-marlin-local",
                        vec![ModelGatewayCompletionChoice::new(
                            0,
                            assistant_gateway_message(full_reply),
                            None,
                        )],
                    ),
                };
            };
            Ok(Box::pin(stream) as ModelGatewayEventStream)
        })
    }
}

fn decode_hex_32(hex: &str) -> Result<[u8; 32], String> {
    let trimmed = hex.trim();
    if trimmed.len() != 64 {
        return Err(format!("expected 64 hex chars, got {}", trimmed.len()));
    }
    let mut out = [0u8; 32];
    for (i, slot) in out.iter_mut().enumerate() {
        let byte = u8::from_str_radix(&trimmed[i * 2..i * 2 + 2], 16)
            .map_err(|e| format!("invalid hex at {i}: {e}"))?;
        *slot = byte;
    }
    Ok(out)
}

fn check_bearer(meta: &MetadataMap, expected_header: &str) -> Result<(), Status> {
    match meta.get("authorization") {
        Some(v) if v.to_str().ok() == Some(expected_header) => Ok(()),
        _ => Err(Status::unauthenticated("invalid bearer token")),
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut sig) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            sig.recv().await;
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {}
        _ = terminate => {}
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bind_addr = env::var("MARLIN_BRIDGE_ADDR").unwrap_or_else(|_| "127.0.0.1:0".to_owned());
    let token = env::var("MARLIN_BRIDGE_TOKEN").unwrap_or_else(|_| "marlin-dev-token".to_owned());
    let publisher_id =
        env::var("MARLIN_BRIDGE_PUBLISHER").unwrap_or_else(|_| "tao3k.official".to_owned());
    let real_session =
        env::var("MARLIN_BRIDGE_REAL_SESSION").unwrap_or_else(|_| "0".to_owned()) == "1";

    let mut keys = BTreeMap::new();
    if let Ok(pub_hex) = env::var("MARLIN_BRIDGE_PUBKEY_HEX")
        && !pub_hex.trim().is_empty()
    {
        let pub_bytes = decode_hex_32(&pub_hex)?;
        keys.insert(PublisherId::try_from(publisher_id)?, pub_bytes);
    }

    let expected_header = Arc::<str>::from(format!("Bearer {token}"));
    let bridge_header = Arc::clone(&expected_header);
    let host = MyGoBridgeHost::new(ReleaseTrust::new(keys), move |meta| {
        check_bearer(meta, &bridge_header)
    });

    if env::var("MARLIN_BRIDGE_SEED_DEMO_GATE").unwrap_or_else(|_| "1".to_owned()) != "0" {
        let gate_id = env::var("MARLIN_BRIDGE_GATE_ID").unwrap_or_else(|_| "gate-e2e-1".to_owned());
        host.enqueue_pending_gate(PendingGate {
            gate_id,
            kind: GateKind::DeviceCapability as i32,
            turn: None,
            call_id: Some("call-e2e-1".to_owned()),
            tool_or_node_name: "device.ios.notify.send".to_owned(),
            prompt: "Approve device notification delivery on iPhone?".to_owned(),
            details_heading: "platform.ios Notifications".to_owned(),
            choices: Vec::new(),
            card: None,
            created_at_unix_ms: 1,
            expires_at_unix_ms: None,
        });
    }

    let mut builder = Server::builder();
    if let (Ok(cert_path), Ok(key_path)) = (
        env::var("MARLIN_BRIDGE_TLS_CERT"),
        env::var("MARLIN_BRIDGE_TLS_KEY"),
    ) && !cert_path.trim().is_empty()
        && !key_path.trim().is_empty()
    {
        let cert_pem = fs::read(cert_path)?;
        let key_pem = fs::read(key_path)?;
        let identity = Identity::from_pem(cert_pem, key_pem);
        builder = builder.tls_config(ServerTlsConfig::new().identity(identity))?;
    }

    let router = if real_session {
        let delay_ms = env::var("MARLIN_BRIDGE_CHUNK_DELAY_MS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(5);
        let storage = InMemoryAgentStorage::default();
        let project = ProjectId::new("default")?;
        let agent = AgentId::new("assistant")?;
        let bindings = Arc::new(Mutex::new(BTreeMap::<
            String,
            SessionBinding<InMemoryAgentStorage>,
        >::new()));
        let app_header = Arc::clone(&expected_header);
        let app_server = AppServer::new(
            ModelEndpoint::new("openai", "gpt-marlin-local"),
            Arc::new(DeterministicLocalGateway {
                chunk_delay: Duration::from_millis(delay_ms),
            }),
            move |meta| check_bearer(meta, &app_header),
        )?
        .with_session_lookup(move |_metadata, id, mode| {
            let bindings = Arc::clone(&bindings);
            let storage = storage.clone();
            let project = project.clone();
            let agent = agent.clone();
            async move {
                let mut guard = bindings.lock().await;
                match mode {
                    SessionOpen::Create => {
                        if guard.contains_key(id.as_str()) {
                            return Err(Status::already_exists("session already exists"));
                        }
                        if guard.len() >= 64 {
                            return Err(Status::resource_exhausted("host session limit reached"));
                        }
                        let (runtime, _events) = TokioAgentRuntime::from_build_request(
                            TokioAgentRuntimeBuildRequest::runtime_root(16).with_session(
                                AgentSessionContext::root(
                                    id.as_str(),
                                    ContextVisibility::default_runtime(),
                                ),
                            ),
                        );
                        let binding = SessionBinding::new(storage, project, agent, runtime)
                            .map_err(|_| Status::internal("failed to bind session runtime"))?;
                        guard.insert(id.as_str().to_owned(), binding.clone());
                        Ok(binding)
                    }
                    SessionOpen::Existing => guard
                        .get(id.as_str())
                        .cloned()
                        .ok_or_else(|| Status::not_found("session not opened")),
                }
            }
        });
        app_server.into_ui_router_on(builder, host)
    } else {
        host.attach_ui_services(builder.add_service(host.app_service()))
    };

    let listener = TcpListener::bind(&bind_addr).await?;
    let local_addr = listener.local_addr()?;

    println!("READY {local_addr}");
    io::stdout().flush()?;

    router
        .serve_with_incoming_shutdown(TcpListenerStream::new(listener), shutdown_signal())
        .await?;
    Ok(())
}
