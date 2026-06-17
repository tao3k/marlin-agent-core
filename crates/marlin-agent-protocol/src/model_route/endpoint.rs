//! Provider/model endpoint identity and validation for `model_route` rules.

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

use serde::{Deserialize, Serialize};

use super::{LiteLlmModelId, ModelAlias, ModelName, ModelProviderId};

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
