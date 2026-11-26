// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Real embedded Gambit/Gerbil VM round-trip verification for all 4 `GraphEngine`
//! rooted-vector C-ABI schemas (dispatch, reconcile, workbench-tabs, governed-context).

use std::collections::BTreeMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use gerbil_scheme::{
    CollectionKind, GerbilRuntime, IdentifierBuffer, IdentifierKind, ObjectKind,
    PackageResolutionProgress, RootedSchemeObject, SchemeScalar,
};
use marlin_agent_app_server::{
    BoundedTokioSandboxRunner, CompositeAtomicEffectExecutor, RegisterTaskActorRequest,
    TaskActorControlPlane,
    ui_grpc::MyGoBridgeHost,
    wire::marlin::{
        common::v1::{DenialCode, TargetKind},
        gate::v1::{
            GateKind, GraphHumanDecision, PendingGate, SubmitGraphHumanDecisionRequest,
            approval_gate_service_client::ApprovalGateServiceClient,
        },
        ui::v1::{OpenWorkbenchRequest, ui_workbench_service_client::UiWorkbenchServiceClient},
    },
};
use marlin_agent_graph::{
    DetourMode, DisclosureDepth, DispatchRequestId, ExecutionReceiptStatus, FencingEpoch,
    GovernedContextCapsule, GovernedContextRejection, GraphConvergenceNodeId,
    GraphDispatchOperation, GraphDispatchRejection, GraphSubstrateTarget, KvPrefixDigest,
    MarlinDispatchEnvelope, MarlinExecutionReceipt, ReceiptReconciliationReason, TaskActorId,
    TaskActorScopeMode, TaskActorStatus, WorkbenchActiveTaskInput, WorkbenchTabKind,
};
use marlin_agent_protocol::{
    ModelGateway, ModelGatewayCompletionChoice, ModelGatewayCompletionResponse, ModelGatewayFuture,
    ModelGatewayRequest, ModelGatewayResult, assistant_gateway_message,
};
use marlin_agent_runtime::{
    AtomicEffect, AtomicEffectExecutor, AtomicEffectReceipt, CasGetEffectPayload,
    CasHitReceiptPayload, CasPutEffectPayload, EffectErrorReceiptPayload, LlmCompleteEffectPayload,
    LlmCompletedReceiptPayload, SandboxExitedReceiptPayload, SandboxRunEffectPayload,
    TaskActorRuntimeRegistry, run_atomic_effect_batch,
};
use marlin_gerbil_scheme::{
    GraphEngineDispatchProjection, GraphEngineGovernedContextProjection,
    GraphEngineProjectionError, GraphEngineRootedOracle, GraphEngineWorkbenchTaskRow,
    SchemeAtomicEffect, SchemeAtomicReceipt, decode_fleet_audit_capsule_vector,
    decode_graph_engine_dispatch_vector, decode_graph_engine_governed_context_vector,
    decode_graph_engine_reconcile_vector, decode_graph_engine_workbench_tabs_vector,
    decode_scheme_atomic_effect_vector, encode_graph_engine_dispatch_request,
    encode_graph_engine_governed_context_request, encode_graph_engine_reconcile_request,
    encode_graph_engine_workbench_tabs_request, encode_scheme_atomic_receipt_vector,
    gerbil_package_root, graph_engine_oracle_channel,
};

fn vec_obj<'vm>(
    runtime: &'vm GerbilRuntime,
    items: &[RootedSchemeObject<'vm>],
) -> RootedSchemeObject<'vm> {
    let refs: Vec<&RootedSchemeObject<'vm>> = items.iter().collect();
    runtime
        .collection_from_objects(CollectionKind::Vector, &refs)
        .unwrap()
        .into_object()
}

fn sym<'vm>(runtime: &'vm GerbilRuntime, name: &str) -> RootedSchemeObject<'vm> {
    runtime
        .intern_identifier(IdentifierKind::Symbol, name)
        .unwrap()
}

fn txt<'vm>(runtime: &'vm GerbilRuntime, text: &str) -> RootedSchemeObject<'vm> {
    runtime.string_from_utf8(text).unwrap().into_object()
}

fn fx<'vm>(runtime: &'vm GerbilRuntime, n: isize) -> RootedSchemeObject<'vm> {
    runtime.scalar_object(SchemeScalar::Fixnum(n)).unwrap()
}

fn flag<'vm>(runtime: &'vm GerbilRuntime, b: bool) -> RootedSchemeObject<'vm> {
    runtime.scalar_object(SchemeScalar::Boolean(b)).unwrap()
}

#[test]
fn graph_engine_vm_round_trips_all_four_rooted_vectors_and_live_scheme_exports() {
    let runtime = GerbilRuntime::initialize().expect("initialize embedded Gerbil VM");
    let _ = runtime.set_package_resolution_progress(PackageResolutionProgress::Quiet);

    // -------------------------------------------------------------------------
    // Phase 1: Always-on real Gambit heap encode/decode & fail-closed gate tests
    // -------------------------------------------------------------------------
    let dispatch_env = MarlinDispatchEnvelope {
        request_id: DispatchRequestId::new("req-vm-1").unwrap(),
        task_id: TaskActorId::new("task-vm-alpha").unwrap(),
        operation: GraphDispatchOperation::ApplySteering,
        fencing_epoch: FencingEpoch::new(2).unwrap(),
        scope_mode: TaskActorScopeMode::Project,
        join_node_id: Some(GraphConvergenceNodeId::new("join-F").unwrap()),
        substrate_target: GraphSubstrateTarget::SqliteTurso,
    };
    let encoded_dispatch = encode_graph_engine_dispatch_request(&runtime, &dispatch_env).unwrap();
    let encoded_dispatch_vec = encoded_dispatch
        .into_collection(CollectionKind::Vector)
        .unwrap();
    assert_eq!(encoded_dispatch_vec.len(), 7);

    // Simulated Scheme output vector claiming admitted=#t WITHOUT join-node-id on apply-steering
    // must be caught fail-closed by Rust's `admit_marlin_dispatch` cross-check!
    let forged_steer = vec_obj(
        &runtime,
        &[
            fx(&runtime, 1),
            flag(&runtime, true),
            txt(&runtime, "req-forged"),
            txt(&runtime, "task-vm-alpha"),
            sym(&runtime, "apply-steering"),
            fx(&runtime, 2),
            sym(&runtime, "project"),
            flag(&runtime, false),
            sym(&runtime, "pure-memory"),
            flag(&runtime, false),
        ],
    );
    assert_eq!(
        decode_graph_engine_dispatch_vector(forged_steer),
        Err(GraphEngineProjectionError::DispatchGate(
            GraphDispatchRejection::SteeringRequiresJoinNode
        ))
    );

    // Simulated Scheme governed-context vector claiming admitted=#t with active defeater at L2
    // must be caught fail-closed by Rust's `admit_governed_context_capsule` cross-check!
    let forged_ctx = vec_obj(
        &runtime,
        &[
            fx(&runtime, 1),
            flag(&runtime, true),
            txt(&runtime, "task-vm-alpha"),
            fx(&runtime, 3),
            txt(&runtime, "sha256:forged"),
            sym(&runtime, "L2"),
            flag(&runtime, true),
            fx(&runtime, 1),
            flag(&runtime, true),
            flag(&runtime, true),
            sym(&runtime, "mainline"),
            flag(&runtime, true),
            flag(&runtime, false),
        ],
    );
    assert_eq!(
        decode_graph_engine_governed_context_vector(forged_ctx),
        Err(GraphEngineProjectionError::GovernedContextGate(
            GovernedContextRejection::ActiveDefeaterRequiresL3Disclosure
        ))
    );

    // Simulated Workbench tabs vector with unpinned slot 0 must fail closed.
    let unpinned_tabs = vec_obj(
        &runtime,
        &[
            fx(&runtime, 1),
            txt(&runtime, "task-vm-alpha"),
            vec_obj(
                &runtime,
                &[vec_obj(
                    &runtime,
                    &[
                        sym(&runtime, "active-task"),
                        txt(&runtime, "task-vm-alpha"),
                        txt(&runtime, "Alpha"),
                        sym(&runtime, "running"),
                        flag(&runtime, true),
                        fx(&runtime, 1),
                    ],
                )],
            ),
        ],
    );
    assert_eq!(
        decode_graph_engine_workbench_tabs_vector(unpinned_tabs),
        Err(GraphEngineProjectionError::Invalid(
            "slot 0 must be pinned-index"
        ))
    );

    // -------------------------------------------------------------------------
    // Phase 2: Live compiled Scheme closure FFI round-trip when .ssi is built
    // -------------------------------------------------------------------------
    let scheme_root = gerbil_package_root();
    let funcs_ssi = scheme_root.join(".gerbil/lib/marlin-agent-core/graph-engine/funcs.ssi");
    if !funcs_ssi.is_file() {
        return;
    }

    let scheme_dir = scheme_root.canonicalize().unwrap();
    let scheme_dir_str = scheme_dir.to_str().unwrap();
    let module_sym = sym(&runtime, ":marlin-agent-core/graph-engine/funcs");

    let resolve = |proc_name: &str| {
        let name_sym = sym(&runtime, proc_name);
        runtime
            .resolve_package_export(scheme_dir_str, &module_sym, &name_sym)
            .unwrap_or_else(|e| panic!("resolve {proc_name}: {e}"))
    };

    let scheme_dispatch = resolve("marlin-graph-engine-project-dispatch-vector");
    let scheme_reconcile = resolve("marlin-graph-engine-reconcile-receipt-vector");
    let scheme_tabs = resolve("marlin-graph-engine-project-workbench-tabs-vector");
    let scheme_governed = resolve("marlin-graph-engine-project-governed-context-vector");

    // 1. Live Scheme dispatch projection -> Rust decode -> Rust TaskActorRuntimeRegistry
    let registry = TaskActorRuntimeRegistry::new();
    let task_id = TaskActorId::new("task-vm-live").unwrap();
    registry
        .register_actor(task_id.clone(), "Live VM Task", TaskActorScopeMode::Project)
        .unwrap();

    let live_dispatch_req = MarlinDispatchEnvelope {
        request_id: DispatchRequestId::new("req-live-steer").unwrap(),
        task_id: task_id.clone(),
        operation: GraphDispatchOperation::ApplySteering,
        fencing_epoch: FencingEpoch::new(1).unwrap(),
        scope_mode: TaskActorScopeMode::Project,
        join_node_id: Some(GraphConvergenceNodeId::new("join-vm").unwrap()),
        substrate_target: GraphSubstrateTarget::SqliteTurso,
    };
    let dispatch_in = encode_graph_engine_dispatch_request(&runtime, &live_dispatch_req).unwrap();
    let dispatch_out = runtime.call_rooted(&scheme_dispatch, &dispatch_in).unwrap();
    let GraphEngineDispatchProjection::Admitted(admitted_by_scheme) =
        decode_graph_engine_dispatch_vector(dispatch_out).unwrap()
    else {
        panic!("expected Scheme dispatch vector to admit valid ApplySteering envelope");
    };
    assert_eq!(admitted_by_scheme.envelope(), &live_dispatch_req);
    registry
        .admit_dispatch(admitted_by_scheme.into_envelope())
        .unwrap();

    // 2. Live Scheme receipt reconciliation matches Rust TaskActorRuntimeRegistry identically!
    let cursor_now = registry.cursor(&task_id).unwrap();
    let receipt = MarlinExecutionReceipt {
        request_id: DispatchRequestId::new("req-live-steer").unwrap(),
        task_id: task_id.clone(),
        fencing_epoch: FencingEpoch::new(1).unwrap(),
        status: ExecutionReceiptStatus::Ok,
    };
    let reconcile_in =
        encode_graph_engine_reconcile_request(&runtime, &cursor_now, &receipt).unwrap();
    let reconcile_out = runtime
        .call_rooted(&scheme_reconcile, &reconcile_in)
        .unwrap();
    let scheme_decision = decode_graph_engine_reconcile_vector(reconcile_out).unwrap();
    let rust_decision = registry.reconcile_receipt(&receipt).unwrap();
    assert_eq!(scheme_decision, rust_decision);
    assert!(scheme_decision.accepted);
    assert_eq!(scheme_decision.next_epoch, FencingEpoch::new(2).unwrap());
    assert_eq!(
        scheme_decision.next_status,
        TaskActorStatus::WaitingSteering
    );
    assert_eq!(
        scheme_decision.reason,
        ReceiptReconciliationReason::AwaitingSteeringConvergence
    );

    // 3. Live Scheme workbench-tabs projection pins slot 0 Index & deduplicates disjoint peeks
    let peek_id = TaskActorId::new("task-archived-vm").unwrap();
    let tabs_in = encode_graph_engine_workbench_tabs_request(
        &runtime,
        Some(&task_id),
        &[GraphEngineWorkbenchTaskRow {
            task: WorkbenchActiveTaskInput {
                task_id: task_id.clone(),
                title: "Live VM Task".to_owned(),
                status: TaskActorStatus::WaitingSteering,
                fencing_epoch: FencingEpoch::new(2).unwrap(),
            },
            scope_mode: TaskActorScopeMode::Project,
        }],
        &[task_id.clone(), peek_id.clone(), peek_id.clone()],
    )
    .unwrap();
    let tabs_out = runtime.call_rooted(&scheme_tabs, &tabs_in).unwrap();
    let decoded_tabs = decode_graph_engine_workbench_tabs_vector(tabs_out).unwrap();
    assert_eq!(decoded_tabs.tabs.len(), 3);
    assert_eq!(decoded_tabs.tabs[0].kind, WorkbenchTabKind::PinnedIndex);
    assert_eq!(decoded_tabs.tabs[1].kind, WorkbenchTabKind::ActiveTask);
    assert!(decoded_tabs.tabs[1].focused);
    assert_eq!(decoded_tabs.tabs[2].kind, WorkbenchTabKind::ReadOnlyPeek);
    assert_eq!(decoded_tabs.tabs[2].id, "task-archived-vm");

    // 4. Live Scheme governed-context projection -> Rust decode -> bind on TaskActorRuntimeRegistry
    let capsule = GovernedContextCapsule {
        task_id: task_id.clone(),
        context_cut: 2,
        kv_prefix_digest: KvPrefixDigest::new("sha256:live-vm-kv-prefix").unwrap(),
        disclosure_depth: DisclosureDepth::L3,
        has_active_defeater: true,
        quarantined_memory_count: 2,
        sandbox_admitted: true,
        cedar_handoff_valid: true,
        detour_mode: DetourMode::BlockerActive,
        auto_archive_enabled: false,
    };
    let gov_in = encode_graph_engine_governed_context_request(&runtime, &capsule).unwrap();
    let gov_out = runtime.call_rooted(&scheme_governed, &gov_in).unwrap();
    let GraphEngineGovernedContextProjection::Admitted(admitted_ctx) =
        decode_graph_engine_governed_context_vector(gov_out).unwrap()
    else {
        panic!("expected Scheme governed-context vector to admit valid L3 blocker capsule");
    };
    assert_eq!(admitted_ctx.capsule(), &capsule);
    registry
        .bind_governed_context(admitted_ctx.into_capsule())
        .unwrap();

    // Negative check through live Scheme: blocker-active with auto_archive=true rejected cleanly
    let mut bad_capsule = capsule;
    bad_capsule.auto_archive_enabled = true;
    let bad_gov_in = encode_graph_engine_governed_context_request(&runtime, &bad_capsule).unwrap();
    let bad_gov_out = runtime.call_rooted(&scheme_governed, &bad_gov_in).unwrap();
    assert_eq!(
        decode_graph_engine_governed_context_vector(bad_gov_out).unwrap(),
        GraphEngineGovernedContextProjection::Rejected {
            reason: "blocker-detour-must-disable-auto-archive".to_owned(),
        }
    );

    // -------------------------------------------------------------------------
    // Phase 3 (Round S3): Live Embedded Gerbil Scheme VM <-> Thin Rust Tokio
    // `marlin_agent_runtime::run_atomic_effect_batch` 4-Primitive Round-Trip!
    // -------------------------------------------------------------------------
    let scheme_make_demo = resolve("marlin-graph-engine-make-demo-driver-state-vector");
    let scheme_step_driver = resolve("marlin-graph-engine-step-driver-vector");

    struct VmLeafSubstrate {
        cas: Mutex<BTreeMap<String, Vec<u8>>>,
    }

    impl AtomicEffectExecutor for VmLeafSubstrate {
        fn execute_atomic<'a>(
            &'a self,
            effect: AtomicEffect,
        ) -> Pin<Box<dyn Future<Output = AtomicEffectReceipt> + Send + 'a>> {
            Box::pin(async move {
                match effect {
                    AtomicEffect::CasGet(p) => {
                        if let Some(bytes) = self.cas.lock().unwrap().get(&p.key).cloned() {
                            AtomicEffectReceipt::CasHit(CasHitReceiptPayload {
                                effect_id: p.effect_id,
                                bytes,
                            })
                        } else {
                            AtomicEffectReceipt::CasMiss(p.effect_id)
                        }
                    }
                    AtomicEffect::CasPut(p) => {
                        self.cas.lock().unwrap().insert(p.key, p.bytes);
                        AtomicEffectReceipt::CasStored(p.effect_id)
                    }
                    AtomicEffect::LlmComplete(p) => {
                        let tail = p
                            .request
                            .messages()
                            .last()
                            .map(|m| m.content.clone())
                            .unwrap_or_default();
                        AtomicEffectReceipt::LlmCompleted(LlmCompletedReceiptPayload {
                            effect_id: p.effect_id,
                            content: format!("vm-llm-out:{tail}"),
                        })
                    }
                    AtomicEffect::SandboxRun(p) => {
                        AtomicEffectReceipt::SandboxExited(SandboxExitedReceiptPayload {
                            effect_id: p.effect_id,
                            exit_code: 0,
                            stdout: format!("ran:{}", p.program).into_bytes(),
                            stderr: Vec::new(),
                        })
                    }
                }
            })
        }
    }

    fn read_sym(obj: RootedSchemeObject<'_>) -> String {
        assert_eq!(obj.as_ref().kind().unwrap(), ObjectKind::Symbol);
        let mut buf = IdentifierBuffer::default();
        obj.as_ref().copy_identifier_names(&mut buf).unwrap();
        let (IdentifierKind::Symbol, name) = buf.get(0).unwrap() else {
            panic!("expected symbol");
        };
        name.to_owned()
    }

    fn read_str(obj: RootedSchemeObject<'_>) -> String {
        obj.into_string()
            .unwrap()
            .to_string()
            .into_result()
            .unwrap()
    }

    fn read_fx(obj: RootedSchemeObject<'_>) -> isize {
        match obj.as_ref().scalar().unwrap() {
            SchemeScalar::Fixnum(n) => n,
            other => panic!("expected fixnum, got {other:?}"),
        }
    }

    fn decode_effect<'vm>(obj: RootedSchemeObject<'vm>) -> AtomicEffect {
        match decode_scheme_atomic_effect_vector(obj).expect("decode Scheme atomic effect vector") {
            SchemeAtomicEffect::CasGet { effect_id, key } => {
                AtomicEffect::CasGet(CasGetEffectPayload { effect_id, key })
            }
            SchemeAtomicEffect::CasPut {
                effect_id,
                key,
                bytes,
            } => AtomicEffect::CasPut(CasPutEffectPayload {
                effect_id,
                key,
                bytes,
            }),
            SchemeAtomicEffect::LlmComplete { effect_id, request } => {
                AtomicEffect::LlmComplete(LlmCompleteEffectPayload { effect_id, request })
            }
            SchemeAtomicEffect::SandboxRun {
                effect_id,
                program,
                args,
                cwd,
            } => AtomicEffect::SandboxRun(SandboxRunEffectPayload {
                effect_id,
                program,
                args,
                cwd,
            }),
        }
    }

    fn encode_receipt<'vm>(
        rt: &'vm GerbilRuntime,
        receipt: &AtomicEffectReceipt,
    ) -> RootedSchemeObject<'vm> {
        let wire = match receipt {
            AtomicEffectReceipt::CasHit(p) => SchemeAtomicReceipt::CasHit {
                effect_id: p.effect_id.clone(),
                bytes: p.bytes.clone(),
            },
            AtomicEffectReceipt::CasMiss(id) => SchemeAtomicReceipt::CasMiss(id.clone()),
            AtomicEffectReceipt::CasStored(id) => SchemeAtomicReceipt::CasStored(id.clone()),
            AtomicEffectReceipt::LlmCompleted(p) => SchemeAtomicReceipt::LlmCompleted {
                effect_id: p.effect_id.clone(),
                content: p.content.clone(),
            },
            AtomicEffectReceipt::SandboxExited(p) => SchemeAtomicReceipt::SandboxExited {
                effect_id: p.effect_id.clone(),
                exit_code: p.exit_code,
                stdout: p.stdout.clone(),
                stderr: p.stderr.clone(),
            },
            AtomicEffectReceipt::EffectError(EffectErrorReceiptPayload { effect_id, message }) => {
                SchemeAtomicReceipt::EffectError {
                    effect_id: effect_id.clone(),
                    message: message.clone(),
                }
            }
        };
        encode_scheme_atomic_receipt_vector(rt, &wire).expect("encode Scheme receipt vector")
    }

    let rt_tokio = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let substrate = Arc::new(VmLeafSubstrate {
        cas: Mutex::new(BTreeMap::new()),
    });

    #[allow(clippy::too_many_arguments)]
    fn pump_vm_rounds<'vm>(
        runtime: &'vm GerbilRuntime,
        step_proc: &RootedSchemeObject<'vm>,
        rt_tokio: &tokio::runtime::Runtime,
        substrate: &Arc<VmLeafSubstrate>,
        state: RootedSchemeObject<'vm>,
        incoming_receipts: Vec<AtomicEffectReceipt>,
        remaining: usize,
        total_llm: isize,
        total_cas_hits: isize,
        total_cas_puts: isize,
    ) -> (String, isize, isize, isize) {
        let rcpt_objs: Vec<RootedSchemeObject<'vm>> = incoming_receipts
            .iter()
            .map(|r| encode_receipt(runtime, r))
            .collect();
        let step_in = vec_obj(runtime, &[state, vec_obj(runtime, &rcpt_objs)]);
        let step_out = runtime
            .call_rooted(step_proc, &step_in)
            .unwrap()
            .into_collection(CollectionKind::Vector)
            .unwrap();

        let last_phase = read_sym(step_out.element(1).unwrap());
        let next_llm = total_llm + read_fx(step_out.element(2).unwrap());
        let next_hits = total_cas_hits + read_fx(step_out.element(3).unwrap());
        let next_puts = total_cas_puts + read_fx(step_out.element(4).unwrap());

        let effs_vec = step_out
            .element(5)
            .unwrap()
            .into_collection(CollectionKind::Vector)
            .unwrap();
        if effs_vec.is_empty() || remaining <= 1 {
            return (last_phase, next_llm, next_hits, next_puts);
        }
        let mut effects = Vec::with_capacity(effs_vec.len());
        for i in 0..effs_vec.len() {
            effects.push(decode_effect(effs_vec.element(i).unwrap()));
        }
        let next_receipts =
            rt_tokio.block_on(run_atomic_effect_batch(Arc::clone(substrate), effects));
        let next_state = step_out.element(0).unwrap();
        pump_vm_rounds(
            runtime,
            step_proc,
            rt_tokio,
            substrate,
            next_state,
            next_receipts,
            remaining - 1,
            next_llm,
            next_hits,
            next_puts,
        )
    }

    let run_vm_to_quiescence = |substrate: Arc<VmLeafSubstrate>| -> (String, isize, isize, isize) {
        let init_req = vec_obj(
            &runtime,
            &[
                txt(&runtime, "task-vm-pump-01"),
                txt(&runtime, "VM Pump Actor"),
                txt(&runtime, "sha256:vm-pump-zone1"),
            ],
        );
        let init_state = runtime.call_rooted(&scheme_make_demo, &init_req).unwrap();
        pump_vm_rounds(
            &runtime,
            &scheme_step_driver,
            &rt_tokio,
            &substrate,
            init_state,
            Vec::new(),
            8,
            0,
            0,
            0,
        )
    };

    // Cold run: cas-get -> cas-miss -> Two-Zone llm-complete -> cas-put -> 2x2 competing hypotheses -> quiescent!
    let (cold_phase, cold_llm, cold_hits, cold_puts) = run_vm_to_quiescence(Arc::clone(&substrate));
    assert_eq!(cold_phase, "quiescent-fixed-point");
    assert_eq!(cold_hits, 0);
    assert_eq!(cold_puts, 1);
    assert_eq!(cold_llm, 5); // 1 deterministic cold-miss fill + 4 competing hypothesis evaluations

    // Warm run against same Rust CAS map: cas-get -> cas-hit (0 LLM for deterministic wave!) + 4 hypothesis evals -> quiescent!
    let (warm_phase, warm_llm, warm_hits, warm_puts) = run_vm_to_quiescence(Arc::clone(&substrate));
    assert_eq!(warm_phase, "quiescent-fixed-point");
    assert_eq!(warm_hits, 1);
    assert_eq!(warm_puts, 0);
    assert_eq!(warm_llm, 4);

    // -------------------------------------------------------------------------
    // Phase 4 (Round S4): Live Embedded Gerbil VM Multi-Actor Fleet <-> REAL
    // `TursoAgentStorage` SQLite WAL on Disk + Full Crash/Reopen Recovery!
    // -------------------------------------------------------------------------
    let scheme_make_fleet = resolve("marlin-graph-engine-make-demo-fleet-vector");
    let scheme_step_fleet = resolve("marlin-graph-engine-step-fleet-vector");
    let scheme_fleet_checkpoint = resolve("marlin-graph-engine-fleet-checkpoint-vector");
    let scheme_fleet_restore = resolve("marlin-graph-engine-fleet-restore-vector");
    let scheme_fleet_summary = resolve("marlin-graph-engine-fleet-summary-vector");

    struct VmEchoModelGateway;

    impl ModelGateway for VmEchoModelGateway {
        fn complete(
            &self,
            request: ModelGatewayRequest,
        ) -> ModelGatewayFuture<ModelGatewayResult<ModelGatewayCompletionResponse>> {
            let tail = request
                .messages()
                .last()
                .map(|m| m.content.clone())
                .unwrap_or_default();
            Box::pin(async move {
                Ok(ModelGatewayCompletionResponse::new(
                    "turso-vm-resp",
                    "gpt-marlin-turso",
                    vec![ModelGatewayCompletionChoice::new(
                        0,
                        assistant_gateway_message(format!("turso-llm:{tail}")),
                        None,
                    )],
                ))
            })
        }
    }

    type ProdTursoExecutor = CompositeAtomicEffectExecutor<
        marlin_agent_storage::TursoAgentStorage,
        VmEchoModelGateway,
        BoundedTokioSandboxRunner,
    >;

    fn make_prod_turso_executor(
        storage: marlin_agent_storage::TursoAgentStorage,
    ) -> Arc<ProdTursoExecutor> {
        Arc::new(
            CompositeAtomicEffectExecutor::new(
                storage,
                VmEchoModelGateway,
                marlin_agent_storage::ProjectId::new("proj-vm-turso").unwrap(),
                marlin_agent_storage::SessionId::new("vm-turso-pump").unwrap(),
                marlin_agent_storage::AgentId::new("gerbil-vm-agent").unwrap(),
            )
            .with_sandbox(BoundedTokioSandboxRunner::new(
                Duration::from_secs(5),
                16 * 1024,
            )),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn pump_fleet_rounds<'vm>(
        runtime: &'vm GerbilRuntime,
        step_proc: &RootedSchemeObject<'vm>,
        cp_proc: &RootedSchemeObject<'vm>,
        rt_tokio: &tokio::runtime::Runtime,
        substrate: &Arc<ProdTursoExecutor>,
        fleet: RootedSchemeObject<'vm>,
        incoming: Vec<AtomicEffectReceipt>,
        remaining: usize,
        cp_key: &str,
    ) -> (Vec<String>, Vec<String>, AtomicEffect) {
        let rcpt_objs: Vec<RootedSchemeObject<'vm>> = incoming
            .iter()
            .map(|r| encode_receipt(runtime, r))
            .collect();
        let step_in = vec_obj(
            runtime,
            &[fleet, vec_obj(runtime, &rcpt_objs), flag(runtime, true)],
        );
        let step_out = runtime
            .call_rooted(step_proc, &step_in)
            .unwrap()
            .into_collection(CollectionKind::Vector)
            .unwrap();
        let active_vec = step_out
            .element(1)
            .unwrap()
            .into_collection(CollectionKind::Vector)
            .unwrap();
        let settled_vec = step_out
            .element(2)
            .unwrap()
            .into_collection(CollectionKind::Vector)
            .unwrap();
        let effs_vec = step_out
            .element(6)
            .unwrap()
            .into_collection(CollectionKind::Vector)
            .unwrap();
        let next_fleet = step_out.element(0).unwrap();
        if effs_vec.is_empty() || remaining <= 1 {
            let active_ids: Vec<String> = (0..active_vec.len())
                .map(|i| read_str(active_vec.element(i).unwrap()))
                .collect();
            let settled_ids: Vec<String> = (0..settled_vec.len())
                .map(|i| read_str(settled_vec.element(i).unwrap()))
                .collect();
            let cp_in = vec_obj(runtime, &[next_fleet, txt(runtime, cp_key)]);
            let cp_eff_obj = runtime.call_rooted(cp_proc, &cp_in).unwrap();
            return (active_ids, settled_ids, decode_effect(cp_eff_obj));
        }
        let mut effects = Vec::with_capacity(effs_vec.len());
        for i in 0..effs_vec.len() {
            effects.push(decode_effect(effs_vec.element(i).unwrap()));
        }
        let next_receipts =
            rt_tokio.block_on(run_atomic_effect_batch(Arc::clone(substrate), effects));
        pump_fleet_rounds(
            runtime,
            step_proc,
            cp_proc,
            rt_tokio,
            substrate,
            next_fleet,
            next_receipts,
            remaining - 1,
            cp_key,
        )
    }

    let tmp_dir = tempfile::tempdir().expect("tempdir for turso vm");
    let turso_cfg = marlin_agent_storage::TursoAgentStorageConfig {
        path: tmp_dir.path().join("gerbil-vm-fleet.db"),
        optimization_profile: marlin_agent_storage::TursoOptimizationProfile::AsyncIoWithMvcc,
        batch_transaction_mode: marlin_agent_storage::TursoBatchTransactionMode::Concurrent,
    };

    // Boot #1: Open real Turso SQLite WAL on disk, run 2-actor Fleet inside live Gerbil VM,
    // persist full Fleet UTF-8 wire checkpoint into Turso WAL, then DROP storage!
    let storage1 = rt_tokio
        .block_on(marlin_agent_storage::TursoAgentStorage::open_local(
            turso_cfg.clone(),
        ))
        .expect("open turso boot #1");
    let turso_sub1 = make_prod_turso_executor(storage1);
    let fleet_req = vec_obj(
        &runtime,
        &[
            txt(&runtime, "task-turso-parent"),
            txt(&runtime, "task-turso-sib"),
            txt(&runtime, "sha256:turso-vm-zone1"),
        ],
    );
    let init_fleet = runtime.call_rooted(&scheme_make_fleet, &fleet_req).unwrap();
    let (active_ids, settled_ids, cp_effect) = pump_fleet_rounds(
        &runtime,
        &scheme_step_fleet,
        &scheme_fleet_checkpoint,
        &rt_tokio,
        &turso_sub1,
        init_fleet,
        Vec::new(),
        8,
        "fleet:turso:boot1",
    );
    assert_eq!(active_ids, vec!["task-turso-parent"]);
    assert_eq!(settled_ids, vec!["task-turso-sib"]);

    // Persist Fleet UTF-8 wire checkpoint to REAL Turso SQLite WAL on disk!
    let put_receipts = rt_tokio.block_on(run_atomic_effect_batch(
        Arc::clone(&turso_sub1),
        vec![cp_effect],
    ));
    assert!(matches!(
        put_receipts.as_slice(),
        [AtomicEffectReceipt::CasStored(_)]
    ));
    drop(turso_sub1);

    // Boot #2: Reopen REAL Turso SQLite WAL from disk, read raw UTF-8 wire bytes via CasGet,
    // and rehydrate the live GraphFleetState inside Gerbil VM!
    let storage2 = rt_tokio
        .block_on(marlin_agent_storage::TursoAgentStorage::open_local(
            turso_cfg,
        ))
        .expect("reopen turso boot #2");
    let turso_sub2 = make_prod_turso_executor(storage2);
    let get_receipts = rt_tokio.block_on(run_atomic_effect_batch(
        Arc::clone(&turso_sub2),
        vec![AtomicEffect::CasGet(CasGetEffectPayload {
            effect_id: "restore-fleet-boot2".to_owned(),
            key: "fleet:turso:boot1".to_owned(),
        })],
    ));
    let cas_hit_rcpt = get_receipts.into_iter().next().unwrap();
    assert!(matches!(cas_hit_rcpt, AtomicEffectReceipt::CasHit(_)));

    let encoded_hit = encode_receipt(&runtime, &cas_hit_rcpt);
    let restored_fleet = runtime
        .call_rooted(&scheme_fleet_restore, &encoded_hit)
        .unwrap();
    let summary = runtime
        .call_rooted(&scheme_fleet_summary, &restored_fleet)
        .unwrap()
        .into_collection(CollectionKind::Vector)
        .unwrap();
    let r_active = summary
        .element(0)
        .unwrap()
        .into_collection(CollectionKind::Vector)
        .unwrap();
    let r_settled = summary
        .element(1)
        .unwrap()
        .into_collection(CollectionKind::Vector)
        .unwrap();
    let r_detours = summary
        .element(2)
        .unwrap()
        .into_collection(CollectionKind::Vector)
        .unwrap();
    let r_defeaters = summary
        .element(3)
        .unwrap()
        .into_collection(CollectionKind::Vector)
        .unwrap();
    let r_inboxes = summary
        .element(4)
        .unwrap()
        .into_collection(CollectionKind::Vector)
        .unwrap();
    let r_vis = summary
        .element(5)
        .unwrap()
        .into_collection(CollectionKind::Vector)
        .unwrap();

    assert_eq!(read_str(r_active.element(0).unwrap()), "task-turso-parent");
    assert_eq!(read_str(r_settled.element(0).unwrap()), "task-turso-sib");
    assert_eq!(read_sym(r_detours.element(0).unwrap()), "blocker-active");
    assert_eq!(read_fx(r_defeaters.element(0).unwrap()), 1);
    assert_eq!(read_fx(r_inboxes.element(0).unwrap()), 1);
    assert_eq!(r_vis.len(), 3); // index + active task-turso-parent + read-only peek task-turso-sib!
    assert_eq!(read_str(r_vis.element(0).unwrap()), "index");
    assert_eq!(read_str(r_vis.element(1).unwrap()), "task-turso-parent");
    assert_eq!(read_str(r_vis.element(2).unwrap()), "task-turso-sib");

    // -------------------------------------------------------------------------
    // Phase 5 (Round S7): Live Embedded Gerbil VM Interactive Fleet <-> TUI
    // Controller Keystroke Loop + REAL Turso SQLite WAL Boot #2 -> Drop -> Boot #3!
    // -------------------------------------------------------------------------
    let ctrl_mod_sym = sym(&runtime, ":marlin-agent-core/graph-engine/fleet-controller");
    let resolve_ctrl = |proc_name: &str| {
        let name_sym = sym(&runtime, proc_name);
        runtime
            .resolve_package_export(scheme_dir_str, &ctrl_mod_sym, &name_sym)
            .unwrap_or_else(|e| panic!("resolve {proc_name}: {e}"))
    };
    let scheme_ctrl_open = resolve_ctrl("marlin-fleet-workbench-controller-open-vector");
    let scheme_ctrl_dispatch = resolve_ctrl("marlin-fleet-workbench-controller-dispatch-vector");

    let turso_cfg3 = marlin_agent_storage::TursoAgentStorageConfig {
        path: tmp_dir.path().join("gerbil-vm-fleet.db"),
        optimization_profile: marlin_agent_storage::TursoOptimizationProfile::AsyncIoWithMvcc,
        batch_transaction_mode: marlin_agent_storage::TursoBatchTransactionMode::Concurrent,
    };

    // Open live FleetWorkbenchController directly over Boot #2's restored Fleet!
    let ctrl_open_in = vec_obj(&runtime, &[restored_fleet, sym(&runtime, "L1")]);
    let ctrl0 = runtime
        .call_rooted(&scheme_ctrl_open, &ctrl_open_in)
        .unwrap();

    // 1. User triggers `SPC o h a` (`task-human-approve`) -> clears defeater & exits blocker-active!
    let approve_in = vec_obj(
        &runtime,
        &[
            ctrl0,
            sym(&runtime, "task-human-approve"),
            flag(&runtime, false),
            flag(&runtime, false),
            vec_obj(&runtime, &[]),
            flag(&runtime, false),
        ],
    );
    let approve_out = runtime
        .call_rooted(&scheme_ctrl_dispatch, &approve_in)
        .unwrap()
        .into_collection(CollectionKind::Vector)
        .unwrap();
    let a1_detours = approve_out
        .element(4)
        .unwrap()
        .into_collection(CollectionKind::Vector)
        .unwrap();
    let a1_defeaters = approve_out
        .element(5)
        .unwrap()
        .into_collection(CollectionKind::Vector)
        .unwrap();
    let a1_ui = approve_out
        .element(6)
        .unwrap()
        .into_collection(CollectionKind::Vector)
        .unwrap();
    assert_eq!(read_sym(a1_detours.element(0).unwrap()), "mainline");
    assert_eq!(read_fx(a1_defeaters.element(0).unwrap()), 0);
    assert_eq!(a1_ui.len(), 9);
    let ctrl1 = approve_out.element(0).unwrap();

    // 2. User triggers `SPC o d s` (side-quest) -> `SPC o d n` (spawn sibling "task-turso-sib2")!
    let sq_in = vec_obj(
        &runtime,
        &[
            ctrl1,
            sym(&runtime, "task-detour-sidequest"),
            flag(&runtime, false),
            txt(&runtime, "sq-tui-boot2"),
            vec_obj(&runtime, &[]),
            flag(&runtime, false),
        ],
    );
    let sq_out = runtime
        .call_rooted(&scheme_ctrl_dispatch, &sq_in)
        .unwrap()
        .into_collection(CollectionKind::Vector)
        .unwrap();
    let sq_detours = sq_out
        .element(4)
        .unwrap()
        .into_collection(CollectionKind::Vector)
        .unwrap();
    assert_eq!(
        read_sym(sq_detours.element(0).unwrap()),
        "side-quest-active"
    );
    let ctrl2 = sq_out.element(0).unwrap();

    let sib_in = vec_obj(
        &runtime,
        &[
            ctrl2,
            sym(&runtime, "task-detour-exit-sibling"),
            flag(&runtime, false),
            txt(&runtime, "task-turso-sib2"),
            vec_obj(&runtime, &[]),
            flag(&runtime, false),
        ],
    );
    let sib_out = runtime
        .call_rooted(&scheme_ctrl_dispatch, &sib_in)
        .unwrap()
        .into_collection(CollectionKind::Vector)
        .unwrap();
    let sib_vis = sib_out
        .element(3)
        .unwrap()
        .into_collection(CollectionKind::Vector)
        .unwrap();
    let sib_detours = sib_out
        .element(4)
        .unwrap()
        .into_collection(CollectionKind::Vector)
        .unwrap();
    assert_eq!(read_sym(sib_detours.element(0).unwrap()), "mainline");
    assert_eq!(sib_vis.len(), 4); // index + task-turso-parent + task-turso-sib2 + peek task-turso-sib!
    assert_eq!(read_str(sib_vis.element(0).unwrap()), "index");
    assert_eq!(read_str(sib_vis.element(1).unwrap()), "task-turso-parent");
    assert_eq!(read_str(sib_vis.element(2).unwrap()), "task-turso-sib2");
    assert_eq!(read_str(sib_vis.element(3).unwrap()), "task-turso-sib");
    let ctrl3 = sib_out.element(0).unwrap();

    // 3. User triggers `SPC o 3` (`task-drill-l3`) -> disclosure elevates to L3 & projects Ratatui vector
    let drill_in = vec_obj(
        &runtime,
        &[
            ctrl3,
            sym(&runtime, "task-drill-l3"),
            flag(&runtime, false),
            flag(&runtime, false),
            vec_obj(&runtime, &[]),
            flag(&runtime, false),
        ],
    );
    let drill_out = runtime
        .call_rooted(&scheme_ctrl_dispatch, &drill_in)
        .unwrap()
        .into_collection(CollectionKind::Vector)
        .unwrap();
    assert_eq!(read_sym(drill_out.element(1).unwrap()), "L3");
    assert_eq!(
        drill_out
            .element(6)
            .unwrap()
            .into_collection(CollectionKind::Vector)
            .unwrap()
            .len(),
        9
    );
    let ctrl4 = drill_out.element(0).unwrap();

    // 4. User triggers `SPC o f c` (`task-fleet-checkpoint`) -> emits UTF-8 CasPut -> persist to Turso WAL!
    let cp_tui_in = vec_obj(
        &runtime,
        &[
            ctrl4,
            sym(&runtime, "task-fleet-checkpoint"),
            flag(&runtime, false),
            txt(&runtime, "fleet:turso:boot2-after-tui"),
            vec_obj(&runtime, &[]),
            flag(&runtime, false),
        ],
    );
    let cp_tui_out = runtime
        .call_rooted(&scheme_ctrl_dispatch, &cp_tui_in)
        .unwrap()
        .into_collection(CollectionKind::Vector)
        .unwrap();
    let cp_tui_effs = cp_tui_out
        .element(2)
        .unwrap()
        .into_collection(CollectionKind::Vector)
        .unwrap();
    assert_eq!(cp_tui_effs.len(), 1);
    let cp_tui_effect = decode_effect(cp_tui_effs.element(0).unwrap());
    let put_tui_receipts = rt_tokio.block_on(run_atomic_effect_batch(
        Arc::clone(&turso_sub2),
        vec![cp_tui_effect],
    ));
    assert!(matches!(
        put_tui_receipts.as_slice(),
        [AtomicEffectReceipt::CasStored(_)]
    ));
    drop(turso_sub2);

    // 5. Boot #3: Reopen REAL disk Turso SQLite WAL, restore post-TUI Fleet, open fresh controller!
    let storage3 = rt_tokio
        .block_on(marlin_agent_storage::TursoAgentStorage::open_local(
            turso_cfg3,
        ))
        .expect("reopen turso boot #3");
    let turso_sub3 = make_prod_turso_executor(storage3);
    let get3_receipts = rt_tokio.block_on(run_atomic_effect_batch(
        Arc::clone(&turso_sub3),
        vec![AtomicEffect::CasGet(CasGetEffectPayload {
            effect_id: "restore-fleet-boot3".to_owned(),
            key: "fleet:turso:boot2-after-tui".to_owned(),
        })],
    ));
    let cas_hit3 = get3_receipts.into_iter().next().unwrap();
    assert!(matches!(cas_hit3, AtomicEffectReceipt::CasHit(_)));
    let encoded_hit3 = encode_receipt(&runtime, &cas_hit3);
    let restored_fleet3 = runtime
        .call_rooted(&scheme_fleet_restore, &encoded_hit3)
        .unwrap();
    let ctrl_b3_in = vec_obj(&runtime, &[restored_fleet3, sym(&runtime, "L3")]);
    let ctrl_b3 = runtime.call_rooted(&scheme_ctrl_open, &ctrl_b3_in).unwrap();
    let verify_b3_in = vec_obj(
        &runtime,
        &[
            ctrl_b3,
            sym(&runtime, "task-drill-l3"),
            flag(&runtime, false),
            flag(&runtime, false),
            vec_obj(&runtime, &[]),
            flag(&runtime, false),
        ],
    );
    let b3_out = runtime
        .call_rooted(&scheme_ctrl_dispatch, &verify_b3_in)
        .unwrap()
        .into_collection(CollectionKind::Vector)
        .unwrap();
    let b3_vis = b3_out
        .element(3)
        .unwrap()
        .into_collection(CollectionKind::Vector)
        .unwrap();
    let b3_detours = b3_out
        .element(4)
        .unwrap()
        .into_collection(CollectionKind::Vector)
        .unwrap();
    let b3_defeaters = b3_out
        .element(5)
        .unwrap()
        .into_collection(CollectionKind::Vector)
        .unwrap();
    let b3_ui = b3_out
        .element(6)
        .unwrap()
        .into_collection(CollectionKind::Vector)
        .unwrap();

    assert_eq!(b3_vis.len(), 4);
    assert_eq!(read_str(b3_vis.element(0).unwrap()), "index");
    assert_eq!(read_str(b3_vis.element(1).unwrap()), "task-turso-parent");
    assert_eq!(read_str(b3_vis.element(2).unwrap()), "task-turso-sib2");
    assert_eq!(read_str(b3_vis.element(3).unwrap()), "task-turso-sib");
    assert_eq!(read_sym(b3_detours.element(0).unwrap()), "mainline");
    assert_eq!(read_sym(b3_detours.element(1).unwrap()), "mainline");
    assert_eq!(read_fx(b3_defeaters.element(0).unwrap()), 0);
    assert_eq!(read_fx(b3_defeaters.element(1).unwrap()), 0);
    assert_eq!(b3_ui.len(), 9);

    // -------------------------------------------------------------------------
    // Phase 6 (Round S10): Boot #3 Durable Reopen (`SPC o r`) + Bounded Branch
    // Proposal (`SPC o h b`) + FleetAuditCapsule Vector + Turso WAL Boot #4!
    // -------------------------------------------------------------------------
    let scheme_ctrl_audit = resolve_ctrl("marlin-fleet-workbench-controller-audit-capsule-vector");
    let ctrl_b3_live = b3_out.element(0).unwrap();

    // 1. User triggers `SPC o r` (`task-reopen` "task-turso-sib") -> durably reopens Boot #1's
    //    archived sibling back into active-drivers (active 2 -> 3, archived 1 -> 0)!
    let reopen_in = vec_obj(
        &runtime,
        &[
            ctrl_b3_live,
            sym(&runtime, "task-reopen"),
            flag(&runtime, false),
            txt(&runtime, "task-turso-sib"),
            vec_obj(&runtime, &[]),
            flag(&runtime, false),
        ],
    );
    let reopen_out = runtime
        .call_rooted(&scheme_ctrl_dispatch, &reopen_in)
        .unwrap()
        .into_collection(CollectionKind::Vector)
        .unwrap();
    let ctrl_b3_reopened = reopen_out.element(0).unwrap();

    // 2. User triggers `SPC o h b` (`task-propose-branch`) on focused actor!
    let branch_in = vec_obj(
        &runtime,
        &[
            ctrl_b3_reopened,
            sym(&runtime, "task-propose-branch"),
            txt(&runtime, "task-turso-parent"),
            txt(&runtime, "B-vm:Verify offline WAL signature"),
            vec_obj(&runtime, &[]),
            flag(&runtime, false),
        ],
    );
    let branch_out = runtime
        .call_rooted(&scheme_ctrl_dispatch, &branch_in)
        .unwrap()
        .into_collection(CollectionKind::Vector)
        .unwrap();
    let ctrl_b3_branched = branch_out.element(0).unwrap();

    // 3. Export FleetAuditCapsule vector via production codec & verify all security invariants!
    let audit3 = decode_fleet_audit_capsule_vector(
        runtime
            .call_rooted(&scheme_ctrl_audit, &ctrl_b3_branched)
            .unwrap(),
    )
    .expect("decode audit3");
    assert_eq!(audit3.active_actor_count, 3);
    assert_eq!(audit3.archived_actor_count, 0);
    assert_eq!(audit3.peek_count, 0);
    assert!(!audit3.favored_path_collapsed);
    assert!(!audit3.authenticates_evidence);
    assert!(!audit3.authorizes_effect);

    // 4. Checkpoint post-reopen Fleet via `SPC o f c` -> REAL disk Turso WAL -> Drop -> Boot #4!
    let cp3_in = vec_obj(
        &runtime,
        &[
            ctrl_b3_branched,
            sym(&runtime, "task-fleet-checkpoint"),
            flag(&runtime, false),
            txt(&runtime, "fleet:turso:boot3-after-reopen"),
            vec_obj(&runtime, &[]),
            flag(&runtime, false),
        ],
    );
    let cp3_out = runtime
        .call_rooted(&scheme_ctrl_dispatch, &cp3_in)
        .unwrap()
        .into_collection(CollectionKind::Vector)
        .unwrap();
    let cp3_effs = cp3_out
        .element(2)
        .unwrap()
        .into_collection(CollectionKind::Vector)
        .unwrap();
    assert_eq!(cp3_effs.len(), 1);
    let put3_rcpts = rt_tokio.block_on(run_atomic_effect_batch(
        Arc::clone(&turso_sub3),
        vec![decode_effect(cp3_effs.element(0).unwrap())],
    ));
    assert!(matches!(
        put3_rcpts.as_slice(),
        [AtomicEffectReceipt::CasStored(_)]
    ));
    drop(turso_sub3);

    // Boot #4: Reopen REAL disk Turso SQLite WAL via production CompositeAtomicEffectExecutor!
    let turso_cfg4 = marlin_agent_storage::TursoAgentStorageConfig {
        path: tmp_dir.path().join("gerbil-vm-fleet.db"),
        optimization_profile: marlin_agent_storage::TursoOptimizationProfile::AsyncIoWithMvcc,
        batch_transaction_mode: marlin_agent_storage::TursoBatchTransactionMode::Concurrent,
    };
    let storage4 = rt_tokio
        .block_on(marlin_agent_storage::TursoAgentStorage::open_local(
            turso_cfg4,
        ))
        .expect("reopen turso boot #4");
    let turso_sub4 = make_prod_turso_executor(storage4);
    let get4_rcpts = rt_tokio.block_on(run_atomic_effect_batch(
        Arc::clone(&turso_sub4),
        vec![AtomicEffect::CasGet(CasGetEffectPayload {
            effect_id: "restore-fleet-boot4".to_owned(),
            key: "fleet:turso:boot3-after-reopen".to_owned(),
        })],
    ));
    let hit4 = get4_rcpts.into_iter().next().unwrap();
    assert!(matches!(hit4, AtomicEffectReceipt::CasHit(_)));
    let restored_fleet4 = runtime
        .call_rooted(&scheme_fleet_restore, &encode_receipt(&runtime, &hit4))
        .unwrap();
    let ctrl_b4 = runtime
        .call_rooted(
            &scheme_ctrl_open,
            &vec_obj(&runtime, &[restored_fleet4, sym(&runtime, "L3")]),
        )
        .unwrap();
    let audit4 = decode_fleet_audit_capsule_vector(
        runtime.call_rooted(&scheme_ctrl_audit, &ctrl_b4).unwrap(),
    )
    .expect("decode audit4");
    assert_eq!(audit4.active_actor_count, 3);
    assert_eq!(audit4.archived_actor_count, 0);
    assert!(!audit4.favored_path_collapsed);
    assert!(!audit4.authenticates_evidence);
    assert!(!audit4.authorizes_effect);

    // -------------------------------------------------------------------------
    // Phase 7 (Round S15): Full Closed Loop — Embedded Gerbil Scheme Fleet
    // Controller <-> C-ABI atomic_effect_abi <-> Live HTTP/2 MyGoBridgeHost
    // (UiWorkbenchService + one-shot ApprovalGateService + run_atomic_effects)
    // <-> Real Disk Turso WAL Boot #4 -> Boot #5 Cold Restart!
    // -------------------------------------------------------------------------
    let unmounted_bridge = MyGoBridgeHost::new(
        marlin_agent_runtime::plugin::ReleaseTrust::new(BTreeMap::new()),
        |_meta| Ok(()),
    );
    let unmounted_err =
        rt_tokio.block_on(
            unmounted_bridge.run_atomic_effects(vec![AtomicEffect::CasGet(CasGetEffectPayload {
                effect_id: "unmounted-check".to_owned(),
                key: "fleet:turso:boot3-after-reopen".to_owned(),
            })]),
        );
    assert!(
        unmounted_err.is_err(),
        "unmounted MyGoBridgeHost must fail closed on run_atomic_effects"
    );

    let bridge4 = MyGoBridgeHost::new(
        marlin_agent_runtime::plugin::ReleaseTrust::new(BTreeMap::new()),
        |_meta| Ok(()),
    )
    .with_atomic_executor(turso_sub4.clone() as Arc<dyn AtomicEffectExecutor>);
    bridge4.enqueue_pending_gate(PendingGate {
        gate_id: "gate-vm-s15".to_owned(),
        kind: GateKind::GraphHumanGate as i32,
        turn: None,
        call_id: None,
        tool_or_node_name: "task-turso-parent".to_owned(),
        prompt: "Approve Scheme Fleet blocker detour release?".to_owned(),
        details_heading: "Scheme Fleet Human Gate".to_owned(),
        choices: Vec::new(),
        card: None,
        created_at_unix_ms: 15,
        expires_at_unix_ms: None,
    });

    // 1. Live HTTP/2 gRPC round-trip: locked host.gate + one-shot GraphHumanGate claim + replay denial.
    let (minted_receipt_id, server_handle4) = rt_tokio.block_on(async {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let srv_bridge = bridge4.clone();
        let handle = tokio::spawn(async move {
            let _ = srv_bridge.serve_standalone_on(listener).await;
        });
        let endpoint = format!("http://{addr}");

        let mut ui_client = UiWorkbenchServiceClient::connect(endpoint.clone())
            .await
            .unwrap();
        let wb = ui_client
            .open_workbench(OpenWorkbenchRequest {
                session_id: "s15-vm-live".to_owned(),
                client_target_kind: TargetKind::DesktopMacos as i32,
                preferred_profile: None,
            })
            .await
            .unwrap()
            .into_inner()
            .workbench
            .unwrap();
        assert!(wb.slots.iter().any(|s| {
            s.slot_id == "approval.gate"
                && s.fallback_renderer == "host.gate"
                && !s.allow_untrusted_plugin
        }));

        let mut gate_client = ApprovalGateServiceClient::connect(endpoint).await.unwrap();
        let first_claim = gate_client
            .submit_graph_human_decision(SubmitGraphHumanDecisionRequest {
                turn: None,
                gate_id: "gate-vm-s15".to_owned(),
                decision: GraphHumanDecision::Approve as i32,
                idempotency_key: "idem-s15-1".to_owned(),
                note: Some("verified live gRPC approval for Scheme VM".to_owned()),
            })
            .await
            .unwrap()
            .into_inner()
            .receipt
            .unwrap();
        assert!(first_claim.claimed);
        assert_eq!(first_claim.kind, GateKind::GraphHumanGate as i32);
        assert!(first_claim.denial.is_none());
        let receipt_id = first_claim.receipt.unwrap().receipt_id;
        assert!(!receipt_id.is_empty());

        let replay = gate_client
            .submit_graph_human_decision(SubmitGraphHumanDecisionRequest {
                turn: None,
                gate_id: "gate-vm-s15".to_owned(),
                decision: GraphHumanDecision::Approve as i32,
                idempotency_key: "idem-s15-2".to_owned(),
                note: None,
            })
            .await
            .unwrap()
            .into_inner()
            .receipt
            .unwrap();
        assert!(!replay.claimed);
        assert_eq!(replay.denial.unwrap().code, DenialCode::StaleOrdinal as i32);

        (receipt_id, handle)
    });
    assert!(!minted_receipt_id.is_empty());

    // 2. Drive the REAL embedded Gerbil Scheme FleetWorkbenchController:
    //    blocker detour -> gRPC-authorized human-approve -> return-to-origin -> branch proposal -> checkpoint!
    let blk_out = runtime
        .call_rooted(
            &scheme_ctrl_dispatch,
            &vec_obj(
                &runtime,
                &[
                    ctrl_b4,
                    sym(&runtime, "task-detour-blocker"),
                    txt(&runtime, "task-turso-parent"),
                    txt(&runtime, "blk-s15-gate"),
                    vec_obj(&runtime, &[]),
                    flag(&runtime, false),
                ],
            ),
        )
        .unwrap()
        .into_collection(CollectionKind::Vector)
        .unwrap();
    let blk_detours = blk_out
        .element(4)
        .unwrap()
        .into_collection(CollectionKind::Vector)
        .unwrap();
    assert_eq!(read_sym(blk_detours.element(0).unwrap()), "blocker-active");

    let appr_s15_out = runtime
        .call_rooted(
            &scheme_ctrl_dispatch,
            &vec_obj(
                &runtime,
                &[
                    blk_out.element(0).unwrap(),
                    sym(&runtime, "task-human-approve"),
                    txt(&runtime, "task-turso-parent"),
                    txt(&runtime, &minted_receipt_id),
                    vec_obj(&runtime, &[]),
                    flag(&runtime, false),
                ],
            ),
        )
        .unwrap()
        .into_collection(CollectionKind::Vector)
        .unwrap();

    let ret_s15_out = runtime
        .call_rooted(
            &scheme_ctrl_dispatch,
            &vec_obj(
                &runtime,
                &[
                    appr_s15_out.element(0).unwrap(),
                    sym(&runtime, "task-detour-exit-return"),
                    txt(&runtime, "task-turso-parent"),
                    flag(&runtime, false),
                    vec_obj(&runtime, &[]),
                    flag(&runtime, false),
                ],
            ),
        )
        .unwrap()
        .into_collection(CollectionKind::Vector)
        .unwrap();
    let ret_detours = ret_s15_out
        .element(4)
        .unwrap()
        .into_collection(CollectionKind::Vector)
        .unwrap();
    assert_eq!(read_sym(ret_detours.element(0).unwrap()), "mainline");

    let br_s15_out = runtime
        .call_rooted(
            &scheme_ctrl_dispatch,
            &vec_obj(
                &runtime,
                &[
                    ret_s15_out.element(0).unwrap(),
                    sym(&runtime, "task-propose-branch"),
                    txt(&runtime, "task-turso-parent"),
                    txt(&runtime, "B-s15:Verify live HTTP/2 gate receipt lineage"),
                    vec_obj(&runtime, &[]),
                    flag(&runtime, false),
                ],
            ),
        )
        .unwrap()
        .into_collection(CollectionKind::Vector)
        .unwrap();

    let cp4_out = runtime
        .call_rooted(
            &scheme_ctrl_dispatch,
            &vec_obj(
                &runtime,
                &[
                    br_s15_out.element(0).unwrap(),
                    sym(&runtime, "task-fleet-checkpoint"),
                    flag(&runtime, false),
                    txt(&runtime, "fleet:turso:boot4-after-grpc-gate"),
                    vec_obj(&runtime, &[]),
                    flag(&runtime, false),
                ],
            ),
        )
        .unwrap()
        .into_collection(CollectionKind::Vector)
        .unwrap();
    let cp4_effs = cp4_out
        .element(2)
        .unwrap()
        .into_collection(CollectionKind::Vector)
        .unwrap();
    assert_eq!(cp4_effs.len(), 1);

    // 3. Dispatch Scheme-emitted CasPut through the EXACT SAME live MyGoBridgeHost::run_atomic_effects!
    let put4_rcpts = rt_tokio
        .block_on(bridge4.run_atomic_effects(vec![decode_effect(cp4_effs.element(0).unwrap())]))
        .expect("live MyGoBridgeHost runs Scheme CasPut");
    assert!(matches!(
        put4_rcpts.as_slice(),
        [AtomicEffectReceipt::CasStored(_)]
    ));

    server_handle4.abort();
    drop(bridge4);
    drop(turso_sub4);

    // 4. Boot #5 Cold Restart: Reopen disk Turso WAL -> fresh MyGoBridgeHost -> CasGet -> Scheme restore!
    let turso_cfg5 = marlin_agent_storage::TursoAgentStorageConfig {
        path: tmp_dir.path().join("gerbil-vm-fleet.db"),
        optimization_profile: marlin_agent_storage::TursoOptimizationProfile::AsyncIoWithMvcc,
        batch_transaction_mode: marlin_agent_storage::TursoBatchTransactionMode::Concurrent,
    };
    let storage5 = rt_tokio
        .block_on(marlin_agent_storage::TursoAgentStorage::open_local(
            turso_cfg5,
        ))
        .expect("reopen turso boot #5");
    let bridge5 = MyGoBridgeHost::new(
        marlin_agent_runtime::plugin::ReleaseTrust::new(BTreeMap::new()),
        |_meta| Ok(()),
    )
    .with_atomic_executor(make_prod_turso_executor(storage5));

    let get5_rcpts = rt_tokio
        .block_on(
            bridge5.run_atomic_effects(vec![AtomicEffect::CasGet(CasGetEffectPayload {
                effect_id: "restore-fleet-boot5".to_owned(),
                key: "fleet:turso:boot4-after-grpc-gate".to_owned(),
            })]),
        )
        .expect("boot #5 host CasGet");
    let hit5 = get5_rcpts.into_iter().next().unwrap();
    assert!(matches!(hit5, AtomicEffectReceipt::CasHit(_)));

    let restored_fleet5 = runtime
        .call_rooted(&scheme_fleet_restore, &encode_receipt(&runtime, &hit5))
        .unwrap();
    let ctrl_b5 = runtime
        .call_rooted(
            &scheme_ctrl_open,
            &vec_obj(&runtime, &[restored_fleet5, sym(&runtime, "L3")]),
        )
        .unwrap();
    let audit5 = decode_fleet_audit_capsule_vector(
        runtime.call_rooted(&scheme_ctrl_audit, &ctrl_b5).unwrap(),
    )
    .expect("decode audit5");
    assert_eq!(audit5.active_actor_count, 3);
    assert_eq!(audit5.archived_actor_count, 0);
    assert_eq!(audit5.peek_count, 0);
    assert!(!audit5.favored_path_collapsed);
    assert!(!audit5.authenticates_evidence);
    assert!(!audit5.authorizes_effect);

    // -------------------------------------------------------------------------
    // Phase 8 (Round S18): Live Embedded Gerbil Scheme `GraphEngineRootedOracle`
    // Fail-Closed Cross-Check on `TaskActorControlPlane` + Real Disk Turso WAL
    // Boot #5 -> Boot #6 Cold Restart!
    // -------------------------------------------------------------------------
    let live_oracle = GraphEngineRootedOracle::new(
        &runtime,
        scheme_dispatch,
        scheme_reconcile,
        scheme_tabs,
        scheme_governed,
    );
    let oracle_project = marlin_agent_storage::ProjectId::new("proj-oracle-s18").unwrap();
    let oracle_pointer =
        marlin_agent_storage::ArtifactPointerKey::new("ptr:s18:oracle-control-plane").unwrap();
    let oracle_agent = marlin_agent_storage::AgentId::new("agent-oracle-s18").unwrap();
    let oracle_task_id = TaskActorId::new("task-oracle-s18").unwrap();

    let turso_cfg_s18 = marlin_agent_storage::TursoAgentStorageConfig {
        path: tmp_dir.path().join("gerbil-vm-oracle-s18.db"),
        optimization_profile: marlin_agent_storage::TursoOptimizationProfile::AsyncIoWithMvcc,
        batch_transaction_mode: marlin_agent_storage::TursoBatchTransactionMode::Concurrent,
    };
    let storage_s18_1 = rt_tokio
        .block_on(marlin_agent_storage::TursoAgentStorage::open_local(
            turso_cfg_s18.clone(),
        ))
        .expect("open turso oracle boot #5");
    let plane_s18_1 = rt_tokio
        .block_on(TaskActorControlPlane::rehydrate_or_new(
            &storage_s18_1,
            &oracle_project,
            &oracle_pointer,
        ))
        .expect("rehydrate empty s18 plane");
    let persist_s18_1 = rt_tokio.block_on(async {
        plane_s18_1.spawn_storage_persistence_loop(
            storage_s18_1.clone(),
            oracle_project.clone(),
            oracle_pointer.clone(),
            oracle_agent.clone(),
        )
    });
    plane_s18_1
        .register_actor(RegisterTaskActorRequest {
            task_id: oracle_task_id.clone(),
            title: "S18 Live Scheme Oracle Actor".to_owned(),
            scope_mode: TaskActorScopeMode::Project,
        })
        .expect("register s18 actor");

    // 1. Live Scheme Oracle admits valid ApplySteering & rejects steering without join_node_id.
    let valid_s18_steer = MarlinDispatchEnvelope {
        request_id: DispatchRequestId::new("req-s18-steer").unwrap(),
        task_id: oracle_task_id.clone(),
        operation: GraphDispatchOperation::ApplySteering,
        fencing_epoch: FencingEpoch::new(1).unwrap(),
        scope_mode: TaskActorScopeMode::Project,
        join_node_id: Some(GraphConvergenceNodeId::new("join-s18").unwrap()),
        substrate_target: GraphSubstrateTarget::SqliteTurso,
    };
    plane_s18_1
        .registry()
        .admit_dispatch_with_oracle(&live_oracle, valid_s18_steer)
        .expect("live Scheme oracle admits valid steering");

    let bad_s18_steer = MarlinDispatchEnvelope {
        request_id: DispatchRequestId::new("req-s18-bad-steer").unwrap(),
        task_id: oracle_task_id.clone(),
        operation: GraphDispatchOperation::ApplySteering,
        fencing_epoch: FencingEpoch::new(1).unwrap(),
        scope_mode: TaskActorScopeMode::Project,
        join_node_id: None,
        substrate_target: GraphSubstrateTarget::SqliteTurso,
    };
    assert!(matches!(
        plane_s18_1
            .registry()
            .admit_dispatch_with_oracle(&live_oracle, bad_s18_steer),
        Err(
            marlin_agent_runtime::TaskActorRuntimeError::DispatchRejected(
                GraphDispatchRejection::SteeringRequiresJoinNode
            )
        )
    ));

    // 2. Live Scheme Oracle reconciles execution receipt fail-closed (epoch 1 -> 2, WaitingSteering).
    let s18_receipt = MarlinExecutionReceipt {
        request_id: DispatchRequestId::new("req-s18-steer").unwrap(),
        task_id: oracle_task_id.clone(),
        fencing_epoch: FencingEpoch::new(1).unwrap(),
        status: ExecutionReceiptStatus::Ok,
    };
    let s18_decision = plane_s18_1
        .registry()
        .reconcile_receipt_with_oracle(&live_oracle, &s18_receipt)
        .expect("live Scheme oracle reconciles receipt");
    assert!(s18_decision.accepted);
    assert_eq!(s18_decision.next_epoch, FencingEpoch::new(2).unwrap());
    assert_eq!(s18_decision.next_status, TaskActorStatus::WaitingSteering);
    assert_eq!(
        s18_decision.reason,
        ReceiptReconciliationReason::AwaitingSteeringConvergence
    );

    // 3. Live Scheme Oracle admits valid L3 blocker governed context & rejects auto_archive=true.
    let valid_s18_ctx = GovernedContextCapsule {
        task_id: oracle_task_id.clone(),
        context_cut: 3,
        kv_prefix_digest: KvPrefixDigest::new("sha256:s18-oracle-kv").unwrap(),
        disclosure_depth: DisclosureDepth::L3,
        has_active_defeater: true,
        quarantined_memory_count: 1,
        sandbox_admitted: true,
        cedar_handoff_valid: true,
        detour_mode: DetourMode::BlockerActive,
        auto_archive_enabled: false,
    };
    plane_s18_1
        .registry()
        .bind_governed_context_with_oracle(&live_oracle, valid_s18_ctx.clone())
        .expect("live Scheme oracle admits valid L3 blocker context");

    let mut invalid_s18_ctx = valid_s18_ctx;
    invalid_s18_ctx.auto_archive_enabled = true;
    assert!(matches!(
        plane_s18_1
            .registry()
            .bind_governed_context_with_oracle(&live_oracle, invalid_s18_ctx),
        Err(
            marlin_agent_runtime::TaskActorRuntimeError::GovernedContextRejected(
                GovernedContextRejection::BlockerDetourMustDisableAutoArchive
            )
        )
    ));

    // 4. Live Scheme Oracle cross-checks pinned-Index Workbench tabs & survives Turso Boot #6!
    let tabs_s18_1 = plane_s18_1
        .registry()
        .project_workbench_tabs_with_oracle(&live_oracle)
        .expect("live Scheme oracle projects workbench tabs");
    assert_eq!(tabs_s18_1.tabs.len(), 2);
    assert_eq!(tabs_s18_1.tabs[0].kind, WorkbenchTabKind::PinnedIndex);
    assert_eq!(tabs_s18_1.tabs[1].id, "task-oracle-s18");
    assert_eq!(tabs_s18_1.tabs[1].fencing_epoch, 2);

    rt_tokio
        .block_on(persist_s18_1.flush_and_stop())
        .expect("flush s18 plane to turso WAL");
    drop(plane_s18_1);
    drop(storage_s18_1);

    // Boot #6 Cold Restart: Reopen disk Turso WAL -> rehydrate -> re-verify against live Scheme Oracle!
    let storage_s18_2 = rt_tokio
        .block_on(marlin_agent_storage::TursoAgentStorage::open_local(
            turso_cfg_s18,
        ))
        .expect("open turso oracle boot #6");
    let plane_s18_2 = rt_tokio
        .block_on(TaskActorControlPlane::rehydrate_or_new(
            &storage_s18_2,
            &oracle_project,
            &oracle_pointer,
        ))
        .expect("rehydrate s18 plane boot #6");
    let tabs_s18_2 = plane_s18_2
        .registry()
        .project_workbench_tabs_with_oracle(&live_oracle)
        .expect("boot #6 live Scheme oracle cross-check");
    assert_eq!(tabs_s18_2, tabs_s18_1);

    // -------------------------------------------------------------------------
    // Phase 9 (Round S19): Multi-Threaded Tokio Host Thread <-> Send+Sync
    // `GraphEngineOracleHandle` <-> VM-Owner Thread `GraphEngineRootedOracle`
    // + `TaskActorControlPlane::with_oracle` + `MyGoBridgeHost` + Turso WAL Boot #7!
    // -------------------------------------------------------------------------
    let (oracle_handle, oracle_server) = graph_engine_oracle_channel(32);
    let s19_db_path = tmp_dir.path().join("gerbil-vm-mt-oracle-s19.db");

    let mt_host_thread = std::thread::spawn(move || {
        let mt_rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("multi-thread tokio runtime");
        mt_rt.block_on(async move {
            let cfg = marlin_agent_storage::TursoAgentStorageConfig {
                path: s19_db_path,
                optimization_profile:
                    marlin_agent_storage::TursoOptimizationProfile::AsyncIoWithMvcc,
                batch_transaction_mode: marlin_agent_storage::TursoBatchTransactionMode::Concurrent,
            };
            let proj = marlin_agent_storage::ProjectId::new("proj-mt-s19").unwrap();
            let ptr = marlin_agent_storage::ArtifactPointerKey::new("ptr:s19:mt-oracle").unwrap();
            let agent = marlin_agent_storage::AgentId::new("agent-mt-s19").unwrap();
            let tid = TaskActorId::new("task-s19-mt").unwrap();
            let oracle_arc: Arc<dyn marlin_agent_graph::GraphEngineOracle + Send + Sync> =
                Arc::new(oracle_handle);

            // Boot #6b on separate OS thread with `TaskActorControlPlane::with_oracle`
            let storage1 = marlin_agent_storage::TursoAgentStorage::open_local(cfg.clone())
                .await
                .expect("open s19 turso #1");
            let plane1 = TaskActorControlPlane::rehydrate_or_new(&storage1, &proj, &ptr)
                .await
                .expect("rehydrate empty s19 plane")
                .with_oracle(Arc::clone(&oracle_arc));
            let persist1 = plane1.spawn_storage_persistence_loop(
                storage1.clone(),
                proj.clone(),
                ptr.clone(),
                agent.clone(),
            );
            let host1 = MyGoBridgeHost::new(
                marlin_agent_runtime::plugin::ReleaseTrust::new(BTreeMap::new()),
                |_meta| Ok(()),
            )
            .with_task_actors(plane1.clone());

            host1
                .task_actors()
                .register_actor(RegisterTaskActorRequest {
                    task_id: tid.clone(),
                    title: "S19 Multi-Thread Oracle Actor".to_owned(),
                    scope_mode: TaskActorScopeMode::Project,
                })
                .expect("register s19 actor");

            // Cross-thread Scheme Oracle admits valid ApplySteering & rejects steering without join.
            host1
                .task_actors()
                .admit_dispatch(MarlinDispatchEnvelope {
                    request_id: DispatchRequestId::new("req-s19-ok").unwrap(),
                    task_id: tid.clone(),
                    operation: GraphDispatchOperation::ApplySteering,
                    fencing_epoch: FencingEpoch::new(1).unwrap(),
                    scope_mode: TaskActorScopeMode::Project,
                    join_node_id: Some(GraphConvergenceNodeId::new("join-s19").unwrap()),
                    substrate_target: GraphSubstrateTarget::SqliteTurso,
                })
                .expect("mt Scheme oracle admits valid steering");

            assert!(matches!(
                host1.task_actors().admit_dispatch(MarlinDispatchEnvelope {
                    request_id: DispatchRequestId::new("req-s19-bad").unwrap(),
                    task_id: tid.clone(),
                    operation: GraphDispatchOperation::ApplySteering,
                    fencing_epoch: FencingEpoch::new(1).unwrap(),
                    scope_mode: TaskActorScopeMode::Project,
                    join_node_id: None,
                    substrate_target: GraphSubstrateTarget::SqliteTurso,
                }),
                Err(
                    marlin_agent_runtime::TaskActorRuntimeError::DispatchRejected(
                        GraphDispatchRejection::SteeringRequiresJoinNode
                    )
                )
            ));

            // Cross-thread Scheme Oracle reconciles receipt (epoch 1 -> 2, WaitingSteering).
            let decision = host1
                .task_actors()
                .reconcile_receipt(&MarlinExecutionReceipt {
                    request_id: DispatchRequestId::new("req-s19-ok").unwrap(),
                    task_id: tid.clone(),
                    fencing_epoch: FencingEpoch::new(1).unwrap(),
                    status: ExecutionReceiptStatus::Ok,
                })
                .expect("mt Scheme oracle reconciles receipt");
            assert!(decision.accepted);
            assert_eq!(decision.next_epoch, FencingEpoch::new(2).unwrap());
            assert_eq!(decision.next_status, TaskActorStatus::WaitingSteering);

            // Cross-thread Scheme Oracle gates governed context & Workbench tabs.
            let ctx = GovernedContextCapsule {
                task_id: tid.clone(),
                context_cut: 4,
                kv_prefix_digest: KvPrefixDigest::new("sha256:s19-mt-kv").unwrap(),
                disclosure_depth: DisclosureDepth::L3,
                has_active_defeater: true,
                quarantined_memory_count: 1,
                sandbox_admitted: true,
                cedar_handoff_valid: true,
                detour_mode: DetourMode::BlockerActive,
                auto_archive_enabled: false,
            };
            host1
                .task_actors()
                .bind_governed_context(ctx.clone())
                .expect("mt Scheme oracle admits L3 blocker context");
            let mut bad_ctx = ctx;
            bad_ctx.auto_archive_enabled = true;
            assert!(matches!(
                host1.task_actors().bind_governed_context(bad_ctx),
                Err(
                    marlin_agent_runtime::TaskActorRuntimeError::GovernedContextRejected(
                        GovernedContextRejection::BlockerDetourMustDisableAutoArchive
                    )
                )
            ));

            let tabs1 = host1
                .task_actors()
                .project_workbench_tabs()
                .expect("mt Scheme oracle projects tabs");
            assert_eq!(tabs1.tabs.len(), 2);
            assert_eq!(tabs1.tabs[0].kind, WorkbenchTabKind::PinnedIndex);
            assert_eq!(tabs1.tabs[1].id, "task-s19-mt");
            assert_eq!(tabs1.tabs[1].fencing_epoch, 2);

            persist1.flush_and_stop().await.expect("flush s19 boot #1");
            drop(host1);
            drop(plane1);
            drop(storage1);

            // Boot #7 Cold Restart on separate OS thread -> rehydrate -> re-verify via live Scheme Oracle!
            let storage2 = marlin_agent_storage::TursoAgentStorage::open_local(cfg)
                .await
                .expect("open s19 turso #2");
            let plane2 = TaskActorControlPlane::rehydrate_or_new(&storage2, &proj, &ptr)
                .await
                .expect("rehydrate s19 boot #2")
                .with_oracle(oracle_arc);
            let tabs2 = plane2
                .project_workbench_tabs()
                .expect("boot #7 mt Scheme oracle cross-check");
            assert_eq!(tabs2, tabs1);
        });
    });

    oracle_server.serve_until_closed(&live_oracle);
    mt_host_thread
        .join()
        .expect("multi-threaded host thread completed cleanly");

    // Verify fail-closed behavior once server half is dropped.
    let (closed_handle, closed_server) = graph_engine_oracle_channel(4);
    drop(closed_server);
    let closed_err = marlin_agent_graph::GraphEngineOracle::project_workbench_tabs(
        &closed_handle,
        None,
        &[],
        &[],
    )
    .expect_err("closed oracle channel must fail closed");
    assert!(closed_err.contains("closed"));
}
