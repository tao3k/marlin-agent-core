//! Model routing protocol for provider/model identity and sub-agent session policy.

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

use serde::{Deserialize, Serialize};

use crate::RuntimeEnvironmentActivationReceipt;

macro_rules! semantic_string {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            pub fn as_str(&self) -> &str {
                self.0.as_str()
            }

            pub fn into_string(self) -> String {
                self.0
            }
        }

        impl Display for $name {
            fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
                formatter.write_str(self.as_str())
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self::new(value)
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self::new(value)
            }
        }
    };
}

semantic_string!(
    /// Real model provider identifier, for example `openai`, `anthropic`, or `local`.
    ModelProviderId
);

semantic_string!(
    /// Provider-owned model name, for example `gpt-5` or `claude-opus-4-8`.
    ModelName
);

semantic_string!(
    /// Optional local alias used for configuration ergonomics.
    ModelAlias
);

semantic_string!(
    /// Stable identifier for a route rule.
    ModelRouteRuleId
);

semantic_string!(
    /// String passed to LiteLLM as the concrete model selector.
    LiteLlmModelId
);

semantic_string!(
    /// Typed command kind used by routing requests.
    ModelCommandKind
);

semantic_string!(
    /// Persistent session reuse key.
    ModelSessionPersistenceKey
);

semantic_string!(
    /// Session pool identifier.
    ModelSessionPoolId
);

semantic_string!(
    /// Requested routed session id.
    ModelRouteSessionId
);

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

/// Concrete model endpoint selected by routing policy.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ModelEndpoint {
    pub provider: ModelProviderId,
    pub model: ModelName,
    pub alias: Option<ModelAlias>,
}

impl ModelEndpoint {
    pub fn new(provider: impl Into<ModelProviderId>, model: impl Into<ModelName>) -> Self {
        Self {
            provider: provider.into(),
            model: model.into(),
            alias: None,
        }
    }

    pub fn with_alias(mut self, alias: impl Into<ModelAlias>) -> Self {
        self.alias = Some(alias.into());
        self
    }

    pub fn litellm_model_id(&self) -> LiteLlmModelId {
        LiteLlmModelId::new(format!(
            "{}/{}",
            self.provider.as_str(),
            self.model.as_str()
        ))
    }

    pub fn validate_contract(&self) -> Result<(), ModelEndpointContractError> {
        self.validate_non_empty_identity()?;
        self.validate_selector_boundaries()?;
        self.validate_provider_model_roles()?;
        self.validate_provider_model_family()?;

        Ok(())
    }

    fn validate_non_empty_identity(&self) -> Result<(), ModelEndpointContractError> {
        let provider = self.provider.as_str();
        let model = self.model.as_str();

        if provider.trim().is_empty() {
            return Err(ModelEndpointContractError::EmptyProvider);
        }
        if model.trim().is_empty() {
            return Err(ModelEndpointContractError::EmptyModel);
        }

        Ok(())
    }

    fn validate_selector_boundaries(&self) -> Result<(), ModelEndpointContractError> {
        let provider = self.provider.as_str();
        let model = self.model.as_str();

        if provider.contains('/') {
            return Err(ModelEndpointContractError::ProviderContainsModelSeparator {
                provider: self.provider.clone(),
            });
        }
        if model.contains('/') {
            return Err(ModelEndpointContractError::ModelContainsProviderSeparator {
                model: self.model.clone(),
            });
        }

        Ok(())
    }

    fn validate_provider_model_roles(&self) -> Result<(), ModelEndpointContractError> {
        let provider_lower = self.provider.as_str().to_ascii_lowercase();
        let model_lower = self.model.as_str().to_ascii_lowercase();

        if provider_lower.starts_with("gpt")
            || provider_lower.starts_with("claude")
            || provider_lower == "codex"
        {
            return Err(ModelEndpointContractError::ProviderLooksLikeModel {
                provider: self.provider.clone(),
            });
        }
        if model_lower == "openai" || model_lower == "anthropic" {
            return Err(ModelEndpointContractError::ModelLooksLikeProvider {
                provider: self.provider.clone(),
                model: self.model.clone(),
            });
        }
        if model_lower.contains("codex") {
            return Err(ModelEndpointContractError::CodexIsNotModelName {
                model: self.model.clone(),
            });
        }

        Ok(())
    }

    fn validate_provider_model_family(&self) -> Result<(), ModelEndpointContractError> {
        let provider_lower = self.provider.as_str().to_ascii_lowercase();
        let model_lower = self.model.as_str().to_ascii_lowercase();

        if provider_lower == "openai" && !model_lower.starts_with("gpt-") {
            return Err(ModelEndpointContractError::OpenAiModelMustBeGpt {
                model: self.model.clone(),
            });
        }
        if provider_lower == "anthropic" && !model_lower.starts_with("claude-") {
            return Err(ModelEndpointContractError::AnthropicModelMustBeClaude {
                model: self.model.clone(),
            });
        }

        Ok(())
    }
}

/// Endpoint identity contract violation for model routing.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModelEndpointContractError {
    EmptyProvider,
    EmptyModel,
    ProviderContainsModelSeparator {
        provider: ModelProviderId,
    },
    ModelContainsProviderSeparator {
        model: ModelName,
    },
    ProviderLooksLikeModel {
        provider: ModelProviderId,
    },
    ModelLooksLikeProvider {
        provider: ModelProviderId,
        model: ModelName,
    },
    CodexIsNotModelName {
        model: ModelName,
    },
    OpenAiModelMustBeGpt {
        model: ModelName,
    },
    AnthropicModelMustBeClaude {
        model: ModelName,
    },
}

impl Display for ModelEndpointContractError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyProvider => formatter.write_str("model route provider must not be empty"),
            Self::EmptyModel => formatter.write_str("model route model must not be empty"),
            Self::ProviderContainsModelSeparator { provider } => write!(
                formatter,
                "model route provider `{provider}` must be a provider id, not a provider/model selector"
            ),
            Self::ModelContainsProviderSeparator { model } => write!(
                formatter,
                "model route model `{model}` must be a provider-owned model id without provider separator"
            ),
            Self::ProviderLooksLikeModel { provider } => write!(
                formatter,
                "model route provider `{provider}` looks like a model id"
            ),
            Self::ModelLooksLikeProvider { provider, model } => write!(
                formatter,
                "model route model `{model}` for provider `{provider}` looks like a provider id"
            ),
            Self::CodexIsNotModelName { model } => write!(
                formatter,
                "model route model `{model}` uses Codex as a product/tool name instead of a GPT model id"
            ),
            Self::OpenAiModelMustBeGpt { model } => {
                write!(
                    formatter,
                    "OpenAI model route `{model}` must use a GPT model id"
                )
            }
            Self::AnthropicModelMustBeClaude { model } => write!(
                formatter,
                "Anthropic model route `{model}` must use a Claude model id"
            ),
        }
    }
}

impl Error for ModelEndpointContractError {}

/// Glob-backed command matcher surface. Empty dimensions match everything.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ModelCommandMatcher {
    pub executable_globs: Vec<String>,
    pub argv_globs: Vec<String>,
    pub cwd_globs: Vec<String>,
    pub workspace_globs: Vec<String>,
    pub sub_agent_role_globs: Vec<String>,
    pub agent_scope_globs: Vec<String>,
    pub command_kind_globs: Vec<String>,
}

impl ModelCommandMatcher {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_argv_glob(mut self, glob: impl Into<String>) -> Self {
        self.argv_globs.push(glob.into());
        self
    }

    pub fn with_executable_glob(mut self, glob: impl Into<String>) -> Self {
        self.executable_globs.push(glob.into());
        self
    }

    pub fn with_sub_agent_role_glob(mut self, glob: impl Into<String>) -> Self {
        self.sub_agent_role_globs.push(glob.into());
        self
    }

    pub fn with_agent_scope_glob(mut self, glob: impl Into<String>) -> Self {
        self.agent_scope_globs.push(glob.into());
        self
    }

    pub fn with_cwd_glob(mut self, glob: impl Into<String>) -> Self {
        self.cwd_globs.push(glob.into());
        self
    }

    pub fn with_workspace_glob(mut self, glob: impl Into<String>) -> Self {
        self.workspace_globs.push(glob.into());
        self
    }

    pub fn with_command_kind_glob(mut self, glob: impl Into<String>) -> Self {
        self.command_kind_globs.push(glob.into());
        self
    }
}

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

/// Runtime request facts used to resolve a command or sub-agent call to a model.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ModelRouteRequest {
    pub executable: Option<String>,
    pub argv: Vec<String>,
    pub cwd: Option<String>,
    pub workspace: Option<String>,
    pub sub_agent_role: Option<String>,
    #[serde(default)]
    pub agent_scope: Option<ModelRouteAgentScope>,
    pub command_kind: Option<ModelCommandKind>,
}

impl ModelRouteRequest {
    pub fn command(argv: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let argv = argv.into_iter().map(Into::into).collect::<Vec<_>>();
        let executable = argv.first().cloned();
        Self {
            executable,
            argv,
            cwd: None,
            workspace: None,
            sub_agent_role: None,
            agent_scope: None,
            command_kind: None,
        }
    }

    pub fn command_line(&self) -> String {
        self.argv.join(" ")
    }

    pub fn with_cwd(mut self, cwd: impl Into<String>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    pub fn with_workspace(mut self, workspace: impl Into<String>) -> Self {
        self.workspace = Some(workspace.into());
        self
    }

    pub fn with_sub_agent_role(mut self, role: impl Into<String>) -> Self {
        self.sub_agent_role = Some(role.into());
        self
    }

    pub fn with_agent_scope(mut self, agent_scope: impl Into<ModelRouteAgentScope>) -> Self {
        self.agent_scope = Some(agent_scope.into());
        self
    }

    pub fn with_command_kind(mut self, kind: impl Into<ModelCommandKind>) -> Self {
        self.command_kind = Some(kind.into());
        self
    }
}

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
