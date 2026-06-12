//! Runtime receipt for configured sub-agent spawn profiles.

use marlin_agent_protocol::{SubAgentSpawnConfig, SubAgentSpawnProfile};
use marlin_agent_sessions::{SessionId, SessionIsolationReceipt};

/// Runtime receipt for a configured sub-agent spawn profile.
#[derive(Clone, Debug)]
pub struct SubAgentSpawnReceipt {
    config: SubAgentSpawnConfig,
    isolation_receipt: SessionIsolationReceipt,
}

impl SubAgentSpawnReceipt {
    pub fn new(config: SubAgentSpawnConfig, isolation_receipt: SessionIsolationReceipt) -> Self {
        Self {
            config,
            isolation_receipt,
        }
    }

    pub fn config(&self) -> &SubAgentSpawnConfig {
        &self.config
    }

    pub fn isolation_receipt(&self) -> &SessionIsolationReceipt {
        &self.isolation_receipt
    }

    pub fn profile_id(&self) -> &str {
        self.config.profile_id.as_str()
    }

    pub fn agent_type(&self) -> &str {
        self.config.agent_type.as_str()
    }

    pub fn role(&self) -> &str {
        self.config.role.as_str()
    }

    pub fn nickname(&self) -> Option<&str> {
        self.config.nickname.as_deref()
    }

    pub fn child_session_id(&self) -> &SessionId {
        self.isolation_receipt.child_session_id()
    }

    pub fn activity_profile(&self) -> SubAgentSpawnProfile {
        SubAgentSpawnProfile::from_config(&self.config)
    }
}
