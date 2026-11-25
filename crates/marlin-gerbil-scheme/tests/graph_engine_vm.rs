// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Real embedded Gambit/Gerbil VM round-trip verification for all 4 `GraphEngine`
//! rooted-vector C-ABI schemas (dispatch, reconcile, workbench-tabs, governed-context).

use gerbil_scheme::{
    CollectionKind, GerbilRuntime, IdentifierKind, PackageResolutionProgress, RootedSchemeObject,
    SchemeScalar,
};
use marlin_agent_graph::{
    DetourMode, DisclosureDepth, DispatchRequestId, ExecutionReceiptStatus, FencingEpoch,
    GovernedContextCapsule, GovernedContextRejection, GraphConvergenceNodeId,
    GraphDispatchOperation, GraphDispatchRejection, GraphSubstrateTarget, KvPrefixDigest,
    MarlinDispatchEnvelope, MarlinExecutionReceipt, ReceiptReconciliationReason, TaskActorId,
    TaskActorScopeMode, TaskActorStatus, WorkbenchActiveTaskInput, WorkbenchTabKind,
};
use marlin_agent_runtime::TaskActorRuntimeRegistry;
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
}
