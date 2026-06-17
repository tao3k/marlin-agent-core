//! Agent execution scope values admitted by `model_route` policy.

use serde::{Deserialize, Serialize};

/// Agent execution scope used by model routing.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum ModelRouteAgentScope {
    Any,
    RootAgent,
    SubAgent,
    CustomAgent,
    CustomerAgent,
    ForkedAgent,
    IsolatedAgent,
    PersistentAgent,
}

impl ModelRouteAgentScope {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Any => "Any",
            Self::RootAgent => "RootAgent",
            Self::SubAgent => "SubAgent",
            Self::CustomAgent => "CustomAgent",
            Self::CustomerAgent => "CustomerAgent",
            Self::ForkedAgent => "ForkedAgent",
            Self::IsolatedAgent => "IsolatedAgent",
            Self::PersistentAgent => "PersistentAgent",
        }
    }
}
