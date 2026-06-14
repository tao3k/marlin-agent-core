//! Rust binding for the Deck runtime Scheme model-route policy selector.

use marlin_agent_protocol::{ModelRouteAgentScope, ModelRouteRequest};
use serde::{Deserialize, Serialize};

/// Context policy mode returned by the Scheme Deck runtime selector.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GerbilDeckRuntimeContextMode(String);

impl GerbilDeckRuntimeContextMode {
    pub fn new(mode: impl Into<String>) -> Self {
        Self(mode.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for GerbilDeckRuntimeContextMode {
    fn default() -> Self {
        Self::new("forked-context")
    }
}

impl From<&str> for GerbilDeckRuntimeContextMode {
    fn from(mode: &str) -> Self {
        Self::new(mode)
    }
}

impl From<String> for GerbilDeckRuntimeContextMode {
    fn from(mode: String) -> Self {
        Self::new(mode)
    }
}

/// Isolation policy mode returned by the Scheme Deck runtime selector.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GerbilDeckRuntimeIsolationMode(String);

impl GerbilDeckRuntimeIsolationMode {
    pub fn new(mode: impl Into<String>) -> Self {
        Self(mode.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for GerbilDeckRuntimeIsolationMode {
    fn default() -> Self {
        Self::new("workspace-isolated")
    }
}

impl From<&str> for GerbilDeckRuntimeIsolationMode {
    fn from(mode: &str) -> Self {
        Self::new(mode)
    }
}

impl From<String> for GerbilDeckRuntimeIsolationMode {
    fn from(mode: String) -> Self {
        Self::new(mode)
    }
}

/// Selected policy kind returned by the Scheme Deck runtime selector.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GerbilDeckRuntimeSelectedPolicyKind(String);

impl GerbilDeckRuntimeSelectedPolicyKind {
    pub fn new(kind: impl Into<String>) -> Self {
        Self(kind.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for GerbilDeckRuntimeSelectedPolicyKind {
    fn from(kind: &str) -> Self {
        Self::new(kind)
    }
}

impl From<String> for GerbilDeckRuntimeSelectedPolicyKind {
    fn from(kind: String) -> Self {
        Self::new(kind)
    }
}

/// Scheme-side model route policy input.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilDeckRuntimeModelRoutePolicy {
    pub name: String,
    pub provider: String,
    pub model: String,
    pub command_prefixes: Vec<String>,
    pub agent_scopes: Vec<String>,
    pub context_mode: GerbilDeckRuntimeContextMode,
    pub isolation_mode: GerbilDeckRuntimeIsolationMode,
}

impl GerbilDeckRuntimeModelRoutePolicy {
    pub fn new(
        name: impl Into<String>,
        provider: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            provider: provider.into(),
            model: model.into(),
            command_prefixes: Vec::new(),
            agent_scopes: Vec::new(),
            context_mode: GerbilDeckRuntimeContextMode::default(),
            isolation_mode: GerbilDeckRuntimeIsolationMode::default(),
        }
    }

    pub fn with_command_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.command_prefixes.push(prefix.into());
        self
    }

    pub fn with_agent_scope(mut self, scope: impl Into<String>) -> Self {
        self.agent_scopes.push(scope.into());
        self
    }

    pub fn with_context_mode(mut self, mode: impl Into<GerbilDeckRuntimeContextMode>) -> Self {
        self.context_mode = mode.into();
        self
    }

    pub fn with_isolation_mode(mut self, mode: impl Into<GerbilDeckRuntimeIsolationMode>) -> Self {
        self.isolation_mode = mode.into();
        self
    }
}

/// Request sent to the Scheme Deck runtime policy selector.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilDeckRuntimeModelRoutePolicyRequest {
    pub policies: Vec<GerbilDeckRuntimeModelRoutePolicy>,
    pub command: String,
    pub agent_scope: String,
}

impl GerbilDeckRuntimeModelRoutePolicyRequest {
    pub fn new(command: impl Into<String>, agent_scope: impl Into<String>) -> Self {
        Self {
            policies: Vec::new(),
            command: command.into(),
            agent_scope: agent_scope.into(),
        }
    }

    pub fn from_model_route_request(
        policies: impl IntoIterator<Item = GerbilDeckRuntimeModelRoutePolicy>,
        request: &ModelRouteRequest,
    ) -> Self {
        Self {
            policies: policies.into_iter().collect(),
            command: request.command_line(),
            agent_scope: request
                .agent_scope
                .as_ref()
                .map(model_route_agent_scope_label)
                .or_else(|| request.sub_agent_role.clone())
                .unwrap_or_else(|| "any".to_string()),
        }
    }

    pub fn with_policy(mut self, policy: GerbilDeckRuntimeModelRoutePolicy) -> Self {
        self.policies.push(policy);
        self
    }
}

/// Selected policy fields returned by the Scheme selector.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilDeckRuntimeModelRouteSelectedPolicy {
    pub kind: GerbilDeckRuntimeSelectedPolicyKind,
    pub name: String,
    pub provider: String,
    pub model: String,
    pub command_prefixes: Vec<String>,
    pub agent_scopes: Vec<String>,
    pub context_mode: GerbilDeckRuntimeContextMode,
    pub isolation_mode: GerbilDeckRuntimeIsolationMode,
}

/// Receipt returned by the Scheme Deck runtime policy selector.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilDeckRuntimeModelRouteSelectionReceipt {
    pub schema_id: String,
    pub command: String,
    pub agent_scope: String,
    pub matched: bool,
    pub policy: Option<GerbilDeckRuntimeModelRouteSelectedPolicy>,
}

impl GerbilDeckRuntimeModelRouteSelectionReceipt {
    pub fn selected_policy(&self) -> Option<&GerbilDeckRuntimeModelRouteSelectedPolicy> {
        self.policy.as_ref()
    }
}

fn model_route_agent_scope_label(scope: &ModelRouteAgentScope) -> String {
    match scope {
        ModelRouteAgentScope::Any => "any",
        ModelRouteAgentScope::RootAgent => "root-agent",
        ModelRouteAgentScope::SubAgent => "sub-agent",
        ModelRouteAgentScope::CustomAgent => "custom-agent",
        ModelRouteAgentScope::CustomerAgent => "customer-agent",
        ModelRouteAgentScope::ForkedAgent => "forked-agent",
        ModelRouteAgentScope::IsolatedAgent => "isolated-agent",
        ModelRouteAgentScope::PersistentAgent => "persistent-agent",
    }
    .to_string()
}
