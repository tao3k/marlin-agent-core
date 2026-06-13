//! Typed model-route selection projection errors.

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

use marlin_agent_protocol::{LiteLlmModelId, ModelRouteDecision, ModelRouteRuleId};
use serde::{Deserialize, Serialize};

/// Runtime selector surface that produced a selected policy index.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ModelRouteSelectionProjectionSource {
    NativeAbiPolicyIndex,
    SchemeCompiledPolicyIndex,
}

/// Receipt proving how a typed selector result projected back into Rust routing.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ModelRouteSelectionProjectionReceipt {
    pub source: ModelRouteSelectionProjectionSource,
    pub policy_index: usize,
    pub rule_id: ModelRouteRuleId,
    pub matched_globs: Vec<String>,
    pub litellm_model_id: LiteLlmModelId,
}

impl ModelRouteSelectionProjectionReceipt {
    pub(crate) fn new(
        source: ModelRouteSelectionProjectionSource,
        policy_index: usize,
        decision: &ModelRouteDecision,
    ) -> Self {
        Self {
            source,
            policy_index,
            rule_id: decision.receipt.rule_id.clone(),
            matched_globs: decision.receipt.matched_globs.clone(),
            litellm_model_id: decision.receipt.litellm_model_id.clone(),
        }
    }
}

/// Model route decision plus the selector projection receipt that produced it.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProjectedModelRouteDecision {
    pub decision: ModelRouteDecision,
    pub projection: ModelRouteSelectionProjectionReceipt,
}

/// Error raised when an external typed selector chooses a route policy index
/// that cannot be projected into a runtime decision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModelRouteSelectionProjectionError {
    /// The selector returned an index outside the original route policy array.
    UnknownPolicyIndex { policy_index: usize },
    /// The selected rule no longer matches the request facts Rust is projecting.
    SelectedRuleDidNotMatch { policy_index: usize },
}

impl Display for ModelRouteSelectionProjectionError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownPolicyIndex { policy_index } => {
                write!(
                    formatter,
                    "model route policy index {policy_index} does not exist"
                )
            }
            Self::SelectedRuleDidNotMatch { policy_index } => write!(
                formatter,
                "model route policy index {policy_index} did not match the request"
            ),
        }
    }
}

impl Error for ModelRouteSelectionProjectionError {}
