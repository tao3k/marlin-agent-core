// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Production Marlin Scheme Authority Host daemon (`marlin-scheme-host`).
//!
//! Architecture:
//! - Main OS thread owns the single-threaded embedded Gambit/Gerbil Scheme VM
//!   (`GerbilRuntime`) and serves live `GraphEngineRootedOracle` requests over a
//!   bounded `GraphEngineOracleServer` channel.
//! - Dedicated multi-threaded Tokio OS thread hosts the co-hosted HTTP/2
//!   `AppServer::control_plane_only` + `MyGoBridgeHost` backed by real disk
//!   `TursoAgentStorage` WAL (`MARLIN_BRIDGE_TURSO_PATH`) and fail-closed
//!   `TaskActorControlPlane::with_oracle(Arc<GraphEngineOracleHandle>)`.
//!
//! Prints `READY <local_addr>` to stdout once listening and flushes Turso WAL
//! cleanly on `SIGINT` / `SIGTERM`.

use std::{
    env,
    io::{self, Write},
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

use gerbil_scheme::{GerbilRuntime, IdentifierKind, PackageResolutionProgress};
use marlin_agent_app_server::{
    AppServer, TaskActorControlPlane, TaskActorPersistenceHandle,
    ui_grpc::MyGoBridgeHost,
    wire::marlin::gate::v1::{GateKind, PendingGate},
};
use marlin_agent_storage::{
    AgentId, ArtifactPointerKey, ProjectId, TursoAgentStorage, TursoAgentStorageConfig,
    TursoBatchTransactionMode, TursoOptimizationProfile,
};
use marlin_gerbil_scheme::{
    GraphEngineRootedOracle, gerbil_package_root, graph_engine_oracle_channel,
};

#[cfg(unix)]
struct ShutdownWatcher {
    sigterm: tokio::signal::unix::Signal,
}

#[cfg(unix)]
impl ShutdownWatcher {
    fn install() -> Result<Self, String> {
        let sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .map_err(|err| format!("install SIGTERM handler: {err}"))?;
        Ok(Self { sigterm })
    }

    async fn wait(mut self) {
        let ctrl_c = async {
            let _ = tokio::signal::ctrl_c().await;
        };
        tokio::select! {
            _ = ctrl_c => {}
            _ = self.sigterm.recv() => {}
        }
    }
}

#[cfg(not(unix))]
struct ShutdownWatcher;

#[cfg(not(unix))]
impl ShutdownWatcher {
    fn install() -> Result<Self, String> {
        Ok(Self)
    }

    async fn wait(self) {
        let _ = tokio::signal::ctrl_c().await;
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bind_addr = env::var("MARLIN_BRIDGE_ADDR").unwrap_or_else(|_| "127.0.0.1:0".to_owned());
    let token = env::var("MARLIN_BRIDGE_TOKEN").unwrap_or_default();
    let expected_header: Option<Arc<str>> = if token.is_empty() {
        None
    } else {
        Some(Arc::from(format!("Bearer {token}")))
    };
    let turso_path = env::var("MARLIN_BRIDGE_TURSO_PATH")
        .ok()
        .map(|p| p.trim().to_owned())
        .filter(|p| !p.is_empty());
    let seed_gate_id = env::var("MARLIN_BRIDGE_GATE_ID")
        .ok()
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty());
    let scheme_dir = env::var("MARLIN_SCHEME_DIR")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(gerbil_package_root);

    // 1. Initialize live Gambit/Gerbil Scheme VM strictly on the main OS thread.
    let runtime = GerbilRuntime::initialize()?;
    let _ = runtime.set_package_resolution_progress(PackageResolutionProgress::Quiet);
    let scheme_dir_str = scheme_dir
        .to_str()
        .ok_or("MARLIN_SCHEME_DIR is not valid UTF-8")?;
    let module_sym = runtime.intern_identifier(
        IdentifierKind::Symbol,
        ":marlin-agent-core/graph-engine/funcs",
    )?;
    let resolve = |proc_name: &str| {
        let name_sym = runtime.intern_identifier(IdentifierKind::Symbol, proc_name)?;
        runtime.resolve_package_export(scheme_dir_str, &module_sym, &name_sym)
    };
    let live_oracle = GraphEngineRootedOracle::new(
        &runtime,
        resolve("marlin-graph-engine-project-dispatch-vector")?,
        resolve("marlin-graph-engine-reconcile-receipt-vector")?,
        resolve("marlin-graph-engine-project-workbench-tabs-vector")?,
        resolve("marlin-graph-engine-project-governed-context-vector")?,
    );

    let (oracle_handle, oracle_server) = graph_engine_oracle_channel(64);
    let oracle_arc: Arc<dyn marlin_agent_graph::GraphEngineOracle + Send + Sync> =
        Arc::new(oracle_handle);

    // 2. Spawn multi-threaded Tokio HTTP/2 AppServer + MyGoBridgeHost on a background OS thread.
    let server_thread = std::thread::spawn(move || -> Result<(), String> {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .map_err(|err| format!("build tokio runtime: {err}"))?;
        let result = rt.block_on(async move {
            let mut persistence_handle: Option<TaskActorPersistenceHandle> = None;
            let task_actors = if let Some(path) = turso_path {
                let storage = TursoAgentStorage::open_local(TursoAgentStorageConfig {
                    path: PathBuf::from(path),
                    optimization_profile: TursoOptimizationProfile::AsyncIoOnlyCompatibility,
                    batch_transaction_mode: TursoBatchTransactionMode::Immediate,
                })
                .await
                .map_err(|err| format!("open turso storage: {err}"))?;
                let project =
                    ProjectId::new("default").map_err(|err| format!("project id: {err}"))?;
                let pointer = ArtifactPointerKey::new("pointer:graph-engine:scheme-host-actors")
                    .map_err(|err| format!("pointer key: {err}"))?;
                let agent =
                    AgentId::new("marlin-scheme-host").map_err(|err| format!("agent id: {err}"))?;
                let plane = TaskActorControlPlane::rehydrate_or_new(&storage, &project, &pointer)
                    .await
                    .map_err(|err| format!("rehydrate control plane: {err}"))?
                    .with_oracle(oracle_arc);
                persistence_handle =
                    Some(plane.spawn_storage_persistence_loop(storage, project, pointer, agent));
                plane
            } else {
                TaskActorControlPlane::new().with_oracle(oracle_arc)
            };

            let app_auth = expected_header.clone();
            let bridge_auth = expected_header;
            let host = MyGoBridgeHost::new_default_trust(move |meta| {
                MyGoBridgeHost::check_bearer_header(meta, bridge_auth.as_deref())
            })
            .with_task_actors(task_actors.clone());

            if let Some(gate_id) = seed_gate_id {
                host.enqueue_pending_gate(PendingGate {
                    gate_id,
                    kind: GateKind::GraphHumanGate as i32,
                    turn: None,
                    call_id: None,
                    tool_or_node_name: "scheme-host-human-gate".to_owned(),
                    prompt: "Approve Scheme-governed GraphEngine action?".to_owned(),
                    details_heading: "Marlin Scheme Authority Host Gate".to_owned(),
                    choices: Vec::new(),
                    card: None,
                    created_at_unix_ms: 1,
                    expires_at_unix_ms: None,
                });
            }

            let app = AppServer::control_plane_only(move |meta| {
                MyGoBridgeHost::check_bearer_header(meta, app_auth.as_deref())
            })
            .with_task_actors(task_actors);

            let listener = tokio::net::TcpListener::bind(&bind_addr)
                .await
                .map_err(|err| format!("bind {bind_addr}: {err}"))?;
            let local_addr = listener
                .local_addr()
                .map_err(|err| format!("local_addr: {err}"))?;

            // Install OS signal handler BEFORE advertising READY to eliminate startup races.
            let shutdown = ShutdownWatcher::install()?;
            let (stop_tx, stop_rx) = tokio::sync::oneshot::channel::<()>();
            let mut srv_task = tokio::spawn(async move {
                app.serve_with_ui_on_shutdown(host, listener, async move {
                    let _ = stop_rx.await;
                })
                .await
            });

            println!("READY {local_addr}");
            io::stdout()
                .flush()
                .map_err(|err| format!("flush stdout: {err}"))?;

            tokio::select! {
                _ = shutdown.wait() => {
                    let _ = stop_tx.send(());
                    if tokio::time::timeout(Duration::from_secs(2), &mut srv_task)
                        .await
                        .is_err()
                    {
                        srv_task.abort();
                        let _ = srv_task.await;
                    }
                }
                res = &mut srv_task => {
                    match res {
                        Ok(Ok(())) => {}
                        Ok(Err(err)) => return Err(format!("serve HTTP/2: {err}")),
                        Err(err) => return Err(format!("server task join: {err}")),
                    }
                }
            }

            if let Some(handle) = persistence_handle {
                handle
                    .flush_and_stop()
                    .await
                    .map_err(|err| format!("flush persistence: {err}"))?;
            }
            Ok(())
        });
        rt.shutdown_timeout(Duration::from_secs(2));
        result
    });

    // 3. Serve live Scheme Oracle requests on the Gambit VM-owner thread until shutdown.
    oracle_server.serve_until_closed(&live_oracle);
    drop(live_oracle);

    let final_res = match server_thread.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(err)) => Err(err),
        Err(_) => Err("marlin-scheme-host server thread panicked".to_owned()),
    };
    let _ = io::stdout().flush();
    let _ = io::stderr().flush();
    // All HTTP/2 servers, Tokio tasks, and Turso WAL checkpoints have already flushed cleanly.
    // Exit directly so Gambit's C `___cleanup()` cannot block process teardown.
    match final_res {
        Ok(()) => std::process::exit(0),
        Err(err) => {
            eprintln!("marlin-scheme-host error: {err}");
            std::process::exit(1);
        }
    }
}
