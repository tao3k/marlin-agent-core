// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Session-free `TaskActor` kernel execution driver uniting
//! `marlin_agent_runtime::TaskActorRuntimeRegistry` (anti-split-brain fencing
//! epochs + Scoped Detours + optional Scheme `GraphEngineOracle` cross-check)
//! with async step execution inside `marlin-agent-kernel`.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use marlin_agent_runtime::{
    AdmittedMarlinDispatch, GraphEngineOracle, MarlinDispatchEnvelope, MarlinExecutionReceipt,
    ReceiptReconciliationDecision, TaskActorRuntimeError, TaskActorRuntimeRegistry,
};

/// Async step handler executing an admitted Session-free `MarlinDispatchEnvelope`
/// inside Marlin's native runtime (FunFlow / ModelGateway / Tool / NonoSandbox).
pub trait TaskActorStepHandler: Send + Sync {
    /// Executes `admitted` and returns the typed `MarlinExecutionReceipt` echoing
    /// the observed `request_id`, `task_id`, and `fencing_epoch`.
    fn execute<'a>(
        &'a self,
        admitted: AdmittedMarlinDispatch,
    ) -> Pin<Box<dyn Future<Output = MarlinExecutionReceipt> + Send + 'a>>;
}

/// Kernel execution driver coupling a Session-free `TaskActorRuntimeRegistry`
/// with a native `TaskActorStepHandler`.
#[derive(Clone)]
pub struct TaskActorKernelExecutor<H> {
    registry: TaskActorRuntimeRegistry,
    handler: Arc<H>,
}

impl<H: TaskActorStepHandler> TaskActorKernelExecutor<H> {
    /// Creates a new kernel executor over `registry` and `handler`.
    pub fn new(registry: TaskActorRuntimeRegistry, handler: Arc<H>) -> Self {
        Self { registry, handler }
    }

    /// Borrows the underlying Session-free `TaskActorRuntimeRegistry`.
    pub fn registry(&self) -> &TaskActorRuntimeRegistry {
        &self.registry
    }

    /// Admits `envelope`, runs it through `self.handler`, and reconciles the
    /// resulting `MarlinExecutionReceipt` against the actor's `FencingEpoch`.
    pub async fn dispatch_and_reconcile(
        &self,
        envelope: MarlinDispatchEnvelope,
    ) -> Result<(AdmittedMarlinDispatch, ReceiptReconciliationDecision), TaskActorRuntimeError>
    {
        let handler = self.handler.clone();
        self.registry
            .execute_and_reconcile(envelope, move |admitted| async move {
                handler.execute(admitted).await
            })
            .await
    }

    /// Cross-checks `envelope` AND its resulting receipt through an authoritative
    /// Scheme `GraphEngineOracle` (e.g. `GraphEngineLinkedBridge`) fail-closed!
    pub async fn dispatch_and_reconcile_with_oracle<O: GraphEngineOracle>(
        &self,
        oracle: &O,
        envelope: MarlinDispatchEnvelope,
    ) -> Result<(AdmittedMarlinDispatch, ReceiptReconciliationDecision), TaskActorRuntimeError>
    {
        let admitted = self.registry.admit_dispatch_with_oracle(oracle, envelope)?;
        let receipt = self.handler.execute(admitted.clone()).await;
        let decision = self
            .registry
            .reconcile_receipt_with_oracle(oracle, &receipt)?;
        Ok((admitted, decision))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use marlin_agent_runtime::{
        AdmittedGovernedContext, DispatchRequestId, ExecutionReceiptStatus, FencingEpoch,
        GovernedContextCapsule, GovernedContextRejection, GraphDispatchOperation,
        GraphDispatchRejection, GraphSubstrateTarget, ReceiptReconciliationReason, TaskActorCursor,
        TaskActorId, TaskActorScopeMode, WorkbenchActiveTaskInput, WorkbenchTabProjection,
    };

    struct EchoStepHandler {
        override_epoch: Option<u64>,
    }

    impl TaskActorStepHandler for EchoStepHandler {
        fn execute<'a>(
            &'a self,
            admitted: AdmittedMarlinDispatch,
        ) -> Pin<Box<dyn Future<Output = MarlinExecutionReceipt> + Send + 'a>> {
            Box::pin(async move {
                let env = admitted.envelope();
                let epoch = self
                    .override_epoch
                    .map(|e| FencingEpoch::new(e).unwrap())
                    .unwrap_or(env.fencing_epoch);
                MarlinExecutionReceipt {
                    request_id: env.request_id.clone(),
                    task_id: env.task_id.clone(),
                    fencing_epoch: epoch,
                    status: ExecutionReceiptStatus::Ok,
                }
            })
        }
    }

    struct LocalParityOracle;

    impl GraphEngineOracle for LocalParityOracle {
        fn project_dispatch(
            &self,
            envelope: &MarlinDispatchEnvelope,
        ) -> Result<Result<AdmittedMarlinDispatch, GraphDispatchRejection>, String> {
            Ok(marlin_agent_runtime::TaskActorRuntimeRegistry::new()
                .admit_dispatch(envelope.clone())
                .map_err(|err| match err {
                    TaskActorRuntimeError::DispatchRejected(r) => r,
                    _ => GraphDispatchRejection::InvalidFencingEpoch,
                }))
        }

        fn reconcile_receipt(
            &self,
            cursor: &TaskActorCursor,
            receipt: &MarlinExecutionReceipt,
        ) -> Result<ReceiptReconciliationDecision, String> {
            let next_epoch = if receipt.fencing_epoch == cursor.fencing_epoch {
                cursor.fencing_epoch.next()
            } else {
                cursor.fencing_epoch
            };
            let accepted = receipt.fencing_epoch == cursor.fencing_epoch;
            Ok(ReceiptReconciliationDecision {
                accepted,
                next_epoch,
                next_status: if accepted {
                    marlin_agent_runtime::TaskActorStatus::Running
                } else {
                    cursor.status
                },
                reason: if accepted {
                    ReceiptReconciliationReason::Accepted
                } else {
                    ReceiptReconciliationReason::StaleFencingEpoch
                },
            })
        }

        fn project_workbench_tabs(
            &self,
            _focused_task_id: Option<&TaskActorId>,
            _active_tasks: &[WorkbenchActiveTaskInput],
            _peek_task_ids: &[TaskActorId],
        ) -> Result<WorkbenchTabProjection, String> {
            Ok(WorkbenchTabProjection {
                focused_task_id: None,
                tabs: Vec::new(),
            })
        }

        fn project_governed_context(
            &self,
            _capsule: &GovernedContextCapsule,
        ) -> Result<Result<AdmittedGovernedContext, GovernedContextRejection>, String> {
            Err("unused in kernel test".to_owned())
        }
    }

    #[tokio::test]
    async fn kernel_executor_advances_epoch_and_blocks_stale_split_brain_receipts() {
        let registry = TaskActorRuntimeRegistry::new();
        let task_id = TaskActorId::new("task-kernel-01").unwrap();
        registry
            .register_actor(
                task_id.clone(),
                "Kernel Graph Task",
                TaskActorScopeMode::Project,
            )
            .unwrap();

        let executor = TaskActorKernelExecutor::new(
            registry.clone(),
            Arc::new(EchoStepHandler {
                override_epoch: None,
            }),
        );
        let (admitted, decision) = executor
            .dispatch_and_reconcile(MarlinDispatchEnvelope {
                request_id: DispatchRequestId::new("req-k1").unwrap(),
                task_id: task_id.clone(),
                operation: GraphDispatchOperation::ExecuteFrontier,
                fencing_epoch: FencingEpoch::new(1).unwrap(),
                scope_mode: TaskActorScopeMode::Project,
                join_node_id: None,
                substrate_target: GraphSubstrateTarget::SqliteTurso,
            })
            .await
            .unwrap();
        assert_eq!(admitted.envelope().fencing_epoch.value(), 1);
        assert!(decision.accepted);
        assert_eq!(decision.next_epoch.value(), 2);

        // Stale handler returning epoch 1 when cursor is at epoch 2 MUST be rejected!
        let stale_executor = TaskActorKernelExecutor::new(
            registry.clone(),
            Arc::new(EchoStepHandler {
                override_epoch: Some(1),
            }),
        );
        let (_admitted2, stale_decision) = stale_executor
            .dispatch_and_reconcile(MarlinDispatchEnvelope {
                request_id: DispatchRequestId::new("req-k2").unwrap(),
                task_id: task_id.clone(),
                operation: GraphDispatchOperation::ExecuteFrontier,
                fencing_epoch: FencingEpoch::new(2).unwrap(),
                scope_mode: TaskActorScopeMode::Project,
                join_node_id: None,
                substrate_target: GraphSubstrateTarget::SqliteTurso,
            })
            .await
            .unwrap();
        assert!(!stale_decision.accepted);
        assert_eq!(
            stale_decision.reason,
            ReceiptReconciliationReason::StaleFencingEpoch
        );
        assert_eq!(registry.cursor(&task_id).unwrap().fencing_epoch.value(), 2);

        // Oracle cross-check catches stale receipt cleanly without divergence.
        let oracle = LocalParityOracle;
        let oracle_decision = registry
            .reconcile_receipt_with_oracle(
                &oracle,
                &MarlinExecutionReceipt {
                    request_id: DispatchRequestId::new("req-k2").unwrap(),
                    task_id,
                    fencing_epoch: FencingEpoch::new(1).unwrap(),
                    status: ExecutionReceiptStatus::Ok,
                },
            )
            .unwrap();
        assert!(!oracle_decision.accepted);
        assert_eq!(
            oracle_decision.reason,
            ReceiptReconciliationReason::StaleFencingEpoch
        );
    }
}
