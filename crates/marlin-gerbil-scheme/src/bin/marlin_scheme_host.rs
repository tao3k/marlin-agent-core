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
    env,
    io::{self, Write},
    path::PathBuf,
    sync::{Arc, mpsc},
    time::Duration,
};

use gerbil_scheme::{
    CollectionKind, GerbilRuntime, IdentifierKind, PackageResolutionProgress, RootedSchemeObject,
};
use marlin_agent_app_server::{
    AllowlistSandboxRunner, AppServer, RegisterTaskActorRequest, StorageCasStore,
    TaskActorControlPlane, TaskActorPersistenceHandle,
    ui_grpc::MyGoBridgeHost,
    wire::marlin::gate::v1::{GateKind, GraphHumanDecision, PendingGate},
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
use marlin_gerbil_scheme::{
    AtomicEffectAbiError, FleetAuditCapsuleProjection, GraphEngineRootedOracle, SchemeAtomicEffect,
    SchemeAtomicReceipt, SchemeFleetSessionDriver, encode_scheme_atomic_receipt_vector,
    gerbil_package_root, graph_engine_oracle_channel,
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
}

struct EffectBatchRequest {
    effects: Vec<SchemeAtomicEffect>,
    reply: mpsc::SyncSender<Vec<SchemeAtomicReceipt>>,
}

struct FleetSchemeProcs<'vm> {
    make_demo_fleet_proc: RootedSchemeObject<'vm>,
    restore_fleet_proc: RootedSchemeObject<'vm>,
    open_proc: RootedSchemeObject<'vm>,
    dispatch_proc: RootedSchemeObject<'vm>,
    audit_proc: RootedSchemeObject<'vm>,
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
    procs: FleetSchemeProcs<'vm>,
    effect_tx: &tokio::sync::mpsc::UnboundedSender<EffectBatchRequest>,
    checkpoint_key: &str,
) -> Result<(SchemeFleetSessionDriver<'vm>, FleetBootReport), String> {
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
            let restored_fleet = runtime
                .call_rooted(&procs.restore_fleet_proc, &rcpt_vec)
                .map_err(|err| format!("restore scheme fleet from cas: {err}"))?;
            let driver = SchemeFleetSessionDriver::open(
                runtime,
                &procs.open_proc,
                procs.dispatch_proc,
                procs.audit_proc,
                restored_fleet,
                "L3",
            )
            .map_err(|err| format!("open restored fleet driver: {err}"))?;
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
            let fleet = runtime
                .call_rooted(&procs.make_demo_fleet_proc, &demo_in)
                .map_err(|err| format!("construct scheme demo fleet: {err}"))?;
            let mut driver = SchemeFleetSessionDriver::open(
                runtime,
                &procs.open_proc,
                procs.dispatch_proc,
                procs.audit_proc,
                fleet,
                "L1",
            )
            .map_err(|err| format!("open cold fleet driver: {err}"))?;
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

async fn run_one_atomic_effect(
    cas: Option<Arc<StorageCasStore<TursoAgentStorage>>>,
    sandbox: Option<Arc<AllowlistSandboxRunner>>,
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
                Err(message) => SchemeAtomicReceipt::EffectError { effect_id, message },
            },
            None => SchemeAtomicReceipt::EffectError {
                effect_id,
                message: "sandbox runner denied: no authorized sandbox policy mounted".to_owned(),
            },
        },
        SchemeAtomicEffect::LlmComplete { effect_id, .. } => match loopback_llm_response {
            Some(content) => SchemeAtomicReceipt::LlmCompleted {
                effect_id,
                content: content.as_ref().to_owned(),
            },
            None => SchemeAtomicReceipt::EffectError {
                effect_id,
                message: "llm gateway not configured on marlin-scheme-host".to_owned(),
            },
        },
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
    let already_present = task_actors.snapshot().actors.contains_key(&parent_id);
    if !already_present {
        task_actors
            .register_actor(RegisterTaskActorRequest {
                task_id: parent_id.clone(),
                title: "Turso Fleet Parent".to_owned(),
                scope_mode: TaskActorScopeMode::Project,
            })
            .map_err(|e| format!("register fleet parent actor: {e}"))?;
    }
    let _ = task_actors.open_read_only_peek(sib_id);
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
    let fleet_checkpoint_key = env::var("MARLIN_SCHEME_FLEET_CHECKPOINT_KEY")
        .ok()
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty());
    let loopback_llm_response: Option<Arc<str>> = env::var("MARLIN_SCHEME_LLM_LOOPBACK_RESPONSE")
        .ok()
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .map(Arc::from);
    let sandbox_runner = parse_sandbox_runner_from_env();
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

    let mut fleet_procs = if fleet_checkpoint_key.is_some() {
        let ctrl_mod_sym = runtime.intern_identifier(
            IdentifierKind::Symbol,
            ":marlin-agent-core/graph-engine/fleet-controller",
        )?;
        Some(FleetSchemeProcs {
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
        })
    } else {
        None
    };

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

            // Spawn thin Tokio atomic-effect worker serving CasGet/CasPut/SandboxRun for the Gambit VM thread.
            let cas_for_worker = cas_store.clone();
            let sandbox_for_worker = sandbox_runner.clone();
            let llm_for_worker = loopback_llm_response.clone();
            let effect_worker = tokio::spawn(async move {
                while let Some(batch) = effect_rx.recv().await {
                    let mut handles = Vec::with_capacity(batch.effects.len());
                    for eff in batch.effects {
                        let cas = cas_for_worker.clone();
                        let sbx = sandbox_for_worker.clone();
                        let llm = llm_for_worker.clone();
                        handles.push(tokio::spawn(async move {
                            run_one_atomic_effect(cas, sbx, llm, eff).await
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
            }

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

            let gate_watcher = if fleet_checkpoint_key_srv.is_some() {
                let mut gate_rx = host.subscribe_claimed_graph_decisions();
                let fleet_tx_gate = fleet_tx.clone();
                let actors_for_gate = task_actors.clone();
                Some(tokio::spawn(async move {
                    while let Ok(ev) = gate_rx.recv().await {
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

            if let Some(watcher) = gate_watcher {
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
    let mut active_fleet_driver: Option<SchemeFleetSessionDriver<'_>> = None;
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
                            bootstrap_scheme_fleet(&runtime, procs, &effect_tx, &checkpoint_key)
                                .map(|(driver, report)| {
                                    active_fleet_driver = Some(driver);
                                    active_checkpoint_key = Some(checkpoint_key);
                                    report
                                })
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
