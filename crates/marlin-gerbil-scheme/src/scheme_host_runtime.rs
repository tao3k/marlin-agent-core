// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Production Marlin Scheme Authority Host daemon (`marlin-scheme-host`).
//!
//! Architecture:
//! - Main OS thread owns the single-threaded embedded Gambit/Gerbil Scheme VM
//!   (`GerbilRuntime`) and multiplexes:
//!   1. Live `GraphEngineRootedOracle` validation/projection requests (`GraphEngineOracleServer`)
//!   2. Optional live `SchemeFleetSessionDriver` rooted in the Gambit heap when
//!      `MARLIN_SCHEME_FLEET_CHECKPOINT_KEY` is configured (bootstrapping from or restoring
//!      disk `TursoAgentStorage` CAS and applying HTTP/2 `ApprovalGateService` human approvals).
//! - Dedicated multi-threaded Tokio OS thread hosts the co-hosted HTTP/2
//!   `AppServer::control_plane_only` + `MyGoBridgeHost` backed by real disk
//!   `TursoAgentStorage` WAL (`MARLIN_BRIDGE_TURSO_PATH`), fail-closed
//!   `TaskActorControlPlane::with_oracle(Arc<GraphEngineOracleHandle>)`, and thin
//!   domain-agnostic `StorageCasStore` atomic effect execution.
//!
//! Prints `READY <local_addr>` to stdout once listening and flushes Turso WAL
//! cleanly on `SIGINT` / `SIGTERM`.

use std::{
    collections::HashMap,
    env,
    io::{self, Write},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
    time::Duration,
};

use crate::{
    AtomicEffectAbiError, FleetAuditCapsuleProjection, GraphEngineLinkedBridge,
    GraphEngineLinkedExports, SchemeAtomicEffect, SchemeAtomicReceipt, SchemeFleetDispatchStep,
    SchemeFleetLinkedDriver, SchemeFleetLinkedExports, SchemeFleetQuiescentSummary,
    encode_scheme_atomic_receipt_vector, graph_engine_oracle_channel,
};
#[cfg(not(feature = "external-program"))]
use crate::{GraphEngineRootedOracle, SchemeFleetSessionDriver, gerbil_package_root};
use gerbil_scheme::{CollectionKind, GerbilRuntime};
#[cfg(not(feature = "external-program"))]
use gerbil_scheme::{IdentifierKind, PackageResolutionProgress, RootedSchemeObject};
use marlin_agent_app_server::{
    AllowlistSandboxRunner, AppServer, OpenAiCompatibleHttpClient, OpenAiHttpConfig,
    RegisterTaskActorRequest, StorageCasStore, TaskActorControlPlane, TaskActorPersistenceHandle,
    ui_grpc::MyGoBridgeHost,
    wire::marlin::{
        gate::v1::{GateKind, GraphHumanDecision, PendingGate},
        projection::v1::{GraphRecordView, GraphTrajectoryEvent, GraphVisibleSurface},
    },
};
use marlin_agent_graph::{
    DetourMode, DisclosureDepth, DispatchRequestId, ExecutionReceiptStatus, FencingEpoch,
    GovernedContextCapsule, GraphDispatchOperation, GraphSubstrateTarget, KvPrefixDigest,
    MarlinDispatchEnvelope, MarlinExecutionReceipt, TaskActorId, TaskActorScopeMode,
};
use marlin_agent_storage::{
    AgentId, ArtifactPointerKey, ProjectId, SessionId, TursoAgentStorage, TursoAgentStorageConfig,
    TursoBatchTransactionMode, TursoOptimizationProfile,
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

struct FleetBootReport {
    mode: &'static str,
    capsule: FleetAuditCapsuleProjection,
}

enum FleetVmCommand {
    Bootstrap {
        checkpoint_key: String,
        reply: mpsc::SyncSender<Result<FleetBootReport, String>>,
    },
    HumanGateApproved {
        gate_id: String,
        note: Option<String>,
        reply: mpsc::SyncSender<Result<FleetAuditCapsuleProjection, String>>,
    },
    HumanGateRejected {
        gate_id: String,
        reply: mpsc::SyncSender<Result<FleetAuditCapsuleProjection, String>>,
    },
    UiAction {
        action_sym: String,
        arg: Option<String>,
        reply: mpsc::SyncSender<
            Result<(SchemeFleetDispatchStep, FleetAuditCapsuleProjection), String>,
        >,
    },
}

struct EffectBatchRequest {
    effects: Vec<SchemeAtomicEffect>,
    reply: mpsc::SyncSender<Vec<SchemeAtomicReceipt>>,
}

/// All 9 statically linked C-ABI function pointers exported by `scheme/c-abi/graph-engine.ss`
/// required to run `marlin-scheme-host-aot` without dynamic `.o1` `dlopen`.
#[derive(Clone, Copy, Debug)]
pub struct LinkedSchemeHostExports {
    pub dispatch_vector: unsafe extern "C" fn(i64) -> i64,
    pub reconcile_receipt_vector: unsafe extern "C" fn(i64) -> i64,
    pub workbench_tabs_vector: unsafe extern "C" fn(i64) -> i64,
    pub governed_context_vector: unsafe extern "C" fn(i64) -> i64,
    pub make_demo_fleet_vector: unsafe extern "C" fn(i64) -> i64,
    pub fleet_restore_vector: unsafe extern "C" fn(i64) -> i64,
    pub fleet_open_vector: unsafe extern "C" fn(i64) -> i64,
    pub fleet_dispatch_vector: unsafe extern "C" fn(i64) -> i64,
    pub fleet_audit_capsule_vector: unsafe extern "C" fn(i64) -> i64,
}

#[derive(Clone, Copy)]
struct LinkedFleetBootstrapExports {
    make_demo_fleet_vector: unsafe extern "C" fn(i64) -> i64,
    fleet_restore_vector: unsafe extern "C" fn(i64) -> i64,
    fleet: SchemeFleetLinkedExports,
}

#[cfg(not(feature = "external-program"))]
struct FleetSchemeProcs<'vm> {
    make_demo_fleet_proc: RootedSchemeObject<'vm>,
    restore_fleet_proc: RootedSchemeObject<'vm>,
    open_proc: RootedSchemeObject<'vm>,
    dispatch_proc: RootedSchemeObject<'vm>,
    audit_proc: RootedSchemeObject<'vm>,
}

enum HostFleetBootstrap<'vm> {
    #[cfg(not(feature = "external-program"))]
    Dynamic(FleetSchemeProcs<'vm>),
    Linked(
        LinkedFleetBootstrapExports,
        std::marker::PhantomData<&'vm ()>,
    ),
}

enum HostFleetDriver<'vm> {
    #[cfg(not(feature = "external-program"))]
    Dynamic(SchemeFleetSessionDriver<'vm>),
    Linked(SchemeFleetLinkedDriver<'vm>),
}

impl<'vm> HostFleetDriver<'vm> {
    fn dispatch(
        &mut self,
        action_sym: &str,
        task_id: Option<&str>,
        arg: Option<&str>,
        receipts: &[SchemeAtomicReceipt],
        auto_archive: bool,
    ) -> Result<SchemeFleetDispatchStep, AtomicEffectAbiError> {
        match self {
            #[cfg(not(feature = "external-program"))]
            Self::Dynamic(d) => d.dispatch(action_sym, task_id, arg, receipts, auto_archive),
            Self::Linked(d) => d.dispatch(action_sym, task_id, arg, receipts, auto_archive),
        }
    }

    fn checkpoint(&mut self, cas_key: &str) -> Result<SchemeAtomicEffect, AtomicEffectAbiError> {
        match self {
            #[cfg(not(feature = "external-program"))]
            Self::Dynamic(d) => d.checkpoint(cas_key),
            Self::Linked(d) => d.checkpoint(cas_key),
        }
    }

    fn pump_until_quiescent<F>(
        &mut self,
        initial_receipts: &[SchemeAtomicReceipt],
        auto_archive: bool,
        max_rounds: usize,
        execute_batch: F,
    ) -> Result<SchemeFleetQuiescentSummary, AtomicEffectAbiError>
    where
        F: FnMut(Vec<SchemeAtomicEffect>) -> Result<Vec<SchemeAtomicReceipt>, AtomicEffectAbiError>,
    {
        match self {
            #[cfg(not(feature = "external-program"))]
            Self::Dynamic(d) => {
                d.pump_until_quiescent(initial_receipts, auto_archive, max_rounds, execute_batch)
            }
            Self::Linked(d) => {
                d.pump_until_quiescent(initial_receipts, auto_archive, max_rounds, execute_batch)
            }
        }
    }

    fn audit_capsule(&self) -> Result<FleetAuditCapsuleProjection, AtomicEffectAbiError> {
        match self {
            #[cfg(not(feature = "external-program"))]
            Self::Dynamic(d) => d.audit_capsule(),
            Self::Linked(d) => d.audit_capsule(),
        }
    }
}

fn execute_effects_on_tokio(
    effect_tx: &tokio::sync::mpsc::UnboundedSender<EffectBatchRequest>,
    effects: Vec<SchemeAtomicEffect>,
) -> Result<Vec<SchemeAtomicReceipt>, AtomicEffectAbiError> {
    let (reply_tx, reply_rx) = mpsc::sync_channel(1);
    effect_tx
        .send(EffectBatchRequest {
            effects,
            reply: reply_tx,
        })
        .map_err(|_| AtomicEffectAbiError("tokio atomic effect worker closed".to_owned()))?;
    reply_rx
        .recv()
        .map_err(|_| AtomicEffectAbiError("tokio atomic effect worker dropped reply".to_owned()))
}

fn assert_non_authorizing_capsule(capsule: &FleetAuditCapsuleProjection) -> Result<(), String> {
    if capsule.favored_path_collapsed || capsule.authenticates_evidence || capsule.authorizes_effect
    {
        return Err(format!(
            "fleet audit capsule violated non-authorizing invariants: {capsule:?}"
        ));
    }
    Ok(())
}

fn bootstrap_scheme_fleet<'vm>(
    runtime: &'vm GerbilRuntime,
    bootstrap: HostFleetBootstrap<'vm>,
    effect_tx: &tokio::sync::mpsc::UnboundedSender<EffectBatchRequest>,
    checkpoint_key: &str,
) -> Result<(HostFleetDriver<'vm>, FleetBootReport), String> {
    let probe = execute_effects_on_tokio(
        effect_tx,
        vec![SchemeAtomicEffect::CasGet {
            effect_id: "scheme-host-boot-probe".to_owned(),
            key: checkpoint_key.to_owned(),
        }],
    )
    .map_err(|err| format!("boot cas probe: {err}"))?;

    match probe.into_iter().next() {
        Some(SchemeAtomicReceipt::CasHit { bytes, .. }) => {
            let rcpt_vec = encode_scheme_atomic_receipt_vector(
                runtime,
                &SchemeAtomicReceipt::CasHit {
                    effect_id: "scheme-host-restore".to_owned(),
                    bytes,
                },
            )
            .map_err(|err| format!("encode restore receipt: {err}"))?;
            let driver = match bootstrap {
                #[cfg(not(feature = "external-program"))]
                HostFleetBootstrap::Dynamic(procs) => {
                    let restored_fleet = runtime
                        .call_rooted(&procs.restore_fleet_proc, &rcpt_vec)
                        .map_err(|err| format!("restore scheme fleet from cas: {err}"))?;
                    HostFleetDriver::Dynamic(
                        SchemeFleetSessionDriver::open(
                            runtime,
                            &procs.open_proc,
                            procs.dispatch_proc,
                            procs.audit_proc,
                            restored_fleet,
                            "L3",
                        )
                        .map_err(|err| format!("open restored fleet driver: {err}"))?,
                    )
                }
                HostFleetBootstrap::Linked(exports, _) => {
                    // SAFETY: exports belong to `runtime`'s initialized linked program.
                    let restore_proc =
                        unsafe { runtime.bind_object_export(exports.fleet_restore_vector) }
                            .map_err(|err| format!("bind linked fleet_restore_vector: {err}"))?;
                    let restored_fleet = restore_proc
                        .call(&rcpt_vec)
                        .into_result()
                        .map_err(|err| format!("linked restore scheme fleet from cas: {err}"))?;
                    HostFleetDriver::Linked(
                        unsafe {
                            SchemeFleetLinkedDriver::open(
                                runtime,
                                exports.fleet,
                                restored_fleet,
                                "L3",
                            )
                        }
                        .map_err(|err| format!("open linked restored fleet driver: {err}"))?,
                    )
                }
            };
            let capsule = driver
                .audit_capsule()
                .map_err(|err| format!("audit restored fleet: {err}"))?;
            assert_non_authorizing_capsule(&capsule)?;
            Ok((
                driver,
                FleetBootReport {
                    mode: "restored",
                    capsule,
                },
            ))
        }
        Some(SchemeAtomicReceipt::CasMiss(_)) => {
            let p_str = runtime
                .string_from_utf8("fleet-host-parent")
                .map_err(|err| format!("alloc parent id: {err}"))?
                .into_object();
            let s_str = runtime
                .string_from_utf8("fleet-host-sib")
                .map_err(|err| format!("alloc sibling id: {err}"))?
                .into_object();
            let kv_str = runtime
                .string_from_utf8("sha256:scheme-host-kv")
                .map_err(|err| format!("alloc kv digest: {err}"))?
                .into_object();
            let demo_in = runtime
                .collection_from_objects(CollectionKind::Vector, &[&p_str, &s_str, &kv_str])
                .map_err(|err| format!("build demo fleet input: {err}"))?
                .into_object();
            let mut driver = match bootstrap {
                #[cfg(not(feature = "external-program"))]
                HostFleetBootstrap::Dynamic(procs) => {
                    let fleet = runtime
                        .call_rooted(&procs.make_demo_fleet_proc, &demo_in)
                        .map_err(|err| format!("construct scheme demo fleet: {err}"))?;
                    HostFleetDriver::Dynamic(
                        SchemeFleetSessionDriver::open(
                            runtime,
                            &procs.open_proc,
                            procs.dispatch_proc,
                            procs.audit_proc,
                            fleet,
                            "L1",
                        )
                        .map_err(|err| format!("open cold fleet driver: {err}"))?,
                    )
                }
                HostFleetBootstrap::Linked(exports, _) => {
                    // SAFETY: exports belong to `runtime`'s initialized linked program.
                    let demo_proc =
                        unsafe { runtime.bind_object_export(exports.make_demo_fleet_vector) }
                            .map_err(|err| format!("bind linked make_demo_fleet_vector: {err}"))?;
                    let fleet = demo_proc
                        .call(&demo_in)
                        .into_result()
                        .map_err(|err| format!("linked construct scheme demo fleet: {err}"))?;
                    HostFleetDriver::Linked(
                        unsafe {
                            SchemeFleetLinkedDriver::open(runtime, exports.fleet, fleet, "L1")
                        }
                        .map_err(|err| format!("open linked cold fleet driver: {err}"))?,
                    )
                }
            };
            let _summary = driver
                .pump_until_quiescent(&[], true, 8, |effects| {
                    execute_effects_on_tokio(effect_tx, effects)
                })
                .map_err(|err| format!("pump scheme fleet to quiescence: {err}"))?;
            let cp_eff = driver
                .checkpoint(checkpoint_key)
                .map_err(|err| format!("plan cold fleet checkpoint: {err}"))?;
            let stored = execute_effects_on_tokio(effect_tx, vec![cp_eff])
                .map_err(|err| format!("store cold fleet checkpoint: {err}"))?;
            match stored.into_iter().next() {
                Some(SchemeAtomicReceipt::CasStored(_)) => {}
                other => {
                    return Err(format!(
                        "cold fleet checkpoint store failed with unexpected receipt: {other:?}"
                    ));
                }
            }
            let capsule = driver
                .audit_capsule()
                .map_err(|err| format!("audit cold fleet: {err}"))?;
            assert_non_authorizing_capsule(&capsule)?;
            Ok((
                driver,
                FleetBootReport {
                    mode: "cold",
                    capsule,
                },
            ))
        }
        other => Err(format!("unexpected boot cas probe receipt: {other:?}")),
    }
}

static TRAJECTORY_EVENT_SEQ: AtomicU64 = AtomicU64::new(1);

fn parse_openai_http_client_from_env() -> Result<Option<Arc<OpenAiCompatibleHttpClient>>, String> {
    let Ok(raw_url) = env::var("MARLIN_SCHEME_LLM_BASE_URL") else {
        return Ok(None);
    };
    let trimmed = raw_url.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let model_override = env::var("MARLIN_SCHEME_LLM_MODEL")
        .ok()
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty());
    let api_key = env::var("MARLIN_SCHEME_LLM_API_KEY")
        .ok()
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty());
    let timeout_ms = env::var("MARLIN_SCHEME_LLM_TIMEOUT_MS")
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .filter(|ms| *ms > 0)
        .unwrap_or(15_000);
    let client = OpenAiCompatibleHttpClient::new(OpenAiHttpConfig {
        base_url: trimmed.to_owned(),
        model_override,
        api_key,
        timeout: Duration::from_millis(timeout_ms),
    })?;
    Ok(Some(Arc::new(client)))
}

fn publish_fleet_audit_projection(
    host: &MyGoBridgeHost,
    node_id: &str,
    node_kind: &str,
    disclosure_level: &str,
    capsule: &FleetAuditCapsuleProjection,
) {
    let mut facets = HashMap::new();
    facets.insert(
        "active_actor_count".to_owned(),
        capsule.active_actor_count.to_string(),
    );
    facets.insert(
        "archived_actor_count".to_owned(),
        capsule.archived_actor_count.to_string(),
    );
    facets.insert("peek_count".to_owned(), capsule.peek_count.to_string());
    facets.insert(
        "total_active_defeaters".to_owned(),
        capsule.total_active_defeaters.to_string(),
    );
    facets.insert(
        "total_human_inbox_items".to_owned(),
        capsule.total_human_inbox_items.to_string(),
    );
    facets.insert("disclosure_level".to_owned(), disclosure_level.to_owned());
    facets.insert(
        "favored_path_collapsed".to_owned(),
        capsule.favored_path_collapsed.to_string(),
    );
    facets.insert(
        "authenticates_evidence".to_owned(),
        capsule.authenticates_evidence.to_string(),
    );
    facets.insert(
        "authorizes_effect".to_owned(),
        capsule.authorizes_effect.to_string(),
    );

    let record = GraphRecordView {
        record_id: "scheme:fleet:audit-capsule".to_owned(),
        surface: GraphVisibleSurface::Workspace as i32,
        kind: "scheme.fleet.audit_capsule".to_owned(),
        title: "Scheme Fleet Non-Authorizing Audit Capsule".to_owned(),
        summary: format!(
            "active={} archived={} defeaters={} inbox={} disclosure={disclosure_level}",
            capsule.active_actor_count,
            capsule.archived_actor_count,
            capsule.total_active_defeaters,
            capsule.total_human_inbox_items
        ),
        source_path: Some("scheme/graph-engine/fleet-controller.ss".to_owned()),
        start_line: None,
        end_line: None,
        facets: facets.clone(),
    };
    host.set_graph_projection_records(vec![record]);

    let seq = TRAJECTORY_EVENT_SEQ.fetch_add(1, Ordering::Relaxed);
    host.publish_graph_trajectory_event(GraphTrajectoryEvent {
        event_id: format!("scheme-ev-{seq}"),
        run_id: "scheme-fleet-live".to_owned(),
        node_id: node_id.to_owned(),
        node_kind: node_kind.to_owned(),
        status: "completed".to_owned(),
        node_receipt: None,
        metadata: facets,
    });
}

/// Strictly allowlists declarative UI commands that may be routed to Scheme
/// `FleetWorkbenchController.` via `DispatchUiEvent(RecordCommand)`.
///
/// Security invariant: `task-human-approve` is deliberately EXCLUDED so human approval
/// can NEVER bypass the one-shot `ApprovalGateService::submit_graph_human_decision` gate.
fn parse_scheme_ui_command(raw: &str) -> Option<(&'static str, Option<String>)> {
    let body = raw.trim().strip_prefix("scheme:")?.trim();
    let (action_raw, arg) = match body.split_once(char::is_whitespace) {
        Some((head, tail)) => {
            let trimmed_arg = tail.trim();
            let opt_arg = if trimmed_arg.is_empty() {
                None
            } else {
                Some(trimmed_arg.to_owned())
            };
            (head, opt_arg)
        }
        None => (body, None),
    };
    let action = match action_raw {
        "task-drill-l0" => "task-drill-l0",
        "task-drill-l1" => "task-drill-l1",
        "task-drill-l2" => "task-drill-l2",
        "task-drill-l3" => "task-drill-l3",
        "task-propose-branch" => "task-propose-branch",
        "task-detour-blocker" => "task-detour-blocker",
        "task-detour-sidequest" => "task-detour-sidequest",
        "task-detour-exit-return" => "task-detour-exit-return",
        "task-detour-exit-absorb" => "task-detour-exit-absorb",
        "task-detour-exit-abort" => "task-detour-exit-abort",
        "task-detour-exit-sibling" => "task-detour-exit-sibling",
        "task-peek" => "task-peek",
        "task-reopen" => "task-reopen",
        "task-fleet-step" => "task-fleet-step",
        "task-sandbox-verify" => "task-sandbox-verify",
        _ => return None,
    };
    Some((action, arg))
}

fn parse_disclosure_depth(raw: &str) -> DisclosureDepth {
    match raw {
        "L0" => DisclosureDepth::L0,
        "L1" => DisclosureDepth::L1,
        "L2" => DisclosureDepth::L2,
        _ => DisclosureDepth::L3,
    }
}

async fn run_one_atomic_effect(
    cas: Option<Arc<StorageCasStore<TursoAgentStorage>>>,
    sandbox: Option<Arc<AllowlistSandboxRunner>>,
    http_llm: Option<Arc<OpenAiCompatibleHttpClient>>,
    loopback_llm_response: Option<Arc<str>>,
    effect: SchemeAtomicEffect,
) -> SchemeAtomicReceipt {
    match effect {
        SchemeAtomicEffect::CasGet { effect_id, key } => {
            let Some(store) = cas else {
                return SchemeAtomicReceipt::EffectError {
                    effect_id,
                    message: "turso cas store not mounted on marlin-scheme-host".to_owned(),
                };
            };
            match store.get_blob(&key).await {
                Ok(Some(bytes)) => SchemeAtomicReceipt::CasHit { effect_id, bytes },
                Ok(None) => SchemeAtomicReceipt::CasMiss(effect_id),
                Err(message) => SchemeAtomicReceipt::EffectError { effect_id, message },
            }
        }
        SchemeAtomicEffect::CasPut {
            effect_id,
            key,
            bytes,
        } => {
            let Some(store) = cas else {
                return SchemeAtomicReceipt::EffectError {
                    effect_id,
                    message: "turso cas store not mounted on marlin-scheme-host".to_owned(),
                };
            };
            match store.put_blob(&key, bytes).await {
                Ok(()) => SchemeAtomicReceipt::CasStored(effect_id),
                Err(message) => SchemeAtomicReceipt::EffectError { effect_id, message },
            }
        }
        SchemeAtomicEffect::SandboxRun {
            effect_id,
            program,
            args,
            cwd,
        } => match sandbox {
            Some(runner) => match runner.run_command(program, args, cwd).await {
                Ok((exit_code, stdout, stderr)) => SchemeAtomicReceipt::SandboxExited {
                    effect_id,
                    exit_code,
                    stdout,
                    stderr,
                },
                Err(message) => SchemeAtomicReceipt::SandboxExited {
                    effect_id,
                    exit_code: 126,
                    stdout: Vec::new(),
                    stderr: format!("sandbox-denied: {message}").into_bytes(),
                },
            },
            None => SchemeAtomicReceipt::SandboxExited {
                effect_id,
                exit_code: 126,
                stdout: Vec::new(),
                stderr: b"sandbox-denied: no authorized sandbox policy mounted".to_vec(),
            },
        },
        SchemeAtomicEffect::LlmComplete { effect_id, request } => {
            if let Some(client) = http_llm {
                match client.complete_text(&request).await {
                    Ok(content) => SchemeAtomicReceipt::LlmCompleted { effect_id, content },
                    Err(message) => SchemeAtomicReceipt::EffectError { effect_id, message },
                }
            } else if let Some(content) = loopback_llm_response {
                SchemeAtomicReceipt::LlmCompleted {
                    effect_id,
                    content: content.as_ref().to_owned(),
                }
            } else {
                SchemeAtomicReceipt::EffectError {
                    effect_id,
                    message: "llm gateway not configured on marlin-scheme-host".to_owned(),
                }
            }
        }
    }
}

fn parse_sandbox_runner_from_env() -> Option<Arc<AllowlistSandboxRunner>> {
    let raw = env::var("MARLIN_SCHEME_SANDBOX_ALLOWLIST").ok()?;
    let allowed: Vec<String> = raw
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect();
    if allowed.is_empty() {
        return None;
    }
    let timeout_ms = env::var("MARLIN_SCHEME_SANDBOX_TIMEOUT_MS")
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .filter(|ms| *ms > 0)
        .unwrap_or(5_000);
    let mut runner =
        AllowlistSandboxRunner::new(Duration::from_millis(timeout_ms), 65_536, allowed);
    if let Ok(root) = env::var("MARLIN_SCHEME_SANDBOX_ROOT") {
        let trimmed = root.trim();
        if !trimmed.is_empty() {
            runner = runner.with_confined_root(trimmed);
        }
    }
    Some(Arc::new(runner))
}

fn sync_boot_fleet_to_control_plane(
    task_actors: &TaskActorControlPlane,
    capsule: &FleetAuditCapsuleProjection,
) -> Result<(), String> {
    let parent_id = TaskActorId::new("fleet-host-parent").map_err(|e| format!("{e}"))?;
    let sib_id = TaskActorId::new("fleet-host-sib").map_err(|e| format!("{e}"))?;
    let kv_digest = KvPrefixDigest::new("sha256:scheme-host-kv").map_err(|e| format!("{e}"))?;
    let snap = task_actors.snapshot();
    if !snap.actors.contains_key(&parent_id) {
        task_actors
            .register_actor(RegisterTaskActorRequest {
                task_id: parent_id.clone(),
                title: "Turso Fleet Parent".to_owned(),
                scope_mode: TaskActorScopeMode::Project,
            })
            .map_err(|e| format!("register fleet parent actor: {e}"))?;
    }
    if capsule.active_actor_count >= 2 {
        if !snap.actors.contains_key(&sib_id) {
            task_actors
                .register_actor(RegisterTaskActorRequest {
                    task_id: sib_id.clone(),
                    title: "Turso Fleet Sibling".to_owned(),
                    scope_mode: TaskActorScopeMode::Project,
                })
                .map_err(|e| format!("register reopened fleet sibling actor: {e}"))?;
        }
        let _ = task_actors.close_read_only_peek(&sib_id);
    } else {
        let _ = task_actors.open_read_only_peek(sib_id);
    }
    let has_defeater = capsule.total_active_defeaters > 0;
    task_actors
        .bind_governed_context(GovernedContextCapsule {
            task_id: parent_id,
            context_cut: 1,
            kv_prefix_digest: kv_digest,
            disclosure_depth: DisclosureDepth::L3,
            has_active_defeater: has_defeater,
            quarantined_memory_count: 0,
            sandbox_admitted: true,
            cedar_handoff_valid: true,
            detour_mode: if has_defeater {
                DetourMode::BlockerActive
            } else {
                DetourMode::Mainline
            },
            auto_archive_enabled: !has_defeater,
        })
        .map_err(|e| format!("bind fleet governed context: {e}"))?;
    Ok(())
}

fn sync_ui_action_fleet_to_control_plane(
    task_actors: &TaskActorControlPlane,
    action_sym: &str,
    arg: Option<&str>,
    step: &SchemeFleetDispatchStep,
    capsule: &FleetAuditCapsuleProjection,
) -> Result<(), String> {
    if action_sym == "task-reopen" {
        let sib_raw = arg.unwrap_or("fleet-host-sib");
        let sib_id = TaskActorId::new(sib_raw).map_err(|e| format!("{e}"))?;
        let snap = task_actors.snapshot();
        if !snap.actors.contains_key(&sib_id) {
            task_actors
                .register_actor(RegisterTaskActorRequest {
                    task_id: sib_id.clone(),
                    title: "Turso Fleet Sibling".to_owned(),
                    scope_mode: TaskActorScopeMode::Project,
                })
                .map_err(|e| format!("register reopened actor {sib_raw}: {e}"))?;
        }
        let _ = task_actors.close_read_only_peek(&sib_id);
    }
    let parent_id = TaskActorId::new("fleet-host-parent").map_err(|e| format!("{e}"))?;
    let kv_digest = KvPrefixDigest::new("sha256:scheme-host-kv").map_err(|e| format!("{e}"))?;
    let has_defeater = capsule.total_active_defeaters > 0;
    let requested_depth = parse_disclosure_depth(&step.disclosure_level);
    let effective_depth = if has_defeater {
        DisclosureDepth::L3
    } else {
        requested_depth
    };
    task_actors
        .bind_governed_context(GovernedContextCapsule {
            task_id: parent_id,
            context_cut: 1,
            kv_prefix_digest: kv_digest,
            disclosure_depth: effective_depth,
            has_active_defeater: has_defeater,
            quarantined_memory_count: 0,
            sandbox_admitted: true,
            cedar_handoff_valid: true,
            detour_mode: if has_defeater {
                DetourMode::BlockerActive
            } else {
                DetourMode::Mainline
            },
            auto_archive_enabled: !has_defeater,
        })
        .map_err(|e| format!("rebind ui action governed context: {e}"))?;
    Ok(())
}

fn sync_post_gate_fleet_to_control_plane(
    task_actors: &TaskActorControlPlane,
    gate_id: &str,
    capsule: &FleetAuditCapsuleProjection,
) -> Result<(), String> {
    let parent_id = TaskActorId::new("fleet-host-parent").map_err(|e| format!("{e}"))?;
    let kv_digest = KvPrefixDigest::new("sha256:scheme-host-kv").map_err(|e| format!("{e}"))?;
    let current_epoch = task_actors
        .snapshot()
        .actors
        .get(&parent_id)
        .map(|rec| rec.cursor.fencing_epoch)
        .unwrap_or_else(|| FencingEpoch::new(1).expect("epoch 1"));
    let req_id = DispatchRequestId::new(format!(
        "req:scheme-gate:{gate_id}:e{}",
        current_epoch.value()
    ))
    .map_err(|e| format!("{e}"))?;
    task_actors
        .admit_dispatch(MarlinDispatchEnvelope {
            request_id: req_id.clone(),
            task_id: parent_id.clone(),
            operation: GraphDispatchOperation::ExecuteFrontier,
            fencing_epoch: current_epoch,
            scope_mode: TaskActorScopeMode::Project,
            join_node_id: None,
            substrate_target: GraphSubstrateTarget::SqliteTurso,
        })
        .map_err(|e| format!("admit post-gate dispatch: {e}"))?;
    task_actors
        .reconcile_receipt(&MarlinExecutionReceipt {
            request_id: req_id,
            task_id: parent_id.clone(),
            fencing_epoch: current_epoch,
            status: ExecutionReceiptStatus::Ok,
        })
        .map_err(|e| format!("reconcile post-gate receipt: {e}"))?;
    let has_defeater = capsule.total_active_defeaters > 0;
    task_actors
        .bind_governed_context(GovernedContextCapsule {
            task_id: parent_id,
            context_cut: 1,
            kv_prefix_digest: kv_digest,
            disclosure_depth: DisclosureDepth::L3,
            has_active_defeater: has_defeater,
            quarantined_memory_count: 0,
            sandbox_admitted: true,
            cedar_handoff_valid: true,
            detour_mode: if has_defeater {
                DetourMode::BlockerActive
            } else {
                DetourMode::Mainline
            },
            auto_archive_enabled: !has_defeater,
        })
        .map_err(|e| format!("rebind post-gate governed context: {e}"))?;
    Ok(())
}

/// Runs the dynamic-VM `marlin-scheme-host` daemon loading `.o1` modules from `MARLIN_SCHEME_DIR`.
#[cfg(not(feature = "external-program"))]
pub fn run_dynamic_scheme_host() -> Result<(), Box<dyn std::error::Error>> {
    let fleet_checkpoint_key = env::var("MARLIN_SCHEME_FLEET_CHECKPOINT_KEY")
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
    let resolve_in = |mod_sym: &RootedSchemeObject<'_>, proc_name: &str| {
        let name_sym = runtime.intern_identifier(IdentifierKind::Symbol, proc_name)?;
        runtime.resolve_package_export(scheme_dir_str, mod_sym, &name_sym)
    };
    let live_oracle = GraphEngineRootedOracle::new(
        &runtime,
        resolve_in(&module_sym, "marlin-graph-engine-project-dispatch-vector")?,
        resolve_in(&module_sym, "marlin-graph-engine-reconcile-receipt-vector")?,
        resolve_in(
            &module_sym,
            "marlin-graph-engine-project-workbench-tabs-vector",
        )?,
        resolve_in(
            &module_sym,
            "marlin-graph-engine-project-governed-context-vector",
        )?,
    );

    let fleet_bootstrap = if fleet_checkpoint_key.is_some() {
        let ctrl_mod_sym = runtime.intern_identifier(
            IdentifierKind::Symbol,
            ":marlin-agent-core/graph-engine/fleet-controller",
        )?;
        Some(HostFleetBootstrap::Dynamic(FleetSchemeProcs {
            make_demo_fleet_proc: resolve_in(
                &module_sym,
                "marlin-graph-engine-make-demo-fleet-vector",
            )?,
            restore_fleet_proc: resolve_in(
                &module_sym,
                "marlin-graph-engine-fleet-restore-vector",
            )?,
            open_proc: resolve_in(
                &ctrl_mod_sym,
                "marlin-fleet-workbench-controller-open-vector",
            )?,
            dispatch_proc: resolve_in(
                &ctrl_mod_sym,
                "marlin-fleet-workbench-controller-dispatch-vector",
            )?,
            audit_proc: resolve_in(
                &ctrl_mod_sym,
                "marlin-fleet-workbench-controller-audit-capsule-vector",
            )?,
        }))
    } else {
        None
    };

    run_scheme_host_core(&runtime, live_oracle, fleet_bootstrap, fleet_checkpoint_key)
}

/// Runs the Linked-AOT `marlin-scheme-host-aot` daemon against statically linked C-ABI exports
/// (zero runtime `.o1` `dlopen` and zero external `scheme/.gerbil` tree required).
///
/// # Safety
/// All function pointers in `exports` must belong to `runtime`'s initialized linked program.
pub unsafe fn run_linked_scheme_host(
    runtime: GerbilRuntime,
    exports: LinkedSchemeHostExports,
) -> Result<(), Box<dyn std::error::Error>> {
    let fleet_checkpoint_key = env::var("MARLIN_SCHEME_FLEET_CHECKPOINT_KEY")
        .ok()
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty());
    let live_oracle = unsafe {
        GraphEngineLinkedBridge::new(
            &runtime,
            GraphEngineLinkedExports {
                dispatch_vector: exports.dispatch_vector,
                reconcile_receipt_vector: exports.reconcile_receipt_vector,
                workbench_tabs_vector: exports.workbench_tabs_vector,
                governed_context_vector: exports.governed_context_vector,
            },
        )
    };
    let fleet_bootstrap = fleet_checkpoint_key.as_ref().map(|_| {
        HostFleetBootstrap::Linked(
            LinkedFleetBootstrapExports {
                make_demo_fleet_vector: exports.make_demo_fleet_vector,
                fleet_restore_vector: exports.fleet_restore_vector,
                fleet: SchemeFleetLinkedExports {
                    open_vector: exports.fleet_open_vector,
                    dispatch_vector: exports.fleet_dispatch_vector,
                    audit_capsule_vector: exports.fleet_audit_capsule_vector,
                },
            },
            std::marker::PhantomData,
        )
    });
    run_scheme_host_core(&runtime, live_oracle, fleet_bootstrap, fleet_checkpoint_key)
}

fn run_scheme_host_core<'vm, O: marlin_agent_graph::GraphEngineOracle>(
    runtime: &'vm GerbilRuntime,
    live_oracle: O,
    mut fleet_procs: Option<HostFleetBootstrap<'vm>>,
    fleet_checkpoint_key: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
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
    let loopback_llm_response: Option<Arc<str>> = env::var("MARLIN_SCHEME_LLM_LOOPBACK_RESPONSE")
        .ok()
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .map(Arc::from);
    let http_llm_client = parse_openai_http_client_from_env()?;
    let sandbox_runner = parse_sandbox_runner_from_env();

    let (oracle_handle, oracle_server) = graph_engine_oracle_channel(64);
    let oracle_arc: Arc<dyn marlin_agent_graph::GraphEngineOracle + Send + Sync> =
        Arc::new(oracle_handle);
    let (fleet_tx, fleet_rx) = mpsc::sync_channel::<FleetVmCommand>(32);
    let (effect_tx, mut effect_rx) = tokio::sync::mpsc::unbounded_channel::<EffectBatchRequest>();
    let fleet_checkpoint_key_srv = fleet_checkpoint_key.clone();

    // 2. Spawn multi-threaded Tokio HTTP/2 AppServer + MyGoBridgeHost on a background OS thread.
    let server_thread = std::thread::spawn(move || -> Result<(), String> {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .map_err(|err| format!("build tokio runtime: {err}"))?;
        let result = rt.block_on(async move {
            let mut persistence_handle: Option<TaskActorPersistenceHandle> = None;
            let mut cas_store: Option<Arc<StorageCasStore<TursoAgentStorage>>> = None;
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
                let session = SessionId::new("session:marlin-scheme-host")
                    .map_err(|err| format!("session id: {err}"))?;
                cas_store = Some(Arc::new(StorageCasStore::new(
                    storage.clone(),
                    project.clone(),
                    session,
                    agent.clone(),
                )));
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

            // Spawn thin Tokio atomic-effect worker serving CasGet/CasPut/SandboxRun/LlmComplete for the Gambit VM thread.
            let cas_for_worker = cas_store.clone();
            let sandbox_for_worker = sandbox_runner.clone();
            let http_llm_for_worker = http_llm_client.clone();
            let llm_for_worker = loopback_llm_response.clone();
            let effect_worker = tokio::spawn(async move {
                while let Some(batch) = effect_rx.recv().await {
                    let mut handles = Vec::with_capacity(batch.effects.len());
                    for eff in batch.effects {
                        let cas = cas_for_worker.clone();
                        let sbx = sandbox_for_worker.clone();
                        let http_llm = http_llm_for_worker.clone();
                        let llm = llm_for_worker.clone();
                        handles.push(tokio::spawn(async move {
                            run_one_atomic_effect(cas, sbx, http_llm, llm, eff).await
                        }));
                    }
                    let mut receipts = Vec::with_capacity(handles.len());
                    for handle in handles {
                        match handle.await {
                            Ok(r) => receipts.push(r),
                            Err(err) => receipts.push(SchemeAtomicReceipt::EffectError {
                                effect_id: "effect-worker-join".to_owned(),
                                message: format!("atomic effect task panicked: {err}"),
                            }),
                        }
                    }
                    let _ = batch.reply.send(receipts);
                }
            });

            let mut initial_fleet_report: Option<FleetBootReport> = None;
            if let Some(cp_key) = fleet_checkpoint_key_srv.clone() {
                let (boot_tx, boot_rx) = mpsc::sync_channel(1);
                fleet_tx
                    .send(FleetVmCommand::Bootstrap {
                        checkpoint_key: cp_key,
                        reply: boot_tx,
                    })
                    .map_err(|_| "send fleet bootstrap to gambit vm thread".to_owned())?;
                let report = tokio::task::spawn_blocking(move || boot_rx.recv())
                    .await
                    .map_err(|err| format!("join fleet bootstrap wait: {err}"))?
                    .map_err(|_| "gambit vm thread dropped fleet bootstrap reply".to_owned())??;
                sync_boot_fleet_to_control_plane(&task_actors, &report.capsule)?;
                println!(
                    "FLEET_BOOT mode={} active={} archived={} defeaters={} inbox={}",
                    report.mode,
                    report.capsule.active_actor_count,
                    report.capsule.archived_actor_count,
                    report.capsule.total_active_defeaters,
                    report.capsule.total_human_inbox_items
                );
                io::stdout()
                    .flush()
                    .map_err(|err| format!("flush fleet boot line: {err}"))?;
                initial_fleet_report = Some(report);
            }

            let app_auth = expected_header.clone();
            let bridge_auth = expected_header;
            let host = MyGoBridgeHost::new_default_trust(move |meta| {
                MyGoBridgeHost::check_bearer_header(meta, bridge_auth.as_deref())
            })
            .with_task_actors(task_actors.clone());

            if let Some(report) = &initial_fleet_report {
                let boot_disc = if report.mode == "restored" {
                    "L3"
                } else {
                    "L1"
                };
                publish_fleet_audit_projection(
                    &host,
                    &format!("fleet_boot_{}", report.mode),
                    "scheme_fleet_boot",
                    boot_disc,
                    &report.capsule,
                );
            }

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

            let (watcher_stop_tx, _) = tokio::sync::broadcast::channel::<()>(4);

            let gate_watcher = if fleet_checkpoint_key_srv.is_some() {
                let mut gate_rx = host.subscribe_claimed_graph_decisions();
                let mut stop_rx = watcher_stop_tx.subscribe();
                let fleet_tx_gate = fleet_tx.clone();
                let actors_for_gate = task_actors.clone();
                let host_for_gate = host.clone();
                Some(tokio::spawn(async move {
                    loop {
                        let ev = tokio::select! {
                            _ = stop_rx.recv() => break,
                            res = gate_rx.recv() => match res {
                                Ok(ev) => ev,
                                Err(_) => break,
                            }
                        };
                        if ev.decision == GraphHumanDecision::Approve as i32 {
                            let (reply_tx, reply_rx) = mpsc::sync_channel(1);
                            let gate_id = ev.gate_id.clone();
                            if fleet_tx_gate
                                .send(FleetVmCommand::HumanGateApproved {
                                    gate_id: ev.gate_id,
                                    note: ev.note,
                                    reply: reply_tx,
                                })
                                .is_ok()
                                && let Ok(Ok(Ok(capsule))) =
                                    tokio::task::spawn_blocking(move || reply_rx.recv()).await
                            {
                                let _ = sync_post_gate_fleet_to_control_plane(
                                    &actors_for_gate,
                                    &gate_id,
                                    &capsule,
                                );
                                publish_fleet_audit_projection(
                                    &host_for_gate,
                                    &format!("fleet_human_gate:{gate_id}"),
                                    "scheme_human_gate_approved",
                                    "L3",
                                    &capsule,
                                );
                            }
                        } else if ev.decision == GraphHumanDecision::Reject as i32 {
                            let (reply_tx, reply_rx) = mpsc::sync_channel(1);
                            let gate_id = ev.gate_id.clone();
                            if fleet_tx_gate
                                .send(FleetVmCommand::HumanGateRejected {
                                    gate_id: ev.gate_id,
                                    reply: reply_tx,
                                })
                                .is_ok()
                                && let Ok(Ok(Ok(capsule))) =
                                    tokio::task::spawn_blocking(move || reply_rx.recv()).await
                            {
                                publish_fleet_audit_projection(
                                    &host_for_gate,
                                    &format!("fleet_human_gate_rejected:{gate_id}"),
                                    "scheme_human_gate_rejected",
                                    "L3",
                                    &capsule,
                                );
                            }
                        }
                    }
                }))
            } else {
                None
            };

            let ui_cmd_watcher = if fleet_checkpoint_key_srv.is_some() {
                let mut ui_cmd_rx = host.subscribe_dispatched_ui_commands();
                let mut stop_rx = watcher_stop_tx.subscribe();
                let fleet_tx_ui = fleet_tx.clone();
                let actors_for_ui = task_actors.clone();
                let host_for_ui = host.clone();
                Some(tokio::spawn(async move {
                    loop {
                        let ev = tokio::select! {
                            _ = stop_rx.recv() => break,
                            res = ui_cmd_rx.recv() => match res {
                                Ok(ev) => ev,
                                Err(_) => break,
                            }
                        };
                        if let Some((action_sym, arg)) = parse_scheme_ui_command(&ev.command) {
                            let (reply_tx, reply_rx) = mpsc::sync_channel(1);
                            let arg_clone = arg.clone();
                            if fleet_tx_ui
                                .send(FleetVmCommand::UiAction {
                                    action_sym: action_sym.to_owned(),
                                    arg,
                                    reply: reply_tx,
                                })
                                .is_ok()
                                && let Ok(Ok(Ok((step, capsule)))) =
                                    tokio::task::spawn_blocking(move || reply_rx.recv()).await
                            {
                                let _ = sync_ui_action_fleet_to_control_plane(
                                    &actors_for_ui,
                                    action_sym,
                                    arg_clone.as_deref(),
                                    &step,
                                    &capsule,
                                );
                                publish_fleet_audit_projection(
                                    &host_for_ui,
                                    &format!("fleet_ui_action:{action_sym}"),
                                    "scheme_ui_action",
                                    &step.disclosure_level,
                                    &capsule,
                                );
                            }
                        }
                    }
                }))
            } else {
                None
            };

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

            let _ = watcher_stop_tx.send(());
            if let Some(watcher) = gate_watcher {
                let _ = tokio::time::timeout(Duration::from_secs(2), watcher).await;
            }
            if let Some(watcher) = ui_cmd_watcher {
                let _ = tokio::time::timeout(Duration::from_secs(2), watcher).await;
            }
            drop(fleet_tx);

            if let Some(handle) = persistence_handle {
                handle
                    .flush_and_stop()
                    .await
                    .map_err(|err| format!("flush persistence: {err}"))?;
            }
            effect_worker.abort();
            let _ = effect_worker.await;
            Ok(())
        });
        rt.shutdown_timeout(Duration::from_secs(2));
        result
    });

    // 3. Multiplex live Scheme Oracle + Fleet Controller commands on the Gambit VM-owner thread.
    let mut active_fleet_driver: Option<HostFleetDriver<'_>> = None;
    let mut active_checkpoint_key = fleet_checkpoint_key;
    loop {
        while let Ok(cmd) = fleet_rx.try_recv() {
            match cmd {
                FleetVmCommand::Bootstrap {
                    checkpoint_key,
                    reply,
                } => {
                    let outcome = match fleet_procs.take() {
                        Some(procs) => {
                            bootstrap_scheme_fleet(runtime, procs, &effect_tx, &checkpoint_key).map(
                                |(driver, report)| {
                                    active_fleet_driver = Some(driver);
                                    active_checkpoint_key = Some(checkpoint_key);
                                    report
                                },
                            )
                        }
                        None => Err("fleet procs unavailable for bootstrap".to_owned()),
                    };
                    let _ = reply.send(outcome);
                }
                FleetVmCommand::HumanGateApproved {
                    gate_id,
                    note,
                    reply,
                } => {
                    let outcome = match (
                        active_fleet_driver.as_mut(),
                        active_checkpoint_key.as_deref(),
                    ) {
                        (Some(driver), Some(cp_key)) => {
                            (|| -> Result<FleetAuditCapsuleProjection, String> {
                                let _step = driver
                                    .dispatch(
                                        "task-human-approve",
                                        None,
                                        note.as_deref(),
                                        &[],
                                        true,
                                    )
                                    .map_err(|err| format!("dispatch task-human-approve: {err}"))?;
                                let cp_eff = driver
                                    .checkpoint(cp_key)
                                    .map_err(|err| format!("plan post-gate checkpoint: {err}"))?;
                                let stored = execute_effects_on_tokio(&effect_tx, vec![cp_eff])
                                    .map_err(|err| format!("store post-gate checkpoint: {err}"))?;
                                match stored.into_iter().next() {
                                    Some(SchemeAtomicReceipt::CasStored(_)) => {}
                                    other => {
                                        return Err(format!(
                                            "post-gate checkpoint store failed: {other:?}"
                                        ));
                                    }
                                }
                                let capsule = driver
                                    .audit_capsule()
                                    .map_err(|err| format!("audit post-gate fleet: {err}"))?;
                                assert_non_authorizing_capsule(&capsule)?;
                                println!(
                                    "FLEET_GATE_APPLIED gate={gate_id} active={} archived={} defeaters={} inbox={}",
                                    capsule.active_actor_count,
                                    capsule.archived_actor_count,
                                    capsule.total_active_defeaters,
                                    capsule.total_human_inbox_items
                                );
                                let _ = io::stdout().flush();
                                Ok(capsule)
                            })()
                        }
                        _ => Err("fleet driver not initialized for human gate".to_owned()),
                    };
                    let _ = reply.send(outcome);
                }
                FleetVmCommand::HumanGateRejected { gate_id, reply } => {
                    let outcome = match active_fleet_driver.as_mut() {
                        Some(driver) => (|| -> Result<FleetAuditCapsuleProjection, String> {
                            // Rejection never resolves defeaters or authorizes effects.
                            let capsule = driver
                                .audit_capsule()
                                .map_err(|err| format!("audit rejected-gate fleet: {err}"))?;
                            assert_non_authorizing_capsule(&capsule)?;
                            println!(
                                "FLEET_GATE_REJECTED gate={gate_id} active={} archived={} defeaters={} inbox={}",
                                capsule.active_actor_count,
                                capsule.archived_actor_count,
                                capsule.total_active_defeaters,
                                capsule.total_human_inbox_items
                            );
                            let _ = io::stdout().flush();
                            Ok(capsule)
                        })(),
                        None => Err("fleet driver not initialized for rejected gate".to_owned()),
                    };
                    let _ = reply.send(outcome);
                }
                FleetVmCommand::UiAction {
                    action_sym,
                    arg,
                    reply,
                } => {
                    let outcome = match (
                        active_fleet_driver.as_mut(),
                        active_checkpoint_key.as_deref(),
                    ) {
                        (Some(driver), Some(cp_key)) => (|| -> Result<(SchemeFleetDispatchStep, FleetAuditCapsuleProjection), String> {
                            let mut step = driver
                                .dispatch(&action_sym, None, arg.as_deref(), &[], true)
                                .map_err(|err| format!("dispatch ui action `{action_sym}`: {err}"))?;
                            if !step.effects.is_empty() {
                                let initial_rcpts =
                                    execute_effects_on_tokio(&effect_tx, step.effects)
                                        .map_err(|err| format!("execute ui action effects: {err}"))?;
                                let summary = driver
                                    .pump_until_quiescent(&initial_rcpts, true, 8, |effs| {
                                        execute_effects_on_tokio(&effect_tx, effs)
                                    })
                                    .map_err(|err| format!("pump ui action to quiescence: {err}"))?;
                                step = summary.final_step;
                            }
                            let cp_eff = driver
                                .checkpoint(cp_key)
                                .map_err(|err| format!("plan post-ui-action checkpoint: {err}"))?;
                            let stored = execute_effects_on_tokio(&effect_tx, vec![cp_eff])
                                .map_err(|err| format!("store post-ui-action checkpoint: {err}"))?;
                            match stored.into_iter().next() {
                                Some(SchemeAtomicReceipt::CasStored(_)) => {}
                                other => {
                                    return Err(format!(
                                        "post-ui-action checkpoint store failed: {other:?}"
                                    ));
                                }
                            }
                            let capsule = driver
                                .audit_capsule()
                                .map_err(|err| format!("audit post-ui-action fleet: {err}"))?;
                            assert_non_authorizing_capsule(&capsule)?;
                            println!(
                                "FLEET_UI_ACTION action={action_sym} disclosure={} active={} archived={} defeaters={} inbox={}",
                                step.disclosure_level,
                                capsule.active_actor_count,
                                capsule.archived_actor_count,
                                capsule.total_active_defeaters,
                                capsule.total_human_inbox_items
                            );
                            let _ = io::stdout().flush();
                            Ok((step, capsule))
                        })(),
                        _ => Err("fleet driver not initialized for ui action".to_owned()),
                    };
                    let _ = reply.send(outcome);
                }
            }
        }

        match oracle_server.serve_next_timeout(&live_oracle, Duration::from_millis(5)) {
            Ok(()) => {}
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    drop(active_fleet_driver);
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
