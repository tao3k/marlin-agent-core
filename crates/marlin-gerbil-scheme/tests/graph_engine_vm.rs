// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Real embedded Gambit/Gerbil VM round-trip verification for all 4 `GraphEngine`
//! rooted-vector C-ABI schemas (dispatch, reconcile, workbench-tabs, governed-context).

use std::collections::BTreeMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
};

use gerbil_scheme::{
    CollectionKind, GerbilRuntime, IdentifierBuffer, IdentifierKind, ObjectKind,
    PackageResolutionProgress, RootedSchemeObject, SchemeScalar,
};
use marlin_agent_graph::{
    DetourMode, DisclosureDepth, DispatchRequestId, ExecutionReceiptStatus, FencingEpoch,
    GovernedContextCapsule, GovernedContextRejection, GraphConvergenceNodeId,
    GraphDispatchOperation, GraphDispatchRejection, GraphSubstrateTarget, KvPrefixDigest,
    MarlinDispatchEnvelope, MarlinExecutionReceipt, ReceiptReconciliationReason, TaskActorId,
    TaskActorScopeMode, TaskActorStatus, WorkbenchActiveTaskInput, WorkbenchTabKind,
};
use marlin_agent_protocol::{
    ModelEndpoint, ModelGatewayRequest, system_gateway_message, user_gateway_message,
};
use marlin_agent_runtime::{
    AtomicEffect, AtomicEffectExecutor, AtomicEffectReceipt, CasGetEffectPayload,
    CasHitReceiptPayload, CasPutEffectPayload, EffectErrorReceiptPayload, LlmCompleteEffectPayload,
    LlmCompletedReceiptPayload, SandboxExitedReceiptPayload, SandboxRunEffectPayload,
    TaskActorRuntimeRegistry, run_atomic_effect_batch,
};
use marlin_gerbil_scheme::{
    GraphEngineDispatchProjection, GraphEngineGovernedContextProjection,
    GraphEngineProjectionError, GraphEngineWorkbenchTaskRow, decode_graph_engine_dispatch_vector,
    decode_graph_engine_governed_context_vector, decode_graph_engine_reconcile_vector,
    decode_graph_engine_workbench_tabs_vector, encode_graph_engine_dispatch_request,
    encode_graph_engine_governed_context_request, encode_graph_engine_reconcile_request,
    encode_graph_engine_workbench_tabs_request, gerbil_package_root,
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

    fn read_opt_str(obj: RootedSchemeObject<'_>) -> Option<String> {
        match obj.as_ref().kind().unwrap() {
            ObjectKind::String => Some(read_str(obj)),
            _ => None,
        }
    }

    fn read_fx(obj: RootedSchemeObject<'_>) -> isize {
        match obj.as_ref().scalar().unwrap() {
            SchemeScalar::Fixnum(n) => n,
            other => panic!("expected fixnum, got {other:?}"),
        }
    }

    fn decode_effect<'vm>(obj: RootedSchemeObject<'vm>) -> AtomicEffect {
        let v = obj.into_collection(CollectionKind::Vector).unwrap();
        let kind = read_sym(v.element(0).unwrap());
        let id = read_str(v.element(1).unwrap());
        match kind.as_str() {
            "cas-get" => AtomicEffect::CasGet(CasGetEffectPayload {
                effect_id: id,
                key: read_str(v.element(2).unwrap()),
            }),
            "cas-put" => AtomicEffect::CasPut(CasPutEffectPayload {
                effect_id: id,
                key: read_str(v.element(2).unwrap()),
                bytes: read_str(v.element(3).unwrap()).into_bytes(),
            }),
            "llm-complete" => {
                let endpoint_raw = read_str(v.element(2).unwrap());
                let (prov, model) = endpoint_raw
                    .split_once('/')
                    .unwrap_or(("openai", endpoint_raw.as_str()));
                let msgs_vec = v
                    .element(3)
                    .unwrap()
                    .into_collection(CollectionKind::Vector)
                    .unwrap();
                let mut messages = Vec::with_capacity(msgs_vec.len());
                for i in 0..msgs_vec.len() {
                    let pair = msgs_vec
                        .element(i)
                        .unwrap()
                        .into_collection(CollectionKind::Vector)
                        .unwrap();
                    let role = read_sym(pair.element(0).unwrap());
                    let body = read_str(pair.element(1).unwrap());
                    messages.push(if role == "system" {
                        system_gateway_message(body)
                    } else {
                        user_gateway_message(body)
                    });
                }
                AtomicEffect::LlmComplete(LlmCompleteEffectPayload {
                    effect_id: id,
                    request: ModelGatewayRequest::new(ModelEndpoint::new(prov, model), messages),
                })
            }
            "sandbox-run" => {
                let prog = read_str(v.element(2).unwrap());
                let args_vec = v
                    .element(3)
                    .unwrap()
                    .into_collection(CollectionKind::Vector)
                    .unwrap();
                let mut args = Vec::with_capacity(args_vec.len());
                for i in 0..args_vec.len() {
                    args.push(read_str(args_vec.element(i).unwrap()));
                }
                let cwd = read_opt_str(v.element(4).unwrap());
                AtomicEffect::SandboxRun(SandboxRunEffectPayload {
                    effect_id: id,
                    program: prog,
                    args,
                    cwd,
                })
            }
            other => panic!("unexpected atomic effect kind from Scheme VM: {other}"),
        }
    }

    fn encode_receipt<'vm>(
        rt: &'vm GerbilRuntime,
        receipt: &AtomicEffectReceipt,
    ) -> RootedSchemeObject<'vm> {
        match receipt {
            AtomicEffectReceipt::CasHit(p) => vec_obj(
                rt,
                &[
                    sym(rt, "cas-hit"),
                    txt(rt, &p.effect_id),
                    txt(rt, &String::from_utf8_lossy(&p.bytes)),
                ],
            ),
            AtomicEffectReceipt::CasMiss(id) => vec_obj(rt, &[sym(rt, "cas-miss"), txt(rt, id)]),
            AtomicEffectReceipt::CasStored(id) => {
                vec_obj(rt, &[sym(rt, "cas-stored"), txt(rt, id)])
            }
            AtomicEffectReceipt::LlmCompleted(p) => vec_obj(
                rt,
                &[
                    sym(rt, "llm-completed"),
                    txt(rt, &p.effect_id),
                    txt(rt, &p.content),
                ],
            ),
            AtomicEffectReceipt::SandboxExited(p) => vec_obj(
                rt,
                &[
                    sym(rt, "sandbox-exited"),
                    txt(rt, &p.effect_id),
                    fx(rt, p.exit_code as isize),
                    txt(rt, &String::from_utf8_lossy(&p.stdout)),
                    txt(rt, &String::from_utf8_lossy(&p.stderr)),
                ],
            ),
            AtomicEffectReceipt::EffectError(EffectErrorReceiptPayload { effect_id, message }) => {
                vec_obj(
                    rt,
                    &[
                        sym(rt, "effect-error"),
                        txt(rt, effect_id),
                        txt(rt, message),
                    ],
                )
            }
        }
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

    struct TursoLeafSubstrate {
        storage: marlin_agent_storage::TursoAgentStorage,
        project_id: marlin_agent_storage::ProjectId,
        session_id: marlin_agent_storage::SessionId,
        agent_id: marlin_agent_storage::AgentId,
        seq: AtomicU64,
    }

    impl TursoLeafSubstrate {
        fn new(storage: marlin_agent_storage::TursoAgentStorage) -> Self {
            Self {
                storage,
                project_id: marlin_agent_storage::ProjectId::new("proj-vm-turso").unwrap(),
                session_id: marlin_agent_storage::SessionId::new("vm-turso-pump").unwrap(),
                agent_id: marlin_agent_storage::AgentId::new("gerbil-vm-agent").unwrap(),
                seq: AtomicU64::new(1),
            }
        }

        fn pointer_key_for(key: &str) -> marlin_agent_storage::ArtifactPointerKey {
            let digest = marlin_agent_context::governed_bridge::compute_kv_prefix_digest(key);
            let hex = digest
                .as_str()
                .strip_prefix("sha256:")
                .unwrap_or(digest.as_str());
            marlin_agent_storage::ArtifactPointerKey::new(format!("ptr:cas:{hex}")).unwrap()
        }
    }

    impl AtomicEffectExecutor for TursoLeafSubstrate {
        fn execute_atomic<'a>(
            &'a self,
            effect: AtomicEffect,
        ) -> Pin<Box<dyn Future<Output = AtomicEffectReceipt> + Send + 'a>> {
            use marlin_agent_storage::AgentStorage;
            Box::pin(async move {
                match effect {
                    AtomicEffect::CasGet(p) => {
                        let ptr_key = Self::pointer_key_for(&p.key);
                        let ptr = self
                            .storage
                            .get_artifact_pointer(&self.project_id, &ptr_key)
                            .await
                            .unwrap();
                        if let Some(pointer) = ptr {
                            let art = self
                                .storage
                                .get_artifact(&self.project_id, &pointer.target_artifact_hash)
                                .await
                                .unwrap()
                                .expect("turso cas artifact must exist");
                            AtomicEffectReceipt::CasHit(CasHitReceiptPayload {
                                effect_id: p.effect_id,
                                bytes: art.body,
                            })
                        } else {
                            AtomicEffectReceipt::CasMiss(p.effect_id)
                        }
                    }
                    AtomicEffect::CasPut(p) => {
                        let body_text = String::from_utf8_lossy(&p.bytes);
                        let digest =
                            marlin_agent_context::governed_bridge::compute_kv_prefix_digest(
                                &body_text,
                            );
                        let hash =
                            marlin_agent_storage::ArtifactHash::new(digest.as_str().to_owned())
                                .unwrap();
                        let n = self.seq.fetch_add(1, Ordering::Relaxed);
                        let event_id =
                            marlin_agent_storage::EventId::new(format!("ev-vm-cas-{n}")).unwrap();
                        self.storage
                            .put_artifact(marlin_agent_storage::ArtifactRecord {
                                project_id: self.project_id.clone(),
                                artifact_hash: hash.clone(),
                                artifact_kind: "scheme-cas-blob".to_owned(),
                                producer_session_id: self.session_id.clone(),
                                producer_agent_id: self.agent_id.clone(),
                                producer_event_id: event_id.clone(),
                                media_type: "text/plain".to_owned(),
                                size_bytes: p.bytes.len() as u64,
                                body: p.bytes,
                                created_at_unix_ms: n as i64,
                            })
                            .await
                            .unwrap();
                        let ptr_key = Self::pointer_key_for(&p.key);
                        let expected = self
                            .storage
                            .get_artifact_pointer(&self.project_id, &ptr_key)
                            .await
                            .unwrap()
                            .map(|ptr| ptr.target_artifact_hash);
                        self.storage
                            .compare_and_swap_artifact_pointer(
                                marlin_agent_storage::ArtifactPointerUpdate {
                                    project_id: self.project_id.clone(),
                                    pointer_key: ptr_key,
                                    expected_artifact_hash: expected,
                                    new_artifact_hash: hash,
                                    updated_by_session_id: self.session_id.clone(),
                                    updated_by_agent_id: self.agent_id.clone(),
                                    updated_by_event_id: event_id,
                                    updated_at_unix_ms: n as i64,
                                },
                            )
                            .await
                            .unwrap();
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
                            content: format!("turso-llm:{tail}"),
                        })
                    }
                    AtomicEffect::SandboxRun(p) => {
                        AtomicEffectReceipt::SandboxExited(SandboxExitedReceiptPayload {
                            effect_id: p.effect_id,
                            exit_code: 0,
                            stdout: format!("turso-ran:{}", p.program).into_bytes(),
                            stderr: Vec::new(),
                        })
                    }
                }
            })
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn pump_fleet_rounds<'vm>(
        runtime: &'vm GerbilRuntime,
        step_proc: &RootedSchemeObject<'vm>,
        cp_proc: &RootedSchemeObject<'vm>,
        rt_tokio: &tokio::runtime::Runtime,
        substrate: &Arc<TursoLeafSubstrate>,
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
    let turso_sub1 = Arc::new(TursoLeafSubstrate::new(storage1));
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
    let turso_sub2 = Arc::new(TursoLeafSubstrate::new(storage2));
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
}
