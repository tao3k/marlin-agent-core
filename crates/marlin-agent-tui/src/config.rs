// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Typed terminal configuration and model route selection.

use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use marlin_agent_protocol::{ModelAlias, ModelEndpoint};
use marlin_agent_stream::{ModelStreamOutputLimit, WeaverModelGateway};
use serde::Deserialize;
use weaver_llm::{Client, Endpoint, Protocol};

use super::model_catalog::{DEFAULT_MODEL_ALIAS, MAX_MODEL_CHOICES, ModelCatalog, ModelRoute};

/// Credentials are read from the named environment variable, never from TOML.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub provider: String,
    pub model: Option<String>,
    pub endpoint: Option<String>,
    pub protocol: WireProtocol,
    pub credential_env: String,
    pub anthropic: Option<weaver_llm::AnthropicSettings>,
    pub session: Option<SessionConfig>,
    #[serde(default)]
    pub models: BTreeMap<String, ModelProfile>,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
    pub process_budget: Option<marlin_agent_runtime::RuntimeProcessBudget>,
}

/// Persistent session identity and local storage location.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionConfig {
    pub path: PathBuf,
    pub project: String,
    pub session: String,
    pub agent: String,
}

/// Supported wire protocol for a configured model route.
#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WireProtocol {
    OpenAi,
    OpenAiResponses,
    Anthropic,
}

/// One named model route with its own provider and credentials.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelProfile {
    pub provider: String,
    pub model: Option<String>,
    pub endpoint: Option<String>,
    pub protocol: WireProtocol,
    pub credential_env: String,
    pub anthropic: Option<weaver_llm::AnthropicSettings>,
}

fn default_timeout() -> u64 {
    120
}

impl Config {
    pub fn load(path: &Path) -> Result<Self, String> {
        let content = fs::read_to_string(path).map_err(|e| format!("cannot read config: {e}"))?;
        toml::from_str(&content).map_err(|e| format!("invalid config: {e}"))
    }

    pub fn catalog(&self) -> Result<(ModelEndpoint, ModelCatalog), String> {
        self.catalog_with(|name| env::var(name).ok())
    }

    pub(crate) fn catalog_with(
        &self,
        read_env: impl Fn(&str) -> Option<String>,
    ) -> Result<(ModelEndpoint, ModelCatalog), String> {
        if self.models.len() >= MAX_MODEL_CHOICES {
            return Err(format!(
                "at most {MAX_MODEL_CHOICES} models including default"
            ));
        }
        if self.models.contains_key(DEFAULT_MODEL_ALIAS) {
            return Err("model alias default is reserved".into());
        }
        let client = Client::new(Duration::from_secs(self.timeout_seconds), 8 * 1024 * 1024)
            .map_err(|e| format!("cannot create client: {e}"))?;
        let primary = ModelProfile {
            provider: self.provider.clone(),
            model: self.model.clone(),
            endpoint: self.endpoint.clone(),
            protocol: self.protocol,
            credential_env: self.credential_env.clone(),
            anthropic: self.anthropic,
        };
        let default = primary.route(
            ModelAlias::new(DEFAULT_MODEL_ALIAS),
            &client,
            &read_env,
            true,
        )?;
        let identity = default.endpoint.clone();
        let mut routes = vec![default];
        for (name, profile) in &self.models {
            routes.push(profile.route(ModelAlias::new(name.clone()), &client, &read_env, false)?);
        }
        Ok((identity, ModelCatalog::new(routes)?))
    }
}

impl ModelProfile {
    fn route(
        &self,
        alias: ModelAlias,
        client: &Client,
        read_env: &impl Fn(&str) -> Option<String>,
        required: bool,
    ) -> Result<ModelRoute, String> {
        let protocol = match self.protocol {
            WireProtocol::OpenAi => Protocol::OpenAi,
            WireProtocol::OpenAiResponses => Protocol::OpenAiResponses,
            WireProtocol::Anthropic => Protocol::Anthropic,
        };
        let options = self
            .anthropic
            .map(|settings| settings.options(protocol))
            .transpose()
            .map_err(|error| format!("invalid Anthropic settings: {error}"))?
            .unwrap_or_default();
        let (resolved_endpoint, resolved_model) = weaver_llm::resolve_direct_target(
            protocol,
            self.endpoint.as_deref(),
            self.model.as_deref(),
            read_env,
        )
        .map_err(|_| match self.protocol {
            WireProtocol::Anthropic => "model or endpoint is missing or invalid; configure them or set ANTHROPIC_MODEL and ANTHROPIC_BASE_URL".to_owned(),
            _ => "model endpoint or model is missing or invalid".to_owned(),
        })?;
        let mut identity = ModelEndpoint::new(self.provider.clone(), resolved_model);
        if alias.as_str() != DEFAULT_MODEL_ALIAS {
            identity = identity.with_alias(alias.clone());
        }
        identity.validate_contract().map_err(|e| e.to_string())?;
        let endpoint = Endpoint::new(protocol, &resolved_endpoint)
            .map_err(|e| format!("invalid endpoint: {e}"))?;
        let credential = read_env(&self.credential_env).filter(|value| !value.is_empty());
        if required && credential.is_none() {
            return Err(format!(
                "credential environment variable {} is missing or empty",
                self.credential_env
            ));
        }
        let gateway = credential.map(|credential| {
            let gateway = WeaverModelGateway::new(
                client.clone(),
                self.provider.clone().into(),
                endpoint,
                credential,
            )
            .with_default_options(options)
            .with_stream_output_limit(ModelStreamOutputLimit::new(
                std::num::NonZeroUsize::new(super::output_limit::MAX_RESPONSE_BYTES)
                    .expect("positive output limit"),
            ));
            Arc::new(gateway) as Arc<dyn marlin_agent_stream::ModelStreamingGateway>
        });
        Ok(ModelRoute {
            alias,
            endpoint: identity,
            gateway,
        })
    }
}
