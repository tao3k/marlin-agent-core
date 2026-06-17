//! Decision receipts emitted by deterministic `model_route` resolution.

use serde::{Deserialize, Serialize};

use crate::RuntimeEnvironmentActivationReceipt;

use super::{
    LiteLlmModelId, ModelContextForkMode, ModelEndpoint, ModelRouteAgentScope, ModelRouteRuleId,
    ModelRouteSessionId, ModelSessionLifecycle, ModelSessionPolicy,
};

/// Audit record emitted by model route resolution.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelRouteReceipt {
    pub rule_id: ModelRouteRuleId,
    pub matched_globs: Vec<String>,
    pub command_line: String,
    pub litellm_model_id: LiteLlmModelId,
    pub session_lifecycle: ModelSessionLifecycle,
    pub context_fork: ModelContextForkMode,
    pub requested_session_id: Option<ModelRouteSessionId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_scope: Option<ModelRouteAgentScope>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub environment_activation: Option<RuntimeEnvironmentActivationReceipt>,
    pub fallback_reason: Option<String>,
}

/// Resolved model route and its receipt.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ModelRouteDecision {
    pub endpoint: ModelEndpoint,
    pub session: ModelSessionPolicy,
    pub receipt: ModelRouteReceipt,
}
