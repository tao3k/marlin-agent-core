//! Runtime policy-combination experiment receipts for `LoopProgram`.

use std::fmt::Debug;

use marlin_agent_protocol::{
    AgentFlowIntent, AgentFlowMemoryOperation, LoopMechanismPolicyId, LoopProgram,
    LoopProgramActionKind, LoopProgramEventKind, LoopProgramId,
};

use crate::{
    LoopProgramExecutionReceipt, LoopProgramExecutionStatus,
    LoopProgramRuntimeHandoffExecutionStatus, LoopProgramRuntimeOwner,
};

macro_rules! define_policy_matrix_string_id {
    ($type_name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $type_name(String);

        impl $type_name {
            /// Creates a new typed string value.
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            /// Returns the inner string slice.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl From<&str> for $type_name {
            fn from(value: &str) -> Self {
                Self::new(value)
            }
        }

        impl From<String> for $type_name {
            fn from(value: String) -> Self {
                Self::new(value)
            }
        }
    };
}

define_policy_matrix_string_id!(
    RuntimePolicyExperimentCaseId,
    "Stable id for one runtime policy-combination experiment."
);
define_policy_matrix_string_id!(
    RuntimePolicyExperimentDigest,
    "Stable digest emitted by runtime policy-combination evidence."
);
define_policy_matrix_string_id!(
    RuntimePolicyRecommendationTarget,
    "Stable target for a runtime policy improvement recommendation."
);
define_policy_matrix_string_id!(
    RuntimePolicyRecommendationEvidence,
    "Evidence summary for a runtime policy improvement recommendation."
);
define_policy_matrix_string_id!(
    RuntimePolicyRecommendationAction,
    "Action summary for a runtime policy improvement recommendation."
);

/// Count of occurrences observed in a runtime policy-combination experiment.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RuntimePolicyExperimentCount(usize);

impl RuntimePolicyExperimentCount {
    /// Creates a typed count.
    pub fn new(value: usize) -> Self {
        Self(value)
    }

    /// Returns the raw count.
    pub fn get(self) -> usize {
        self.0
    }
}

/// Priority attached to a runtime policy improvement recommendation.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RuntimePolicyRecommendationPriority {
    P0,
    P1,
    P2,
}

impl RuntimePolicyRecommendationPriority {
    /// Stable priority label used in artifact summaries.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::P0 => "P0",
            Self::P1 => "P1",
            Self::P2 => "P2",
        }
    }
}

/// One actionable improvement derived from a policy experiment receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimePolicyImprovementRecommendation {
    pub priority: RuntimePolicyRecommendationPriority,
    pub target: RuntimePolicyRecommendationTarget,
    pub evidence: RuntimePolicyRecommendationEvidence,
    pub action: RuntimePolicyRecommendationAction,
}

impl RuntimePolicyImprovementRecommendation {
    /// Creates one typed improvement recommendation.
    pub fn new(
        priority: RuntimePolicyRecommendationPriority,
        target: impl Into<RuntimePolicyRecommendationTarget>,
        evidence: impl Into<RuntimePolicyRecommendationEvidence>,
        action: impl Into<RuntimePolicyRecommendationAction>,
    ) -> Self {
        Self {
            priority,
            target: target.into(),
            evidence: evidence.into(),
            action: action.into(),
        }
    }
}

/// Typed receipt for one runtime policy-combination experiment.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimePolicyExperimentReceipt {
    pub case_id: RuntimePolicyExperimentCaseId,
    pub program_id: LoopProgramId,
    pub policy_ids: Box<[LoopMechanismPolicyId]>,
    pub policy_digest: RuntimePolicyExperimentDigest,
    pub loop_program_digest: RuntimePolicyExperimentDigest,
    pub runtime_behavior_digest: RuntimePolicyExperimentDigest,
    pub receipt_digest: RuntimePolicyExperimentDigest,
    pub status: LoopProgramExecutionStatus,
    pub action_path: Box<[LoopProgramActionKind]>,
    pub owner_path: Box<[LoopProgramRuntimeOwner]>,
    pub generated_events: Box<[Option<LoopProgramEventKind>]>,
    pub denied_handoff_count: RuntimePolicyExperimentCount,
    pub handled_handoff_count: RuntimePolicyExperimentCount,
    pub agent_flow_intent_count: RuntimePolicyExperimentCount,
    pub tool_projection_count: RuntimePolicyExperimentCount,
    pub memory_projection_count: RuntimePolicyExperimentCount,
    pub improvement_recommendations: Box<[RuntimePolicyImprovementRecommendation]>,
}

/// Projects a `LoopProgram` execution into a runtime policy-combination receipt.
pub fn runtime_policy_experiment_receipt(
    case_id: impl Into<RuntimePolicyExperimentCaseId>,
    loop_program: &LoopProgram,
    execution_receipt: &LoopProgramExecutionReceipt,
) -> RuntimePolicyExperimentReceipt {
    let mut receipt = RuntimePolicyExperimentReceipt {
        case_id: case_id.into(),
        program_id: execution_receipt.program_id.clone(),
        policy_ids: loop_program.mechanism_policies.clone(),
        policy_digest: RuntimePolicyExperimentDigest::new(hex_bytes(
            loop_program.policy_digest.as_bytes(),
        )),
        loop_program_digest: RuntimePolicyExperimentDigest::new(loop_program_digest(loop_program)),
        runtime_behavior_digest: RuntimePolicyExperimentDigest::new(runtime_behavior_digest(
            execution_receipt,
        )),
        receipt_digest: RuntimePolicyExperimentDigest::new("pending"),
        status: execution_receipt.status.clone(),
        action_path: execution_receipt
            .steps
            .iter()
            .map(|step| step.machine_receipt.action.clone())
            .collect::<Vec<_>>()
            .into_boxed_slice(),
        owner_path: execution_receipt
            .steps
            .iter()
            .flat_map(|step| {
                step.runtime_handoff_execution
                    .executions
                    .iter()
                    .map(|execution| execution.owner.clone())
            })
            .collect::<Vec<_>>()
            .into_boxed_slice(),
        generated_events: execution_receipt
            .steps
            .iter()
            .map(|step| step.generated_event.clone())
            .collect::<Vec<_>>()
            .into_boxed_slice(),
        denied_handoff_count: RuntimePolicyExperimentCount::new(count_executions(
            execution_receipt,
            LoopProgramRuntimeHandoffExecutionStatus::Denied,
        )),
        handled_handoff_count: RuntimePolicyExperimentCount::new(count_executions(
            execution_receipt,
            LoopProgramRuntimeHandoffExecutionStatus::Handled,
        )),
        agent_flow_intent_count: RuntimePolicyExperimentCount::new(count_agent_flow_intents(
            execution_receipt,
        )),
        tool_projection_count: RuntimePolicyExperimentCount::new(count_tool_projections(
            execution_receipt,
        )),
        memory_projection_count: RuntimePolicyExperimentCount::new(count_memory_projections(
            execution_receipt,
        )),
        improvement_recommendations: Box::new([]),
    };
    receipt.improvement_recommendations =
        runtime_policy_improvement_recommendations(&receipt).into_boxed_slice();
    receipt.receipt_digest =
        RuntimePolicyExperimentDigest::new(runtime_policy_receipt_digest(&receipt));
    receipt
}

fn count_executions(
    execution_receipt: &LoopProgramExecutionReceipt,
    status: LoopProgramRuntimeHandoffExecutionStatus,
) -> usize {
    execution_receipt
        .steps
        .iter()
        .flat_map(|step| step.runtime_handoff_execution.executions.iter())
        .filter(|execution| execution.status == status)
        .count()
}

fn count_agent_flow_intents(execution_receipt: &LoopProgramExecutionReceipt) -> usize {
    execution_receipt
        .steps
        .iter()
        .flat_map(|step| step.runtime_handoff_execution.executions.iter())
        .filter(|execution| execution.agent_flow_intent.is_some())
        .count()
}

fn count_tool_projections(execution_receipt: &LoopProgramExecutionReceipt) -> usize {
    execution_receipt
        .steps
        .iter()
        .map(|step| {
            step.runtime_handoff_execution
                .tool_process_projections
                .len()
        })
        .sum()
}

fn count_memory_projections(execution_receipt: &LoopProgramExecutionReceipt) -> usize {
    execution_receipt
        .steps
        .iter()
        .map(|step| step.runtime_handoff_execution.memory_projections.len())
        .sum()
}

fn loop_program_digest(loop_program: &LoopProgram) -> String {
    let mut digest = StableReceiptDigest::new("loop-program.v1");
    digest.write_u32(loop_program.schema_version);
    digest.write_str(loop_program.program_id.as_str());
    digest.write_debug(&loop_program.policy_epoch);
    digest.write_bytes(loop_program.policy_digest.as_bytes());
    digest.write_str(loop_program.initial_state.as_str());
    for policy in loop_program.mechanism_policies.iter() {
        digest.write_str(policy.as_str());
    }
    for transition in loop_program.transitions.iter() {
        digest.write_str(transition.transition_id.as_str());
        digest.write_str(transition.from.as_str());
        digest.write_debug(&transition.event);
        digest.write_debug(&transition.action);
        digest.write_str(transition.to.as_str());
    }
    digest.finish()
}

fn runtime_behavior_digest(execution_receipt: &LoopProgramExecutionReceipt) -> String {
    let mut digest = StableReceiptDigest::new("runtime-behavior.v1");
    digest.write_str(execution_receipt.program_id.as_str());
    digest.write_debug(&execution_receipt.status);
    for step in execution_receipt.steps.iter() {
        digest.write_debug(&step.machine_receipt.action);
        digest.write_debug(&step.generated_event);
        digest.write_debug(&step.runtime_handoff_execution.status);
        digest.write_usize(
            step.runtime_handoff_execution
                .tool_process_projections
                .len(),
        );
        digest.write_usize(step.runtime_handoff_execution.memory_projections.len());
        for execution in step.runtime_handoff_execution.executions.iter() {
            digest.write_str(execution.owner.as_str());
            digest.write_debug(&execution.status);
            digest.write_debug(&execution.kind);
            digest.write_debug(&execution.next_event);
            digest.write_bool(execution.agent_flow_intent.is_some());
        }
    }
    digest.finish()
}

fn runtime_policy_receipt_digest(receipt: &RuntimePolicyExperimentReceipt) -> String {
    let mut digest = StableReceiptDigest::new("runtime-policy-experiment.v1");
    digest.write_str(receipt.case_id.as_str());
    digest.write_str(receipt.program_id.as_str());
    digest.write_str(receipt.policy_digest.as_str());
    digest.write_str(receipt.loop_program_digest.as_str());
    digest.write_str(receipt.runtime_behavior_digest.as_str());
    digest.write_debug(&receipt.status);
    for policy in receipt.policy_ids.iter() {
        digest.write_str(policy.as_str());
    }
    for action in receipt.action_path.iter() {
        digest.write_debug(action);
    }
    for owner in receipt.owner_path.iter() {
        digest.write_str(owner.as_str());
    }
    for event in receipt.generated_events.iter() {
        digest.write_debug(event);
    }
    digest.write_usize(receipt.denied_handoff_count.get());
    digest.write_usize(receipt.handled_handoff_count.get());
    digest.write_usize(receipt.agent_flow_intent_count.get());
    digest.write_usize(receipt.tool_projection_count.get());
    digest.write_usize(receipt.memory_projection_count.get());
    for recommendation in receipt.improvement_recommendations.iter() {
        digest.write_str(recommendation.priority.as_str());
        digest.write_str(recommendation.target.as_str());
        digest.write_str(recommendation.evidence.as_str());
        digest.write_str(recommendation.action.as_str());
    }
    digest.finish()
}

fn runtime_policy_improvement_recommendations(
    receipt: &RuntimePolicyExperimentReceipt,
) -> Vec<RuntimePolicyImprovementRecommendation> {
    let mut recommendations = Vec::new();

    if receipt.denied_handoff_count.get() > 0 {
        recommendations.push(RuntimePolicyImprovementRecommendation::new(
            RuntimePolicyRecommendationPriority::P0,
            "runtime.sandbox.denylist",
            "denied handoff receipt crossed the event pump",
            "preserve deny receipt details when promoting this case into policy reflection",
        ));
    }

    if receipt
        .policy_ids
        .iter()
        .any(|policy| policy.as_str() == "runtime-memory-recall")
        && receipt.memory_projection_count.get() == 0
    {
        recommendations.push(RuntimePolicyImprovementRecommendation::new(
            RuntimePolicyRecommendationPriority::P1,
            "runtime.agent-flow.memory-projection",
            "memory recall survived as a typed intent but did not emit a memory projection receipt",
            "route the memory leg through the Agent-Flow executor before treating recall as closed",
        ));
    }

    if receipt
        .action_path
        .iter()
        .any(|action| action == &LoopProgramActionKind::DispatchTools)
        && receipt.tool_projection_count.get() == 0
        && receipt.denied_handoff_count.get() == 0
    {
        recommendations.push(RuntimePolicyImprovementRecommendation::new(
            RuntimePolicyRecommendationPriority::P1,
            "runtime.tool-sandbox.spawn",
            "tool dispatch was handled without a tool-process projection receipt",
            "promote the case to a real tool+sandbox spawn before using it as full-loop evidence",
        ));
    }

    if receipt.policy_ids.len() > 1
        && !receipt
            .policy_ids
            .iter()
            .any(|policy| policy.as_str().contains("poo"))
    {
        recommendations.push(RuntimePolicyImprovementRecommendation::new(
            RuntimePolicyRecommendationPriority::P2,
            "gerbil.config-interface.policy-pack",
            "policy combination is still assembled as a Rust test fixture",
            "derive the same LoopProgram from a Gerbil POO profile and resolved policy pack",
        ));
    }

    recommendations
}

/// Returns true when a handoff receipt preserved a memory recall intent.
pub fn has_memory_recall_intent(
    runtime_handoff_execution: &crate::LoopProgramRuntimeHandoffExecutionReceipt,
) -> bool {
    runtime_handoff_execution
        .executions
        .iter()
        .filter_map(|execution| execution.agent_flow_intent.as_ref())
        .any(|intent| {
            matches!(
                intent,
                AgentFlowIntent::Memory(memory_intent)
                    if memory_intent.operation == AgentFlowMemoryOperation::Recall
            )
        })
}

struct StableReceiptDigest {
    value: u64,
}

impl StableReceiptDigest {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x00000100000001b3;

    fn new(scope: &'static str) -> Self {
        let mut digest = Self {
            value: Self::FNV_OFFSET,
        };
        digest.write_str(scope);
        digest
    }

    fn write_bool(&mut self, value: bool) {
        self.write_str(if value { "true" } else { "false" });
    }

    fn write_u32(&mut self, value: u32) {
        self.write_str(&value.to_string());
    }

    fn write_usize(&mut self, value: usize) {
        self.write_str(&value.to_string());
    }

    fn write_debug(&mut self, value: &impl Debug) {
        self.write_str(&format!("{value:?}"));
    }

    fn write_str(&mut self, value: &str) {
        self.write_bytes(value.as_bytes());
        self.write_bytes(&[0]);
    }

    fn write_bytes(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.value ^= u64::from(*byte);
            self.value = self.value.wrapping_mul(Self::FNV_PRIME);
        }
    }

    fn finish(self) -> String {
        format!("fnv1a64:{:016x}", self.value)
    }
}

fn hex_bytes(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}
