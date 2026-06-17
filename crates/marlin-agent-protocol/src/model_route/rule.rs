//! Deterministic route rule contracts for `model_route` configuration.

use serde::{Deserialize, Serialize};

use super::{
    ModelCommandMatcher, ModelEndpoint, ModelEndpointContractError, ModelRouteRuleId,
    ModelSessionPolicy,
};

/// One deterministic model route rule.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ModelRouteRule {
    pub rule_id: ModelRouteRuleId,
    pub priority: i16,
    pub matcher: ModelCommandMatcher,
    pub endpoint: ModelEndpoint,
    #[serde(default)]
    pub session: ModelSessionPolicy,
}

impl ModelRouteRule {
    pub fn new(
        rule_id: impl Into<ModelRouteRuleId>,
        priority: i16,
        matcher: ModelCommandMatcher,
        endpoint: ModelEndpoint,
    ) -> Self {
        Self {
            rule_id: rule_id.into(),
            priority,
            matcher,
            endpoint,
            session: ModelSessionPolicy::default(),
        }
    }

    pub fn with_session(mut self, session: ModelSessionPolicy) -> Self {
        self.session = session;
        self
    }

    pub fn validate_endpoint_contract(&self) -> Result<(), ModelEndpointContractError> {
        self.endpoint.validate_contract()
    }
}
