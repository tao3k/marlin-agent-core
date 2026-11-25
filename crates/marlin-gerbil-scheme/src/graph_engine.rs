// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Rooted Gerbil Scheme C-ABI projection for Marlin's Session-free `GraphEngine`
//! `TaskActor` dispatch, anti-split-brain receipt reconciliation, and Workbench tabs.

use std::collections::BTreeSet;
use std::fmt::{self, Display};

use gerbil_scheme::{
    CollectionKind, GerbilRuntime, IdentifierBuffer, IdentifierKind, ObjectKind,
    RootedSchemeObject, SchemeScalar,
};
use marlin_agent_graph::{
    AdmittedMarlinDispatch, DispatchRequestId, ExecutionReceiptStatus, FencingEpoch,
    GraphConvergenceNodeId, GraphDispatchOperation, GraphDispatchRejection, GraphSubstrateTarget,
    MarlinDispatchEnvelope, MarlinExecutionReceipt, ReceiptReconciliationDecision,
    ReceiptReconciliationReason, TaskActorCursor, TaskActorId, TaskActorScopeMode, TaskActorStatus,
    WorkbenchActiveTaskInput, WorkbenchTabEntry, WorkbenchTabKind, WorkbenchTabProjection,
    admit_marlin_dispatch,
};

use crate::{
    GerbilSchemeAbiSymbol, GerbilSchemeCAbiContract, GerbilSchemeCAbiId,
    GerbilSchemeCAbiReadinessPlan,
};

/// Native ABI id for Marlin's rooted `GraphEngine` projection boundary.
pub const MARLIN_GRAPH_ENGINE_ABI_ID: &str = "marlin.agent.graph-engine.native-projection";

/// Native ABI version expected for `GraphEngine` rooted vector payloads.
pub const MARLIN_GRAPH_ENGINE_ABI_VERSION: u32 = 1;

/// C-ABI symbol projecting a `MarlinDispatchEnvelope` rooted vector.
pub const MARLIN_GRAPH_ENGINE_DISPATCH_SYMBOL: &str = "marlin_graph_engine_dispatch_vector";

/// C-ABI symbol reconciling a `MarlinExecutionReceipt` rooted vector.
pub const MARLIN_GRAPH_ENGINE_RECONCILE_SYMBOL: &str =
    "marlin_graph_engine_reconcile_receipt_vector";

/// C-ABI symbol projecting a `WorkbenchTabProjection` rooted vector.
pub const MARLIN_GRAPH_ENGINE_WORKBENCH_TABS_SYMBOL: &str =
    "marlin_graph_engine_workbench_tabs_vector";

const MAX_IDENTIFIER_BYTES: usize = 256;
const MAX_REASON_BYTES: usize = 512;
const MAX_WORKBENCH_TABS: usize = 256;

/// Native ABI contract declared by `scheme/c-abi/graph-engine.ss`.
pub fn marlin_graph_engine_abi_contract() -> GerbilSchemeCAbiContract {
    GerbilSchemeCAbiContract::new(
        GerbilSchemeCAbiId::new(MARLIN_GRAPH_ENGINE_ABI_ID),
        MARLIN_GRAPH_ENGINE_ABI_VERSION,
    )
    .with_exported_symbols([
        GerbilSchemeAbiSymbol::new(MARLIN_GRAPH_ENGINE_DISPATCH_SYMBOL),
        GerbilSchemeAbiSymbol::new(MARLIN_GRAPH_ENGINE_RECONCILE_SYMBOL),
        GerbilSchemeAbiSymbol::new(MARLIN_GRAPH_ENGINE_WORKBENCH_TABS_SYMBOL),
    ])
}

/// Readiness plan verified before invoking `scheme/c-abi/graph-engine.ss`.
pub fn marlin_graph_engine_readiness_plan() -> GerbilSchemeCAbiReadinessPlan {
    GerbilSchemeCAbiReadinessPlan::new(
        GerbilSchemeCAbiId::new(MARLIN_GRAPH_ENGINE_ABI_ID),
        MARLIN_GRAPH_ENGINE_ABI_VERSION,
    )
    .with_exported_symbols([
        GerbilSchemeAbiSymbol::new(MARLIN_GRAPH_ENGINE_DISPATCH_SYMBOL),
        GerbilSchemeAbiSymbol::new(MARLIN_GRAPH_ENGINE_RECONCILE_SYMBOL),
        GerbilSchemeAbiSymbol::new(MARLIN_GRAPH_ENGINE_WORKBENCH_TABS_SYMBOL),
    ])
}

/// Decoded dispatch projection returned by `marlin_graph_engine_dispatch_vector`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GraphEngineDispatchProjection {
    /// Admitted by both Scheme validation and Rust's `admit_marlin_dispatch` gate.
    Admitted(AdmittedMarlinDispatch),
    /// Rejected fail-closed by Scheme validation with a structured reason.
    Rejected { reason: String },
}

/// Decode or admission rejection on the rooted `GraphEngine` C-ABI boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GraphEngineProjectionError {
    /// Native Gambit/Gerbil rooted-object extraction error.
    Native(String),
    /// Malformed vector arity, type, or invariant violation.
    Invalid(&'static str),
    /// Scheme marked a dispatch admitted, but Rust's readiness gate rejected it.
    DispatchGate(GraphDispatchRejection),
}

impl Display for GraphEngineProjectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Native(message) => {
                write!(formatter, "graph-engine native projection failed: {message}")
            }
            Self::Invalid(message) => {
                write!(formatter, "invalid graph-engine projection: {message}")
            }
            Self::DispatchGate(rejection) => {
                write!(formatter, "graph-engine dispatch gate rejected: {rejection}")
            }
        }
    }
}

impl std::error::Error for GraphEngineProjectionError {}

fn native<T>(result: Result<T, impl Display>) -> Result<T, GraphEngineProjectionError> {
    result.map_err(|error| GraphEngineProjectionError::Native(error.to_string()))
}

fn decode_bool(value: RootedSchemeObject<'_>) -> Result<bool, GraphEngineProjectionError> {
    match native(value.as_ref().scalar())? {
        SchemeScalar::Boolean(flag) => Ok(flag),
        _ => Err(GraphEngineProjectionError::Invalid("expected boolean")),
    }
}

fn decode_u64(value: RootedSchemeObject<'_>) -> Result<u64, GraphEngineProjectionError> {
    match native(value.as_ref().scalar())? {
        SchemeScalar::Fixnum(number) if number >= 0 => Ok(number as u64),
        _ => Err(GraphEngineProjectionError::Invalid(
            "expected non-negative fixnum",
        )),
    }
}

fn decode_symbol(value: RootedSchemeObject<'_>) -> Result<String, GraphEngineProjectionError> {
    if native(value.as_ref().kind())? != ObjectKind::Symbol {
        return Err(GraphEngineProjectionError::Invalid("expected symbol"));
    }
    let mut buffer = IdentifierBuffer::default();
    native(value.as_ref().copy_identifier_names(&mut buffer))?;
    let Some((IdentifierKind::Symbol, name)) = buffer.get(0) else {
        return Err(GraphEngineProjectionError::Invalid("invalid symbol"));
    };
    if buffer.len() != 1 || name.is_empty() || name.len() > MAX_IDENTIFIER_BYTES {
        return Err(GraphEngineProjectionError::Invalid("invalid symbol length"));
    }
    Ok(name.to_owned())
}

fn decode_string(
    value: RootedSchemeObject<'_>,
    max_bytes: usize,
) -> Result<String, GraphEngineProjectionError> {
    let string_obj = native(value.into_string())?;
    if native(string_obj.len().into_result())? > max_bytes {
        return Err(GraphEngineProjectionError::Invalid(
            "string exceeds byte limit",
        ));
    }
    let text = native(string_obj.to_string().into_result())?;
    if text.len() > max_bytes {
        return Err(GraphEngineProjectionError::Invalid(
            "string exceeds byte limit",
        ));
    }
    Ok(text)
}

fn decode_optional_string(
    value: RootedSchemeObject<'_>,
    max_bytes: usize,
) -> Result<Option<String>, GraphEngineProjectionError> {
    match native(value.as_ref().kind())? {
        ObjectKind::Boolean
            if matches!(
                native(value.as_ref().scalar())?,
                SchemeScalar::Boolean(false)
            ) =>
        {
            Ok(None)
        }
        ObjectKind::String => Ok(Some(decode_string(value, max_bytes)?)),
        _ => Err(GraphEngineProjectionError::Invalid(
            "expected #f or string value",
        )),
    }
}

fn parse_operation(name: &str) -> Result<GraphDispatchOperation, GraphEngineProjectionError> {
    match name {
        "execute-frontier" => Ok(GraphDispatchOperation::ExecuteFrontier),
        "apply-steering" => Ok(GraphDispatchOperation::ApplySteering),
        "converge-apex" => Ok(GraphDispatchOperation::ConvergeApex),
        "commit-settlement" => Ok(GraphDispatchOperation::CommitSettlement),
        _ => Err(GraphEngineProjectionError::Invalid(
            "unknown dispatch operation",
        )),
    }
}

fn parse_scope_mode(name: &str) -> Result<TaskActorScopeMode, GraphEngineProjectionError> {
    match name {
        "project" => Ok(TaskActorScopeMode::Project),
        "detached" => Ok(TaskActorScopeMode::Detached),
        _ => Err(GraphEngineProjectionError::Invalid("unknown scope mode")),
    }
}

fn parse_substrate(name: &str) -> Result<GraphSubstrateTarget, GraphEngineProjectionError> {
    match name {
        "pure-memory" => Ok(GraphSubstrateTarget::PureMemory),
        "sqlite-turso" => Ok(GraphSubstrateTarget::SqliteTurso),
        _ => Err(GraphEngineProjectionError::Invalid(
            "unknown substrate target",
        )),
    }
}

fn parse_actor_status(name: &str) -> Result<TaskActorStatus, GraphEngineProjectionError> {
    match name {
        "open-focus" => Ok(TaskActorStatus::OpenFocus),
        "running" => Ok(TaskActorStatus::Running),
        "waiting-steering" => Ok(TaskActorStatus::WaitingSteering),
        "settled" => Ok(TaskActorStatus::Settled),
        "failed" => Ok(TaskActorStatus::Failed),
        "aborted" => Ok(TaskActorStatus::Aborted),
        _ => Err(GraphEngineProjectionError::Invalid("unknown actor status")),
    }
}

fn parse_reconcile_reason(
    name: &str,
) -> Result<ReceiptReconciliationReason, GraphEngineProjectionError> {
    match name {
        "accepted" => Ok(ReceiptReconciliationReason::Accepted),
        "awaiting-steering-convergence" => {
            Ok(ReceiptReconciliationReason::AwaitingSteeringConvergence)
        }
        "settled" => Ok(ReceiptReconciliationReason::Settled),
        "execution-failed" => Ok(ReceiptReconciliationReason::ExecutionFailed),
        "task-id-mismatch" => Ok(ReceiptReconciliationReason::TaskIdMismatch),
        "actor-already-terminal" => Ok(ReceiptReconciliationReason::ActorAlreadyTerminal),
        "stale-fencing-epoch" => Ok(ReceiptReconciliationReason::StaleFencingEpoch),
        "future-fencing-epoch" => Ok(ReceiptReconciliationReason::FutureFencingEpoch),
        "cannot-settle-with-pending-steering" => {
            Ok(ReceiptReconciliationReason::CannotSettleWithPendingSteering)
        }
        _ => Err(GraphEngineProjectionError::Invalid(
            "unknown reconciliation reason",
        )),
    }
}

fn parse_tab_kind(name: &str) -> Result<WorkbenchTabKind, GraphEngineProjectionError> {
    match name {
        "pinned-index" => Ok(WorkbenchTabKind::PinnedIndex),
        "active-task" => Ok(WorkbenchTabKind::ActiveTask),
        "read-only-peek" => Ok(WorkbenchTabKind::ReadOnlyPeek),
        _ => Err(GraphEngineProjectionError::Invalid(
            "unknown workbench tab kind",
        )),
    }
}

/// Decodes the 10-slot rooted Scheme vector produced by `marlin_graph_engine_dispatch_vector`
/// and passes admitted envelopes through Rust's fail-closed `admit_marlin_dispatch` gate.
pub fn decode_graph_engine_dispatch_vector(
    value: RootedSchemeObject<'_>,
) -> Result<GraphEngineDispatchProjection, GraphEngineProjectionError> {
    let items = native(value.into_collection(CollectionKind::Vector))?;
    if items.len() != 10 {
        return Err(GraphEngineProjectionError::Invalid(
            "dispatch vector must have 10 slots",
        ));
    }
    let version = decode_u64(native(items.element(0))?)?;
    if version != u64::from(MARLIN_GRAPH_ENGINE_ABI_VERSION) {
        return Err(GraphEngineProjectionError::Invalid(
            "unsupported graph-engine ABI version",
        ));
    }
    let admitted = decode_bool(native(items.element(1))?)?;
    let request_id_raw = decode_string(native(items.element(2))?, MAX_IDENTIFIER_BYTES)?;
    let task_id_raw = decode_string(native(items.element(3))?, MAX_IDENTIFIER_BYTES)?;
    let op_sym = decode_symbol(native(items.element(4))?)?;
    let epoch_raw = decode_u64(native(items.element(5))?)?;
    let scope_sym = decode_symbol(native(items.element(6))?)?;
    let join_raw = decode_optional_string(native(items.element(7))?, MAX_IDENTIFIER_BYTES)?;
    let substrate_sym = decode_symbol(native(items.element(8))?)?;
    let reason_raw = decode_optional_string(native(items.element(9))?, MAX_REASON_BYTES)?;

    if !admitted {
        return Ok(GraphEngineDispatchProjection::Rejected {
            reason: reason_raw.unwrap_or_else(|| "dispatch-rejected".to_owned()),
        });
    }

    let request_id = DispatchRequestId::new(request_id_raw)
        .map_err(|_| GraphEngineProjectionError::Invalid("empty request_id"))?;
    let task_id = TaskActorId::new(task_id_raw)
        .map_err(|_| GraphEngineProjectionError::Invalid("empty task_id"))?;
    let operation = parse_operation(&op_sym)?;
    let fencing_epoch =
        FencingEpoch::new(epoch_raw).map_err(GraphEngineProjectionError::DispatchGate)?;
    let scope_mode = parse_scope_mode(&scope_sym)?;
    let join_node_id = match join_raw {
        Some(raw) => Some(
            GraphConvergenceNodeId::new(raw)
                .map_err(|_| GraphEngineProjectionError::Invalid("empty join_node_id"))?,
        ),
        None => None,
    };
    let substrate_target = parse_substrate(&substrate_sym)?;

    let envelope = MarlinDispatchEnvelope {
        request_id,
        task_id,
        operation,
        fencing_epoch,
        scope_mode,
        join_node_id,
        substrate_target,
    };
    let admitted_witness =
        admit_marlin_dispatch(envelope).map_err(GraphEngineProjectionError::DispatchGate)?;
    Ok(GraphEngineDispatchProjection::Admitted(admitted_witness))
}

/// Decodes the 5-slot rooted Scheme vector produced by `marlin_graph_engine_reconcile_receipt_vector`.
pub fn decode_graph_engine_reconcile_vector(
    value: RootedSchemeObject<'_>,
) -> Result<ReceiptReconciliationDecision, GraphEngineProjectionError> {
    let items = native(value.into_collection(CollectionKind::Vector))?;
    if items.len() != 5 {
        return Err(GraphEngineProjectionError::Invalid(
            "reconcile vector must have 5 slots",
        ));
    }
    let version = decode_u64(native(items.element(0))?)?;
    if version != u64::from(MARLIN_GRAPH_ENGINE_ABI_VERSION) {
        return Err(GraphEngineProjectionError::Invalid(
            "unsupported graph-engine ABI version",
        ));
    }
    let accepted = decode_bool(native(items.element(1))?)?;
    let next_epoch = FencingEpoch::new(decode_u64(native(items.element(2))?)?)
        .map_err(GraphEngineProjectionError::DispatchGate)?;
    let next_status = parse_actor_status(&decode_symbol(native(items.element(3))?)?)?;
    let reason = parse_reconcile_reason(&decode_symbol(native(items.element(4))?)?)?;

    Ok(ReceiptReconciliationDecision {
        accepted,
        next_epoch,
        next_status,
        reason,
    })
}

/// Decodes the 3-slot rooted Scheme vector produced by `marlin_graph_engine_workbench_tabs_vector`,
/// verifying slot 0 is `PinnedIndex` and read-only peeks are strictly disjoint from active tasks.
pub fn decode_graph_engine_workbench_tabs_vector(
    value: RootedSchemeObject<'_>,
) -> Result<WorkbenchTabProjection, GraphEngineProjectionError> {
    let items = native(value.into_collection(CollectionKind::Vector))?;
    if items.len() != 3 {
        return Err(GraphEngineProjectionError::Invalid(
            "workbench tabs vector must have 3 slots",
        ));
    }
    let version = decode_u64(native(items.element(0))?)?;
    if version != u64::from(MARLIN_GRAPH_ENGINE_ABI_VERSION) {
        return Err(GraphEngineProjectionError::Invalid(
            "unsupported graph-engine ABI version",
        ));
    }
    let focused_raw =
        decode_optional_string(native(items.element(1))?, MAX_IDENTIFIER_BYTES)?;
    let focused_task_id = match focused_raw {
        Some(raw) => Some(
            TaskActorId::new(raw)
                .map_err(|_| GraphEngineProjectionError::Invalid("empty focused_task_id"))?,
        ),
        None => None,
    };
    let raw_tabs = native(native(items.element(2))?.into_collection(CollectionKind::Vector))?;
    if raw_tabs.is_empty() || raw_tabs.len() > MAX_WORKBENCH_TABS {
        return Err(GraphEngineProjectionError::Invalid(
            "workbench tab count out of bounds",
        ));
    }

    let mut tabs = Vec::with_capacity(raw_tabs.len());
    let mut active_ids = BTreeSet::new();
    let mut peek_ids = BTreeSet::new();

    for index in 0..raw_tabs.len() {
        let entry_vec = native(native(raw_tabs.element(index))?.into_collection(CollectionKind::Vector))?;
        if entry_vec.len() != 6 {
            return Err(GraphEngineProjectionError::Invalid(
                "workbench tab entry must have 6 slots",
            ));
        }
        let kind = parse_tab_kind(&decode_symbol(native(entry_vec.element(0))?)?)?;
        let id = decode_string(native(entry_vec.element(1))?, MAX_IDENTIFIER_BYTES)?;
        let title = decode_string(native(entry_vec.element(2))?, MAX_IDENTIFIER_BYTES)?;
        let status_label = decode_symbol(native(entry_vec.element(3))?)?;
        let focused = decode_bool(native(entry_vec.element(4))?)?;
        let fencing_epoch = decode_u64(native(entry_vec.element(5))?)?;

        match kind {
            WorkbenchTabKind::PinnedIndex => {}
            WorkbenchTabKind::ActiveTask => {
                if !active_ids.insert(id.clone()) {
                    return Err(GraphEngineProjectionError::Invalid(
                        "duplicate active task tab",
                    ));
                }
            }
            WorkbenchTabKind::ReadOnlyPeek => {
                if active_ids.contains(&id) || !peek_ids.insert(id.clone()) {
                    return Err(GraphEngineProjectionError::Invalid(
                        "read-only peek overlaps active task or duplicates peek",
                    ));
                }
            }
        }

        tabs.push(WorkbenchTabEntry {
            kind,
            id,
            title,
            status_label,
            focused,
            fencing_epoch,
        });
    }

    if tabs[0].kind != WorkbenchTabKind::PinnedIndex {
        return Err(GraphEngineProjectionError::Invalid(
            "slot 0 must be pinned-index",
        ));
    }

    Ok(WorkbenchTabProjection {
        focused_task_id,
        tabs,
    })
}

/// Linked native C-ABI function pointers exported by `scheme/c-abi/graph-engine.ss`.
#[derive(Clone, Copy, Debug)]
pub struct GraphEngineLinkedExports {
    /// Exported symbol `marlin_graph_engine_dispatch_vector`.
    pub dispatch_vector: unsafe extern "C" fn(i64) -> i64,
    /// Exported symbol `marlin_graph_engine_reconcile_receipt_vector`.
    pub reconcile_receipt_vector: unsafe extern "C" fn(i64) -> i64,
    /// Exported symbol `marlin_graph_engine_workbench_tabs_vector`.
    pub workbench_tabs_vector: unsafe extern "C" fn(i64) -> i64,
}

/// Active-task row encoded into `marlin_graph_engine_workbench_tabs_vector`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphEngineWorkbenchTaskRow {
    /// Active task metadata.
    pub task: WorkbenchActiveTaskInput,
    /// Scope isolation mode for the active task.
    pub scope_mode: TaskActorScopeMode,
}

fn operation_symbol(operation: GraphDispatchOperation) -> &'static str {
    match operation {
        GraphDispatchOperation::ExecuteFrontier => "execute-frontier",
        GraphDispatchOperation::ApplySteering => "apply-steering",
        GraphDispatchOperation::ConvergeApex => "converge-apex",
        GraphDispatchOperation::CommitSettlement => "commit-settlement",
    }
}

fn scope_mode_symbol(scope_mode: TaskActorScopeMode) -> &'static str {
    match scope_mode {
        TaskActorScopeMode::Project => "project",
        TaskActorScopeMode::Detached => "detached",
    }
}

fn substrate_symbol(substrate: GraphSubstrateTarget) -> &'static str {
    match substrate {
        GraphSubstrateTarget::PureMemory => "pure-memory",
        GraphSubstrateTarget::SqliteTurso => "sqlite-turso",
    }
}

fn actor_status_symbol(status: TaskActorStatus) -> &'static str {
    match status {
        TaskActorStatus::OpenFocus => "open-focus",
        TaskActorStatus::Running => "running",
        TaskActorStatus::WaitingSteering => "waiting-steering",
        TaskActorStatus::Settled => "settled",
        TaskActorStatus::Failed => "failed",
        TaskActorStatus::Aborted => "aborted",
    }
}

fn receipt_status_symbol(status: ExecutionReceiptStatus) -> &'static str {
    match status {
        ExecutionReceiptStatus::Ok => "ok",
        ExecutionReceiptStatus::Settled => "settled",
        ExecutionReceiptStatus::Failed => "failed",
        ExecutionReceiptStatus::Rejected => "rejected",
    }
}

fn encode_u64<'vm>(
    runtime: &'vm GerbilRuntime,
    value: u64,
) -> Result<RootedSchemeObject<'vm>, GraphEngineProjectionError> {
    let fixnum = isize::try_from(value)
        .map_err(|_| GraphEngineProjectionError::Invalid("u64 overflows fixnum"))?;
    native(runtime.scalar_object(SchemeScalar::Fixnum(fixnum)))
}

fn encode_optional_str<'vm>(
    runtime: &'vm GerbilRuntime,
    value: Option<&str>,
) -> Result<RootedSchemeObject<'vm>, GraphEngineProjectionError> {
    match value {
        Some(text) => Ok(native(runtime.string_from_utf8(text))?.into_object()),
        None => native(runtime.scalar_object(SchemeScalar::Boolean(false))),
    }
}

/// Encodes a `MarlinDispatchEnvelope` into the 7-slot rooted vector expected by
/// `marlin_graph_engine_dispatch_vector`.
pub fn encode_graph_engine_dispatch_request<'vm>(
    runtime: &'vm GerbilRuntime,
    envelope: &MarlinDispatchEnvelope,
) -> Result<RootedSchemeObject<'vm>, GraphEngineProjectionError> {
    let request_id = native(runtime.string_from_utf8(envelope.request_id.as_str()))?.into_object();
    let task_id = native(runtime.string_from_utf8(envelope.task_id.as_str()))?.into_object();
    let op = native(
        runtime.intern_identifier(IdentifierKind::Symbol, operation_symbol(envelope.operation)),
    )?;
    let epoch = encode_u64(runtime, envelope.fencing_epoch.value())?;
    let scope = native(
        runtime.intern_identifier(
            IdentifierKind::Symbol,
            scope_mode_symbol(envelope.scope_mode),
        ),
    )?;
    let join = encode_optional_str(
        runtime,
        envelope.join_node_id.as_ref().map(|id| id.as_str()),
    )?;
    let substrate = native(runtime.intern_identifier(
        IdentifierKind::Symbol,
        substrate_symbol(envelope.substrate_target),
    ))?;
    Ok(native(runtime.collection_from_objects(
        CollectionKind::Vector,
        &[
            &request_id,
            &task_id,
            &op,
            &epoch,
            &scope,
            &join,
            &substrate,
        ],
    ))?
    .into_object())
}

/// Encodes a `(TaskActorCursor, MarlinExecutionReceipt)` pair into the 8-slot rooted vector
/// expected by `marlin_graph_engine_reconcile_receipt_vector`.
pub fn encode_graph_engine_reconcile_request<'vm>(
    runtime: &'vm GerbilRuntime,
    cursor: &TaskActorCursor,
    receipt: &MarlinExecutionReceipt,
) -> Result<RootedSchemeObject<'vm>, GraphEngineProjectionError> {
    let actor_task_id = native(runtime.string_from_utf8(cursor.task_id.as_str()))?.into_object();
    let actor_epoch = encode_u64(runtime, cursor.fencing_epoch.value())?;
    let actor_status = native(
        runtime.intern_identifier(IdentifierKind::Symbol, actor_status_symbol(cursor.status)),
    )?;
    let pending_steering = encode_u64(runtime, u64::from(cursor.pending_steering_branches))?;
    let receipt_req_id =
        native(runtime.string_from_utf8(receipt.request_id.as_str()))?.into_object();
    let receipt_task_id = native(runtime.string_from_utf8(receipt.task_id.as_str()))?.into_object();
    let receipt_epoch = encode_u64(runtime, receipt.fencing_epoch.value())?;
    let receipt_status = native(
        runtime.intern_identifier(IdentifierKind::Symbol, receipt_status_symbol(receipt.status)),
    )?;
    Ok(native(runtime.collection_from_objects(
        CollectionKind::Vector,
        &[
            &actor_task_id,
            &actor_epoch,
            &actor_status,
            &pending_steering,
            &receipt_req_id,
            &receipt_task_id,
            &receipt_epoch,
            &receipt_status,
        ],
    ))?
    .into_object())
}

/// Encodes Workbench tab inputs into the 3-slot rooted vector expected by
/// `marlin_graph_engine_workbench_tabs_vector`.
pub fn encode_graph_engine_workbench_tabs_request<'vm>(
    runtime: &'vm GerbilRuntime,
    focused_task_id: Option<&TaskActorId>,
    active_tasks: &[GraphEngineWorkbenchTaskRow],
    peek_task_ids: &[TaskActorId],
) -> Result<RootedSchemeObject<'vm>, GraphEngineProjectionError> {
    let focused = encode_optional_str(runtime, focused_task_id.map(|id| id.as_str()))?;
    let mut active_rows = Vec::with_capacity(active_tasks.len());
    for row in active_tasks {
        let tid = native(runtime.string_from_utf8(row.task.task_id.as_str()))?.into_object();
        let title = native(runtime.string_from_utf8(&row.task.title))?.into_object();
        let status = native(
            runtime.intern_identifier(IdentifierKind::Symbol, actor_status_symbol(row.task.status)),
        )?;
        let scope = native(
            runtime.intern_identifier(IdentifierKind::Symbol, scope_mode_symbol(row.scope_mode)),
        )?;
        let epoch = encode_u64(runtime, row.task.fencing_epoch.value())?;
        let vec = native(runtime.collection_from_objects(
            CollectionKind::Vector,
            &[&tid, &title, &status, &scope, &epoch],
        ))?
        .into_object();
        active_rows.push(vec);
    }
    let active_refs: Vec<&RootedSchemeObject<'vm>> = active_rows.iter().collect();
    let active_vec =
        native(runtime.collection_from_objects(CollectionKind::Vector, &active_refs))?
            .into_object();

    let mut peek_objs = Vec::with_capacity(peek_task_ids.len());
    for peek_id in peek_task_ids {
        peek_objs.push(native(runtime.string_from_utf8(peek_id.as_str()))?.into_object());
    }
    let peek_refs: Vec<&RootedSchemeObject<'vm>> = peek_objs.iter().collect();
    let peek_vec =
        native(runtime.collection_from_objects(CollectionKind::Vector, &peek_refs))?.into_object();

    Ok(native(runtime.collection_from_objects(
        CollectionKind::Vector,
        &[&focused, &active_vec, &peek_vec],
    ))?
    .into_object())
}

/// Invokes the linked `marlin_graph_engine_dispatch_vector` C-ABI symbol and decodes the result.
///
/// # Safety
/// `export` must belong to `runtime`'s initialized linked programme and return a fresh strong root.
pub unsafe fn invoke_linked_graph_engine_dispatch(
    runtime: &GerbilRuntime,
    export: unsafe extern "C" fn(i64) -> i64,
    envelope: &MarlinDispatchEnvelope,
) -> Result<GraphEngineDispatchProjection, GraphEngineProjectionError> {
    let procedure = native(unsafe { runtime.bind_object_export(export) })?;
    let input = encode_graph_engine_dispatch_request(runtime, envelope)?;
    let output = native(procedure.call(&input).into_result())?;
    decode_graph_engine_dispatch_vector(output)
}

/// Invokes the linked `marlin_graph_engine_reconcile_receipt_vector` C-ABI symbol and decodes the result.
///
/// # Safety
/// `export` must belong to `runtime`'s initialized linked programme and return a fresh strong root.
pub unsafe fn invoke_linked_graph_engine_reconcile(
    runtime: &GerbilRuntime,
    export: unsafe extern "C" fn(i64) -> i64,
    cursor: &TaskActorCursor,
    receipt: &MarlinExecutionReceipt,
) -> Result<ReceiptReconciliationDecision, GraphEngineProjectionError> {
    let procedure = native(unsafe { runtime.bind_object_export(export) })?;
    let input = encode_graph_engine_reconcile_request(runtime, cursor, receipt)?;
    let output = native(procedure.call(&input).into_result())?;
    decode_graph_engine_reconcile_vector(output)
}

/// Invokes the linked `marlin_graph_engine_workbench_tabs_vector` C-ABI symbol and decodes the result.
///
/// # Safety
/// `export` must belong to `runtime`'s initialized linked programme and return a fresh strong root.
pub unsafe fn invoke_linked_graph_engine_workbench_tabs(
    runtime: &GerbilRuntime,
    export: unsafe extern "C" fn(i64) -> i64,
    focused_task_id: Option<&TaskActorId>,
    active_tasks: &[GraphEngineWorkbenchTaskRow],
    peek_task_ids: &[TaskActorId],
) -> Result<WorkbenchTabProjection, GraphEngineProjectionError> {
    let procedure = native(unsafe { runtime.bind_object_export(export) })?;
    let input = encode_graph_engine_workbench_tabs_request(
        runtime,
        focused_task_id,
        active_tasks,
        peek_task_ids,
    )?;
    let output = native(procedure.call(&input).into_result())?;
    decode_graph_engine_workbench_tabs_vector(output)
}

/// Borrowed bridge binding a live `GerbilRuntime` with `scheme/c-abi/graph-engine.ss` exports.
pub struct GraphEngineLinkedBridge<'vm> {
    runtime: &'vm GerbilRuntime,
    exports: GraphEngineLinkedExports,
}

impl<'vm> GraphEngineLinkedBridge<'vm> {
    /// Creates a bridge over a live `GerbilRuntime` and its linked `GraphEngine` C-ABI symbols.
    ///
    /// # Safety
    /// All symbols in `exports` must belong to `runtime`'s initialized linked programme.
    pub unsafe fn new(runtime: &'vm GerbilRuntime, exports: GraphEngineLinkedExports) -> Self {
        Self { runtime, exports }
    }

    /// Projects and gates a `MarlinDispatchEnvelope` across the rooted Scheme C-ABI.
    pub fn project_dispatch(
        &self,
        envelope: &MarlinDispatchEnvelope,
    ) -> Result<GraphEngineDispatchProjection, GraphEngineProjectionError> {
        // SAFETY: `self.exports` were validated against `self.runtime` at construction.
        unsafe {
            invoke_linked_graph_engine_dispatch(
                self.runtime,
                self.exports.dispatch_vector,
                envelope,
            )
        }
    }

    /// Reconciles an execution receipt against a `TaskActorCursor` across the rooted Scheme C-ABI.
    pub fn reconcile_receipt(
        &self,
        cursor: &TaskActorCursor,
        receipt: &MarlinExecutionReceipt,
    ) -> Result<ReceiptReconciliationDecision, GraphEngineProjectionError> {
        // SAFETY: `self.exports` were validated against `self.runtime` at construction.
        unsafe {
            invoke_linked_graph_engine_reconcile(
                self.runtime,
                self.exports.reconcile_receipt_vector,
                cursor,
                receipt,
            )
        }
    }

    /// Projects pinned-Index Workbench tabs across the rooted Scheme C-ABI.
    pub fn project_workbench_tabs(
        &self,
        focused_task_id: Option<&TaskActorId>,
        active_tasks: &[GraphEngineWorkbenchTaskRow],
        peek_task_ids: &[TaskActorId],
    ) -> Result<WorkbenchTabProjection, GraphEngineProjectionError> {
        // SAFETY: `self.exports` were validated against `self.runtime` at construction.
        unsafe {
            invoke_linked_graph_engine_workbench_tabs(
                self.runtime,
                self.exports.workbench_tabs_vector,
                focused_task_id,
                active_tasks,
                peek_task_ids,
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graph_engine_abi_contract_and_readiness_plan_match_exported_symbols() {
        let contract = marlin_graph_engine_abi_contract();
        let plan = marlin_graph_engine_readiness_plan();
        assert_eq!(contract.abi_id.as_str(), MARLIN_GRAPH_ENGINE_ABI_ID);
        assert_eq!(contract.version, MARLIN_GRAPH_ENGINE_ABI_VERSION);
        assert_eq!(plan.abi_id, contract.abi_id);
        assert_eq!(plan.version, contract.version);
        assert_eq!(plan.exported_symbols.len(), 3);
        assert_eq!(
            plan.exported_symbols[0].as_str(),
            MARLIN_GRAPH_ENGINE_DISPATCH_SYMBOL
        );
        assert_eq!(
            plan.exported_symbols[1].as_str(),
            MARLIN_GRAPH_ENGINE_RECONCILE_SYMBOL
        );
        assert_eq!(
            plan.exported_symbols[2].as_str(),
            MARLIN_GRAPH_ENGINE_WORKBENCH_TABS_SYMBOL
        );
    }

    #[test]
    fn symbol_tables_round_trip_between_rust_and_scheme_names() {
        for op in [
            GraphDispatchOperation::ExecuteFrontier,
            GraphDispatchOperation::ApplySteering,
            GraphDispatchOperation::ConvergeApex,
            GraphDispatchOperation::CommitSettlement,
        ] {
            assert_eq!(parse_operation(operation_symbol(op)).unwrap(), op);
        }
        for scope in [TaskActorScopeMode::Project, TaskActorScopeMode::Detached] {
            assert_eq!(parse_scope_mode(scope_mode_symbol(scope)).unwrap(), scope);
        }
        for substrate in [
            GraphSubstrateTarget::PureMemory,
            GraphSubstrateTarget::SqliteTurso,
        ] {
            assert_eq!(parse_substrate(substrate_symbol(substrate)).unwrap(), substrate);
        }
        for status in [
            TaskActorStatus::OpenFocus,
            TaskActorStatus::Running,
            TaskActorStatus::WaitingSteering,
            TaskActorStatus::Settled,
            TaskActorStatus::Failed,
            TaskActorStatus::Aborted,
        ] {
            assert_eq!(
                parse_actor_status(actor_status_symbol(status)).unwrap(),
                status
            );
        }
        assert_eq!(receipt_status_symbol(ExecutionReceiptStatus::Ok), "ok");
        assert_eq!(
            receipt_status_symbol(ExecutionReceiptStatus::Settled),
            "settled"
        );
        assert_eq!(
            receipt_status_symbol(ExecutionReceiptStatus::Failed),
            "failed"
        );
        assert_eq!(
            receipt_status_symbol(ExecutionReceiptStatus::Rejected),
            "rejected"
        );
    }
}
