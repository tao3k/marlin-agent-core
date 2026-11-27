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

use crate::{
    DerivationId, ExecutionCircuitBreaker, GovernanceAdmissionGate, GovernedSubjectName,
    ToolBudgetUnits, ToolIfcContext,
};

const DEFAULT_TASK_ACTOR_TOOL_BUDGET: u32 = 1024;

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
/// with a native `TaskActorStepHandler` and fail-closed `GovernanceAdmissionGate`
/// mediation (SM25).
#[derive(Clone)]
pub struct TaskActorKernelExecutor<H> {
    registry: TaskActorRuntimeRegistry,
    handler: Arc<H>,
    governance_gate: Arc<GovernanceAdmissionGate>,
    governance_actor: GovernedSubjectName,
    derivation_id: Option<DerivationId>,
    ifc_context: ToolIfcContext,
}

impl<H: TaskActorStepHandler> TaskActorKernelExecutor<H> {
    /// Creates a new kernel executor over `registry` and `handler`, backed by a
    /// live fail-closed `GovernanceAdmissionGate` (`ExecutionCircuitBreaker` +
    /// Cedar/ASCENT/TMS mediation).
    pub fn new(registry: TaskActorRuntimeRegistry, handler: Arc<H>) -> Self {
        let breaker = Arc::new(ExecutionCircuitBreaker::new(ToolBudgetUnits::new(
            DEFAULT_TASK_ACTOR_TOOL_BUDGET,
        )));
        let governance_gate = Arc::new(GovernanceAdmissionGate::new(breaker));
        let governance_actor =
            GovernedSubjectName::new("agent-task-actor").expect("valid static actor name");
        Self {
            registry,
            handler,
            governance_gate,
            governance_actor,
            derivation_id: None,
            ifc_context: ToolIfcContext::clean_trusted(),
        }
    }

    /// Mounts a shared authoritative `GovernanceAdmissionGate` onto this GraphEngine
    /// executor so operator E-Stop, Cedar `forbid` rules, ASCENT actor fencing, and
    /// Bi-Temporal TMS retractions block step execution fail-closed (SM25).
    pub fn with_governance_gate(
        mut self,
        gate: Arc<GovernanceAdmissionGate>,
        actor: GovernedSubjectName,
        derivation_id: Option<DerivationId>,
    ) -> Self {
        self.governance_gate = gate;
        self.governance_actor = actor;
        self.derivation_id = derivation_id;
        self
    }

    /// Configures the CaMeL/FIDES `ToolIfcContext` evaluated before each GraphEngine dispatch.
    pub fn with_ifc_context(mut self, ifc_context: ToolIfcContext) -> Self {
        self.ifc_context = ifc_context;
        self
    }

    /// Borrows the authoritative `GovernanceAdmissionGate` governing this executor.
    pub fn governance_gate(&self) -> &Arc<GovernanceAdmissionGate> {
        &self.governance_gate
    }

    /// Borrows the underlying Session-free `TaskActorRuntimeRegistry`.
    pub fn registry(&self) -> &TaskActorRuntimeRegistry {
        &self.registry
    }

    fn authorize_dispatch(
        &self,
        envelope: &MarlinDispatchEnvelope,
    ) -> Result<(), TaskActorRuntimeError> {
        let tool_name = GovernedSubjectName::new(format!("graph_dispatch:{:?}", envelope.operation))
            .unwrap_or_else(|_| GovernedSubjectName::new("graph_dispatch").unwrap());
        let resource_name = GovernedSubjectName::new(envelope.task_id.as_str())
            .unwrap_or_else(|_| GovernedSubjectName::new("task_actor").unwrap());
        self.governance_gate
            .authorize_side_effect_on_resource(
                self.derivation_id.as_ref(),
                &self.governance_actor,
                &tool_name,
                &resource_name,
                &self.ifc_context,
            )
            .map(|_| ())
            .map_err(|err| TaskActorRuntimeError::GovernanceDenied {
                reason: err.to_string(),
            })
    }

    /// Admits `envelope`, enforces `GovernanceAdmissionGate` fail-closed, runs it
    /// through `self.handler`, and reconciles the resulting `MarlinExecutionReceipt`
    /// against the actor's `FencingEpoch`.
    pub async fn dispatch_and_reconcile(
        &self,
        envelope: MarlinDispatchEnvelope,
    ) -> Result<(AdmittedMarlinDispatch, ReceiptReconciliationDecision), TaskActorRuntimeError>
    {
        self.authorize_dispatch(&envelope)?;
        let handler = self.handler.clone();
        self.registry
            .execute_and_reconcile(envelope, move |admitted| async move {
                handler.execute(admitted).await
            })
            .await
    }

    /// Cross-checks `envelope` AND its resulting receipt through an authoritative
    /// Scheme `GraphEngineOracle` (e.g. `GraphEngineLinkedBridge`) AND enforces
    /// `GovernanceAdmissionGate` fail-closed!
    pub async fn dispatch_and_reconcile_with_oracle<O: GraphEngineOracle>(
        &self,
        oracle: &O,
        envelope: MarlinDispatchEnvelope,
    ) -> Result<(AdmittedMarlinDispatch, ReceiptReconciliationDecision), TaskActorRuntimeError>
    {
        self.authorize_dispatch(&envelope)?;
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

    #[tokio::test]
    async fn sm25_adv_l4_task_actor_kernel_executor_must_block_dispatch_when_estop_tripped() {
        // ADV-L4 REAL-REPO SPLIT-BRAIN EXPLOIT:
        // Operator pulls E-Stop on GovernanceAdmissionGate; production GraphEngine
        // TaskActorKernelExecutor MUST refuse dispatch BEFORE invoking step handler.
        use crate::{ExecutionCircuitBreaker, GovernanceAdmissionGate, ToolBudgetUnits};
        use std::sync::atomic::{AtomicUsize, Ordering};

        struct CountingHandler {
            calls: Arc<AtomicUsize>,
        }
        impl TaskActorStepHandler for CountingHandler {
            fn execute<'a>(
                &'a self,
                admitted: AdmittedMarlinDispatch,
            ) -> Pin<Box<dyn Future<Output = MarlinExecutionReceipt> + Send + 'a>> {
                let calls = Arc::clone(&self.calls);
                Box::pin(async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    let env = admitted.envelope();
                    MarlinExecutionReceipt {
                        request_id: env.request_id.clone(),
                        task_id: env.task_id.clone(),
                        fencing_epoch: env.fencing_epoch,
                        status: ExecutionReceiptStatus::Ok,
                    }
                })
            }
        }

        let registry = TaskActorRuntimeRegistry::new();
        let task_id = TaskActorId::new("task-sm25-estop").unwrap();
        registry
            .register_actor(
                task_id.clone(),
                "SM25 Governed Graph Task",
                TaskActorScopeMode::Project,
            )
            .unwrap();

        let breaker = Arc::new(ExecutionCircuitBreaker::new(ToolBudgetUnits::new(8)));
        let gate = Arc::new(GovernanceAdmissionGate::new(breaker));
        gate.trip_operator_estop("operator-alice", "sm25-task-actor-estop")
            .expect("trip estop");

        let calls = Arc::new(AtomicUsize::new(0));
        let executor = TaskActorKernelExecutor::new(
            registry.clone(),
            Arc::new(CountingHandler {
                calls: Arc::clone(&calls),
            }),
        )
        .with_governance_gate(
            Arc::clone(&gate),
            GovernedSubjectName::new("agent-sm25").unwrap(),
            None,
        );

        let result = executor
            .dispatch_and_reconcile(MarlinDispatchEnvelope {
                request_id: DispatchRequestId::new("req-sm25-1").unwrap(),
                task_id: task_id.clone(),
                operation: GraphDispatchOperation::ExecuteFrontier,
                fencing_epoch: FencingEpoch::new(1).unwrap(),
                scope_mode: TaskActorScopeMode::Project,
                join_node_id: None,
                substrate_target: GraphSubstrateTarget::SqliteTurso,
            })
            .await;

        assert!(
            matches!(result, Err(TaskActorRuntimeError::GovernanceDenied { .. }))
                && calls.load(Ordering::SeqCst) == 0,
            "ADV-L4 DISCONNECTED GOVERNANCE BYPASS PROVEN: TaskActorKernelExecutor executed step handler (calls={}) despite active E-Stop! result={result:?}",
            calls.load(Ordering::SeqCst)
        );

        // 2. Verified human re-arm + Cedar forbid policy STILL blocks TaskActor dispatch fail-closed!
        use crate::{
            BreakerActor, BreakerActorId, CedarPolicyId, GOVERNANCE_ADMISSION_SCHEMA_VERSION,
            GovernanceProofDigest, GovernanceProofEnvelope, GovernanceProofEpoch,
            SynthesizedCedarPolicy,
        };
        let human = BreakerActor::VerifiedHuman(BreakerActorId::new("operator-alice").unwrap());
        gate.rearm_by_verified_human(human).expect("human rearm");
        let digest = GovernanceProofDigest::new(format!("sha256:{}", "a".repeat(64))).unwrap();
        gate.synchronize_proof_envelope(&GovernanceProofEnvelope {
            schema_version: GOVERNANCE_ADMISSION_SCHEMA_VERSION,
            admitted: true,
            revocation_epoch: GovernanceProofEpoch::new(2).unwrap(),
            policy_revision: 1,
            assessment_digest: digest.clone(),
            taint_proof_digest: digest.clone(),
            fixpoint_proof_digest: digest.clone(),
            temporal_proof_digest: digest.clone(),
            invalidation_digest: digest.clone(),
            rejection_reasons: Vec::new(),
            fenced_actors: Vec::new(),
            blocked_tools: Vec::new(),
            retracted_conclusions: Vec::new(),
            cedar_policies: vec![SynthesizedCedarPolicy {
                policy_id: CedarPolicyId::new("forbid-agent-sm25-task").unwrap(),
                source: r#"forbid(principal == Marlin::Actor::"agent-sm25", action, resource);"#
                    .to_owned(),
            }],
        })
        .expect("sync cedar forbid");

        let cedar_denied = executor
            .dispatch_and_reconcile(MarlinDispatchEnvelope {
                request_id: DispatchRequestId::new("req-sm25-2").unwrap(),
                task_id: task_id.clone(),
                operation: GraphDispatchOperation::ExecuteFrontier,
                fencing_epoch: FencingEpoch::new(1).unwrap(),
                scope_mode: TaskActorScopeMode::Project,
                join_node_id: None,
                substrate_target: GraphSubstrateTarget::SqliteTurso,
            })
            .await;
        assert!(matches!(
            cedar_denied,
            Err(TaskActorRuntimeError::GovernanceDenied { .. })
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 0);

        // 3. Clearing Cedar forbid via fresh epoch allows governed dispatch and advances epoch 1 -> 2.
        gate.synchronize_proof_envelope(&GovernanceProofEnvelope {
            schema_version: GOVERNANCE_ADMISSION_SCHEMA_VERSION,
            admitted: true,
            revocation_epoch: GovernanceProofEpoch::new(3).unwrap(),
            policy_revision: 2,
            assessment_digest: digest.clone(),
            taint_proof_digest: digest.clone(),
            fixpoint_proof_digest: digest.clone(),
            temporal_proof_digest: digest.clone(),
            invalidation_digest: digest,
            rejection_reasons: Vec::new(),
            fenced_actors: Vec::new(),
            blocked_tools: Vec::new(),
            retracted_conclusions: Vec::new(),
            cedar_policies: Vec::new(),
        })
        .expect("sync clean envelope");

        let (_admitted, decision) = executor
            .dispatch_and_reconcile(MarlinDispatchEnvelope {
                request_id: DispatchRequestId::new("req-sm25-3").unwrap(),
                task_id,
                operation: GraphDispatchOperation::ExecuteFrontier,
                fencing_epoch: FencingEpoch::new(1).unwrap(),
                scope_mode: TaskActorScopeMode::Project,
                join_node_id: None,
                substrate_target: GraphSubstrateTarget::SqliteTurso,
            })
            .await
            .expect("clean governed dispatch must succeed");
        assert!(decision.accepted);
        assert_eq!(decision.next_epoch.value(), 2);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}
