//! Sub-agent source and activity protocol contracts.

use serde::{Deserialize, Serialize};

use crate::{HookAgentScope, RunId};

/// Source that caused a sub-agent to run.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SubAgentSource {
    Review,
    Compact,
    ThreadSpawn {
        parent_run_id: Option<RunId>,
        depth: u32,
        agent_path: Option<String>,
        agent_nickname: Option<String>,
        agent_role: Option<String>,
    },
    MemoryConsolidation,
    Other(String),
}

/// Activity state emitted for a sub-agent.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum SubAgentActivityKind {
    Started,
    Interacted,
    Interrupted,
    Stopped,
}

/// Typed activity notification for a sub-agent runtime.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SubAgentActivity {
    pub agent_reference: String,
    pub source: SubAgentSource,
    pub kind: SubAgentActivityKind,
    pub status_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spawn_profile: Option<SubAgentSpawnProfile>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_receipt: Option<SubAgentSearchReceipt>,
}

/// Compact custom-agent profile metadata carried by spawn and activity receipts.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SubAgentSpawnProfile {
    /// Stable profile identifier, such as a custom-agent config name.
    pub profile_id: String,
    /// Runtime agent type selected for this sub-agent profile.
    pub agent_type: SubAgentType,
    /// Role used for routing, policy, and human-readable activity views.
    pub role: String,
    /// Optional display nickname assigned by the agent runtime.
    pub nickname: Option<String>,
    /// Hook agent scope assigned to this sub-agent at runtime.
    pub hook_agent_scope: HookAgentScope,
}

/// Compact receipt emitted by ASP-style sub-agent search audits.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SubAgentSearchReceipt {
    pub role: String,
    pub action: String,
    pub evidence: Vec<String>,
    pub missing: Option<String>,
    pub next: Option<String>,
    pub risk: Option<String>,
}

/// Serializable source format for a sub-agent spawn profile.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum SubAgentConfigSurface {
    #[default]
    Toml,
    Scheme,
    Other(String),
}

/// Optional strategy extension used after declarative spawn policy is fixed.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum SubAgentSpawnStrategy {
    #[default]
    Declarative,
    Scheme {
        module: String,
        procedure: Option<String>,
        aot: bool,
    },
}

/// Context visibility requested by a sub-agent profile.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum SubAgentContextVisibility {
    System,
    User,
    Workspace,
    Memory,
}

/// Context visibility requested when deriving a child session for a sub-agent.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubAgentContextPolicy {
    pub session_id: Option<String>,
    #[serde(default)]
    pub visibility: Vec<SubAgentContextVisibility>,
    pub max_history_items: Option<usize>,
}

/// Capability policy for one sub-agent profile.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SubAgentPermissionSet {
    pub read_only: bool,
    pub workspace_write: bool,
    pub network_access: bool,
    pub process_spawn: bool,
    pub descendant_spawn: bool,
    #[serde(default)]
    pub tool_access: bool,
    #[serde(default)]
    pub hook_access: bool,
    #[serde(default)]
    pub secret_access: bool,
}

/// Bounded runtime budget for high-performance sub-agent spawning.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SubAgentPerformanceBudget {
    pub max_concurrency: Option<u16>,
    pub timeout_ms: Option<u64>,
    pub token_budget: Option<u32>,
    pub max_depth: Option<u32>,
}

/// Declarative policy compiled from TOML, with optional strategy metadata.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SubAgentSpawnPolicy {
    pub permissions: SubAgentPermissionSet,
    pub context: SubAgentContextPolicy,
    pub performance: SubAgentPerformanceBudget,
}

/// Top-level configurable profile for a sub-agent spawn target.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SubAgentSpawnConfig {
    pub profile_id: String,
    pub agent_type: SubAgentType,
    pub role: String,
    pub nickname: Option<String>,
    pub hook_agent_scope: HookAgentScope,
    pub surface: SubAgentConfigSurface,
    pub strategy: SubAgentSpawnStrategy,
    pub policy: SubAgentSpawnPolicy,
}

/// Typed sub-agent runtime type identifier.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SubAgentType(String);

impl SubAgentActivity {
    pub fn new(
        agent_reference: impl Into<String>,
        source: SubAgentSource,
        kind: SubAgentActivityKind,
    ) -> Self {
        Self {
            agent_reference: agent_reference.into(),
            source,
            kind,
            status_message: None,
            spawn_profile: None,
            search_receipt: None,
        }
    }

    pub fn with_status_message(mut self, status_message: impl Into<String>) -> Self {
        self.status_message = Some(status_message.into());
        self
    }

    pub fn with_spawn_profile(mut self, profile: SubAgentSpawnProfile) -> Self {
        self.spawn_profile = Some(profile);
        self
    }

    pub fn with_search_receipt(mut self, receipt: SubAgentSearchReceipt) -> Self {
        self.search_receipt = Some(receipt);
        self
    }
}

impl SubAgentSpawnProfile {
    pub fn new(
        profile_id: impl Into<String>,
        agent_type: impl Into<SubAgentType>,
        role: impl Into<String>,
    ) -> Self {
        Self {
            profile_id: profile_id.into(),
            agent_type: agent_type.into(),
            role: role.into(),
            nickname: None,
            hook_agent_scope: HookAgentScope::SubAgent,
        }
    }

    pub fn from_config(config: &SubAgentSpawnConfig) -> Self {
        Self {
            profile_id: config.profile_id.clone(),
            agent_type: config.agent_type.clone(),
            role: config.role.clone(),
            nickname: config.nickname.clone(),
            hook_agent_scope: config.hook_agent_scope.clone(),
        }
    }

    pub fn with_nickname(mut self, nickname: impl Into<String>) -> Self {
        self.nickname = Some(nickname.into());
        self
    }
}

impl SubAgentContextPolicy {
    pub fn isolated(session_id: impl Into<String>) -> Self {
        Self {
            session_id: Some(session_id.into()),
            visibility: vec![SubAgentContextVisibility::System],
            max_history_items: Some(0),
        }
    }

    pub fn workspace_read(session_id: impl Into<String>) -> Self {
        Self {
            session_id: Some(session_id.into()),
            visibility: vec![
                SubAgentContextVisibility::System,
                SubAgentContextVisibility::User,
                SubAgentContextVisibility::Workspace,
                SubAgentContextVisibility::Memory,
            ],
            max_history_items: Some(32),
        }
    }

    pub fn visible_context(&self) -> &[SubAgentContextVisibility] {
        &self.visibility
    }
}

impl SubAgentPermissionSet {
    pub fn read_only() -> Self {
        Self {
            read_only: true,
            workspace_write: false,
            network_access: false,
            process_spawn: false,
            descendant_spawn: false,
            tool_access: true,
            hook_access: false,
            secret_access: false,
        }
    }

    pub fn worker() -> Self {
        Self {
            read_only: false,
            workspace_write: true,
            network_access: false,
            process_spawn: false,
            descendant_spawn: false,
            tool_access: true,
            hook_access: false,
            secret_access: false,
        }
    }
}

impl Default for SubAgentPermissionSet {
    fn default() -> Self {
        Self::read_only()
    }
}

impl SubAgentPerformanceBudget {
    pub fn interactive() -> Self {
        Self {
            max_concurrency: Some(1),
            timeout_ms: Some(300_000),
            token_budget: None,
            max_depth: Some(1),
        }
    }

    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }

    pub fn with_max_concurrency(mut self, max_concurrency: u16) -> Self {
        self.max_concurrency = Some(max_concurrency);
        self
    }
}

impl Default for SubAgentPerformanceBudget {
    fn default() -> Self {
        Self::interactive()
    }
}

impl SubAgentSpawnPolicy {
    pub fn read_only(context: SubAgentContextPolicy) -> Self {
        Self {
            permissions: SubAgentPermissionSet::read_only(),
            context,
            performance: SubAgentPerformanceBudget::interactive(),
        }
    }
}

impl SubAgentSpawnConfig {
    pub fn toml(
        profile_id: impl Into<String>,
        agent_type: impl Into<SubAgentType>,
        role: impl Into<String>,
    ) -> Self {
        let profile_id = profile_id.into();
        Self {
            policy: SubAgentSpawnPolicy::read_only(SubAgentContextPolicy::workspace_read(
                profile_id.clone(),
            )),
            profile_id,
            agent_type: agent_type.into(),
            role: role.into(),
            nickname: None,
            hook_agent_scope: HookAgentScope::SubAgent,
            surface: SubAgentConfigSurface::Toml,
            strategy: SubAgentSpawnStrategy::Declarative,
        }
    }

    pub fn with_hook_agent_scope(mut self, hook_agent_scope: HookAgentScope) -> Self {
        self.hook_agent_scope = hook_agent_scope;
        self
    }

    pub fn with_nickname(mut self, nickname: impl Into<String>) -> Self {
        self.nickname = Some(nickname.into());
        self
    }

    pub fn with_strategy(mut self, strategy: SubAgentSpawnStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    pub fn with_policy(mut self, policy: SubAgentSpawnPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn child_session_id(&self) -> &str {
        self.policy
            .context
            .session_id
            .as_deref()
            .unwrap_or(&self.profile_id)
    }
}

impl SubAgentType {
    pub fn new(agent_type: impl Into<String>) -> Self {
        Self(agent_type.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for SubAgentType {
    fn from(agent_type: String) -> Self {
        Self::new(agent_type)
    }
}

impl From<&str> for SubAgentType {
    fn from(agent_type: &str) -> Self {
        Self::new(agent_type)
    }
}

impl SubAgentSearchReceipt {
    pub fn new(role: impl Into<String>, action: impl Into<String>) -> Self {
        Self {
            role: role.into(),
            action: action.into(),
            evidence: Vec::new(),
            missing: None,
            next: None,
            risk: None,
        }
    }

    pub fn with_evidence<I, S>(mut self, evidence: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.evidence = evidence.into_iter().map(Into::into).collect();
        self
    }

    pub fn with_missing(mut self, missing: impl Into<String>) -> Self {
        self.missing = Some(missing.into());
        self
    }

    pub fn with_next(mut self, next: impl Into<String>) -> Self {
        self.next = Some(next.into());
        self
    }

    pub fn with_risk(mut self, risk: impl Into<String>) -> Self {
        self.risk = Some(risk.into());
        self
    }
}
