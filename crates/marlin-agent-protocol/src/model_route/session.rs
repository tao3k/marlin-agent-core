//! Session lifecycle and context policy types for `model_route` decisions.

use serde::{Deserialize, Serialize};

use super::{ModelRouteSessionId, ModelSessionPersistenceKey, ModelSessionPoolId};

/// Context semantics used when deriving or reusing a sub-agent session.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ModelContextForkMode {
    ForkSnapshot,
    SharedLive,
    Minimal,
    Isolated,
}

/// Session lifecycle selected for the routed model call.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ModelSessionLifecycle {
    Ephemeral,
    Persistent { key: ModelSessionPersistenceKey },
    Pooled { pool: ModelSessionPoolId },
}

/// Session policy attached to a route rule.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ModelSessionPolicy {
    pub context: ModelContextForkMode,
    pub lifecycle: ModelSessionLifecycle,
    pub requested_session_id: Option<ModelRouteSessionId>,
}

impl ModelSessionPolicy {
    pub fn ephemeral(context: ModelContextForkMode) -> Self {
        Self {
            context,
            lifecycle: ModelSessionLifecycle::Ephemeral,
            requested_session_id: None,
        }
    }

    pub fn persistent(
        key: impl Into<ModelSessionPersistenceKey>,
        context: ModelContextForkMode,
    ) -> Self {
        Self {
            context,
            lifecycle: ModelSessionLifecycle::Persistent { key: key.into() },
            requested_session_id: None,
        }
    }

    pub fn pooled(pool: impl Into<ModelSessionPoolId>, context: ModelContextForkMode) -> Self {
        Self {
            context,
            lifecycle: ModelSessionLifecycle::Pooled { pool: pool.into() },
            requested_session_id: None,
        }
    }

    pub fn with_requested_session_id(mut self, session_id: impl Into<ModelRouteSessionId>) -> Self {
        self.requested_session_id = Some(session_id.into());
        self
    }
}

impl Default for ModelSessionPolicy {
    fn default() -> Self {
        Self::ephemeral(ModelContextForkMode::ForkSnapshot)
    }
}
