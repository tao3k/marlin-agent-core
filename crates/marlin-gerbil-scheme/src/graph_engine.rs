// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Rooted Gerbil Scheme C-ABI projection for Marlin's Session-free `GraphEngine`
//! `TaskActor` dispatch, anti-split-brain receipt reconciliation, and Workbench tabs.

use std::collections::BTreeSet;
use std::fmt::{self, Display};

use gerbil_scheme::{
    CollectionKind, IdentifierBuffer, IdentifierKind, ObjectKind, RootedSchemeObject, SchemeScalar,
};
use marlin_agent_graph::{
    AdmittedMarlinDispatch, DispatchRequestId, FencingEpoch, GraphConvergenceNodeId,
    GraphDispatchOperation, GraphDispatchRejection, GraphSubstrateTarget, MarlinDispatchEnvelope,
    ReceiptReconciliationDecision, ReceiptReconciliationReason, TaskActorId, TaskActorScopeMode,
    TaskActorStatus, WorkbenchTabEntry, WorkbenchTabKind, WorkbenchTabProjection,
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
}
