//! Configurable graph-loop policy profile contracts.

use serde::{Deserialize, Serialize};

use super::loop_event::{GraphLoopInputDrainPolicy, GraphToolBatchExecutionMode};

/// Stable identifier for a configurable graph-loop policy profile.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LoopPolicyProfileId(String);

impl LoopPolicyProfileId {
    /// Creates a loop policy profile identifier.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns the profile identifier as text.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Converts the profile identifier into its owned string.
    pub fn into_string(self) -> String {
        self.0
    }
}

impl From<String> for LoopPolicyProfileId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for LoopPolicyProfileId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// Typed allow/deny capability flag for continuation action classes.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LoopContinuationCapability(bool);

impl LoopContinuationCapability {
    /// Creates an enabled continuation capability.
    pub fn enabled() -> Self {
        Self(true)
    }

    /// Creates a disabled continuation capability.
    pub fn disabled() -> Self {
        Self(false)
    }

    /// Returns this capability as a boolean.
    pub fn as_bool(self) -> bool {
        self.0
    }
}

impl From<bool> for LoopContinuationCapability {
    fn from(value: bool) -> Self {
        Self(value)
    }
}

/// Typed policy profile for configuring a graph-loop controller without embedding runtime state.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LoopPolicyProfile {
    pub profile_id: LoopPolicyProfileId,
    pub queue_policy: LoopQueuePolicy,
    pub continuation_policy: LoopContinuationPolicy,
    pub tool_batch_policy: LoopToolBatchPolicy,
    pub model_route_policy: LoopModelRoutePolicy,
    pub evidence_policy: LoopEvidenceCapturePolicy,
    pub failure_policy: LoopFailurePolicy,
    pub memory_policy: LoopMemoryPolicy,
    pub human_gate_policy: LoopHumanGatePolicy,
    pub self_evolution_policy: LoopSelfEvolutionPolicy,
}

impl LoopPolicyProfile {
    /// Creates a conservative configurable graph-loop policy profile.
    pub fn new(profile_id: impl Into<LoopPolicyProfileId>) -> Self {
        Self {
            profile_id: profile_id.into(),
            queue_policy: LoopQueuePolicy::default(),
            continuation_policy: LoopContinuationPolicy::default(),
            tool_batch_policy: LoopToolBatchPolicy::default(),
            model_route_policy: LoopModelRoutePolicy::default(),
            evidence_policy: LoopEvidenceCapturePolicy::default(),
            failure_policy: LoopFailurePolicy::default(),
            memory_policy: LoopMemoryPolicy::default(),
            human_gate_policy: LoopHumanGatePolicy::default(),
            self_evolution_policy: LoopSelfEvolutionPolicy::default(),
        }
    }

    /// Sets the queue policy surface.
    pub fn with_queue_policy(mut self, queue_policy: LoopQueuePolicy) -> Self {
        self.queue_policy = queue_policy;
        self
    }

    /// Sets the continuation policy surface.
    pub fn with_continuation_policy(mut self, continuation_policy: LoopContinuationPolicy) -> Self {
        self.continuation_policy = continuation_policy;
        self
    }

    /// Sets the tool-batch policy surface.
    pub fn with_tool_batch_policy(mut self, tool_batch_policy: LoopToolBatchPolicy) -> Self {
        self.tool_batch_policy = tool_batch_policy;
        self
    }

    /// Sets the model-route policy surface.
    pub fn with_model_route_policy(mut self, model_route_policy: LoopModelRoutePolicy) -> Self {
        self.model_route_policy = model_route_policy;
        self
    }

    /// Sets the evidence capture policy surface.
    pub fn with_evidence_policy(mut self, evidence_policy: LoopEvidenceCapturePolicy) -> Self {
        self.evidence_policy = evidence_policy;
        self
    }

    /// Sets the failure policy surface.
    pub fn with_failure_policy(mut self, failure_policy: LoopFailurePolicy) -> Self {
        self.failure_policy = failure_policy;
        self
    }

    /// Sets the memory policy surface.
    pub fn with_memory_policy(mut self, memory_policy: LoopMemoryPolicy) -> Self {
        self.memory_policy = memory_policy;
        self
    }

    /// Sets the human gate policy surface.
    pub fn with_human_gate_policy(mut self, human_gate_policy: LoopHumanGatePolicy) -> Self {
        self.human_gate_policy = human_gate_policy;
        self
    }

    /// Sets the self-evolution policy surface.
    pub fn with_self_evolution_policy(
        mut self,
        self_evolution_policy: LoopSelfEvolutionPolicy,
    ) -> Self {
        self.self_evolution_policy = self_evolution_policy;
        self
    }
}

/// Queue policy for steering and follow-up input lanes.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LoopQueuePolicy {
    pub steering_drain_policy: GraphLoopInputDrainPolicy,
    pub follow_up_drain_policy: GraphLoopInputDrainPolicy,
    #[serde(default)]
    pub prioritize_steering: bool,
}

impl Default for LoopQueuePolicy {
    fn default() -> Self {
        Self {
            steering_drain_policy: GraphLoopInputDrainPolicy::DrainOne,
            follow_up_drain_policy: GraphLoopInputDrainPolicy::DrainOne,
            prioritize_steering: true,
        }
    }
}

/// Continuation policy capabilities exposed to typed strategy planes.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LoopContinuationPolicy {
    pub allow_accept: LoopContinuationCapability,
    pub allow_deny: LoopContinuationCapability,
    pub allow_defer: LoopContinuationCapability,
    pub allow_rewrite: LoopContinuationCapability,
    #[serde(default)]
    pub require_decision_receipt: bool,
}

impl Default for LoopContinuationPolicy {
    fn default() -> Self {
        Self {
            allow_accept: LoopContinuationCapability::enabled(),
            allow_deny: LoopContinuationCapability::enabled(),
            allow_defer: LoopContinuationCapability::enabled(),
            allow_rewrite: LoopContinuationCapability::enabled(),
            require_decision_receipt: true,
        }
    }
}

/// Tool batch policy for a graph node frontier.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LoopToolBatchPolicy {
    pub execution_mode: GraphToolBatchExecutionMode,
    #[serde(default)]
    pub force_sequential: bool,
    #[serde(default)]
    pub require_all_tools_to_terminate: bool,
    #[serde(default)]
    pub require_before_after_hook_receipts: bool,
}

impl Default for LoopToolBatchPolicy {
    fn default() -> Self {
        Self {
            execution_mode: GraphToolBatchExecutionMode::Sequential,
            force_sequential: true,
            require_all_tools_to_terminate: true,
            require_before_after_hook_receipts: true,
        }
    }
}

/// Model route policy for provider request projection.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LoopModelRoutePolicy {
    #[serde(default)]
    pub allow_model_override: bool,
    #[serde(default)]
    pub require_route_receipt: bool,
    #[serde(default)]
    pub require_no_live_llm_receipt: bool,
    #[serde(default)]
    pub allow_context_transform: bool,
}

impl Default for LoopModelRoutePolicy {
    fn default() -> Self {
        Self {
            allow_model_override: false,
            require_route_receipt: true,
            require_no_live_llm_receipt: false,
            allow_context_transform: true,
        }
    }
}

/// Evidence capture policy for configurable graph-loop replay.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LoopEvidenceCapturePolicy {
    #[serde(default)]
    pub capture_events: bool,
    #[serde(default)]
    pub capture_node_receipts: bool,
    #[serde(default)]
    pub capture_tool_receipts: bool,
    #[serde(default)]
    pub capture_content_receipts: bool,
    #[serde(default)]
    pub capture_trace: bool,
    #[serde(default)]
    pub replayable: bool,
}

impl Default for LoopEvidenceCapturePolicy {
    fn default() -> Self {
        Self {
            capture_events: true,
            capture_node_receipts: true,
            capture_tool_receipts: true,
            capture_content_receipts: true,
            capture_trace: true,
            replayable: true,
        }
    }
}

/// Failure policy for retry, repair, and human escalation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LoopFailurePolicy {
    #[serde(default)]
    pub classify_failure: bool,
    #[serde(default)]
    pub allow_retry: bool,
    #[serde(default)]
    pub allow_repair_graph: bool,
    #[serde(default)]
    pub escalate_unknown_to_human: bool,
}

impl Default for LoopFailurePolicy {
    fn default() -> Self {
        Self {
            classify_failure: true,
            allow_retry: true,
            allow_repair_graph: true,
            escalate_unknown_to_human: true,
        }
    }
}

/// Memory policy for project-scoped promotion and retention.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LoopMemoryPolicy {
    #[serde(default)]
    pub allow_project_promotion: bool,
    #[serde(default)]
    pub require_contract_validated: bool,
    #[serde(default)]
    pub allow_cross_project_memory: bool,
    #[serde(default)]
    pub require_source_anchor: bool,
}

impl Default for LoopMemoryPolicy {
    fn default() -> Self {
        Self {
            allow_project_promotion: false,
            require_contract_validated: true,
            allow_cross_project_memory: false,
            require_source_anchor: true,
        }
    }
}

/// Human gate policy for sensitive graph-loop transitions.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LoopHumanGatePolicy {
    #[serde(default)]
    pub require_for_permission_escalation: bool,
    #[serde(default)]
    pub require_for_policy_change: bool,
    #[serde(default)]
    pub require_for_cross_project_memory: bool,
    #[serde(default)]
    pub require_for_unverified_root_cause: bool,
}

impl Default for LoopHumanGatePolicy {
    fn default() -> Self {
        Self {
            require_for_permission_escalation: true,
            require_for_policy_change: true,
            require_for_cross_project_memory: true,
            require_for_unverified_root_cause: true,
        }
    }
}

/// Self-evolution policy for causal improvement loops.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LoopSelfEvolutionPolicy {
    #[serde(default)]
    pub require_failure_observation_receipt: bool,
    #[serde(default)]
    pub require_root_cause_receipt: bool,
    #[serde(default)]
    pub require_intervention_receipt: bool,
    #[serde(default)]
    pub require_progress_receipt: bool,
    #[serde(default)]
    pub allow_policy_update: bool,
}

impl Default for LoopSelfEvolutionPolicy {
    fn default() -> Self {
        Self {
            require_failure_observation_receipt: true,
            require_root_cause_receipt: true,
            require_intervention_receipt: true,
            require_progress_receipt: true,
            allow_policy_update: false,
        }
    }
}
