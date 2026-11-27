// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Typed TOML route loading and cost guardrails for live Weaver LLM evaluations.

use std::collections::BTreeMap;
use std::env;
use std::fs::{self, File};
use std::io::Read;
use std::num::NonZeroUsize;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use marlin_agent_protocol::{ModelAlias, ModelEndpoint};
use marlin_agent_stream::{ModelStreamOutputLimit, WeaverModelGateway};
use serde::Deserialize;
use weaver_llm::{Client, Endpoint, Protocol};

/// Default per-turn stream output cap (2 MiB) for live harness turns.
const DEFAULT_MAX_STREAM_BYTES: usize = 2 * 1024 * 1024;
/// Maximum TOML route file size accepted by `LiveWeaverTomlConfig::load` (256 KiB).
pub const MAX_WEAVER_CONFIG_BYTES: usize = 256 * 1024;
/// Maximum HTTP timeout accepted by `LiveWeaverTomlConfig` (10 minutes).
pub const MAX_WEAVER_TIMEOUT_SECONDS: u64 = 600;
/// Maximum per-turn stream output cap accepted by `resolve_route_with` (32 MiB).
pub const MAX_HARNESS_STREAM_BYTES: usize = 32 * 1024 * 1024;

/// Supported wire protocols in harness TOML route files.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum LiveWeaverWireProtocol {
    OpenAi,
    OpenAiResponses,
    Anthropic,
}

impl LiveWeaverWireProtocol {
    fn into_weaver_protocol(self) -> Protocol {
        match self {
            Self::OpenAi => Protocol::OpenAi,
            Self::OpenAiResponses => Protocol::OpenAiResponses,
            Self::Anthropic => Protocol::Anthropic,
        }
    }
}

/// One named model route inside a TOML configuration envelope.
#[derive(Clone, Debug, Deserialize)]
pub struct LiveWeaverModelProfile {
    pub provider: String,
    pub model: Option<String>,
    pub endpoint: Option<String>,
    pub protocol: LiveWeaverWireProtocol,
    pub credential_env: String,
    pub anthropic: Option<weaver_llm::AnthropicSettings>,
}

/// Top-level TOML model route file compatible with Marlin's standard TOML configs
/// (e.g. `crates/marlin-agent-tui/example.toml` or dedicated harness TOML configs).
///
/// Unknown host-specific tables (such as `[session]` or `[process_budget]`) are
/// intentionally ignored here so one typed TOML route file can be shared across
/// headless harness runs and interactive hosts without duplicating credentials.
#[derive(Clone, Debug, Deserialize)]
pub struct LiveWeaverTomlConfig {
    pub provider: String,
    pub model: Option<String>,
    pub endpoint: Option<String>,
    pub protocol: LiveWeaverWireProtocol,
    pub credential_env: String,
    pub anthropic: Option<weaver_llm::AnthropicSettings>,
    #[serde(default)]
    pub models: BTreeMap<String, LiveWeaverModelProfile>,
    #[serde(default = "default_timeout_seconds")]
    pub timeout_seconds: u64,
}

fn default_timeout_seconds() -> u64 {
    120
}

/// Resolved live Weaver endpoint paired with its live streaming gateway.
#[derive(Clone)]
pub struct LiveWeaverRoute {
    pub alias: ModelAlias,
    pub endpoint: ModelEndpoint,
    pub gateway: Arc<WeaverModelGateway>,
}

/// Explicit per-scenario cost and safety bounds for live LLM harness runs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LiveEvalBudget {
    pub max_rounds: usize,
    pub max_tool_calls: usize,
    pub max_stream_bytes: usize,
    pub force_reflection: bool,
}

impl Default for LiveEvalBudget {
    fn default() -> Self {
        Self {
            max_rounds: 4,
            max_tool_calls: 24,
            max_stream_bytes: DEFAULT_MAX_STREAM_BYTES,
            force_reflection: false,
        }
    }
}

impl LiveEvalBudget {
    /// Load optional environment overrides (`MARLIN_LIVE_EVAL_MAX_ROUNDS`,
    /// `MARLIN_LIVE_EVAL_MAX_CALLS`, `MARLIN_LIVE_EVAL_REFLECTION`) while keeping
    /// strict hard upper bounds.
    #[must_use]
    pub fn from_env() -> Self {
        let mut budget = Self::default();
        if let Ok(raw) = env::var("MARLIN_LIVE_EVAL_MAX_ROUNDS")
            && let Ok(parsed) = raw.parse::<usize>()
        {
            budget.max_rounds = parsed.clamp(1, 16);
        }
        if let Ok(raw) = env::var("MARLIN_LIVE_EVAL_MAX_CALLS")
            && let Ok(parsed) = raw.parse::<usize>()
        {
            budget.max_tool_calls = parsed.clamp(1, 32);
        }
        if let Ok(raw) = env::var("MARLIN_LIVE_EVAL_REFLECTION") {
            let trimmed = raw.trim();
            budget.force_reflection = trimmed == "1" || trimmed.eq_ignore_ascii_case("true");
        }
        budget
    }
}

impl LiveWeaverTomlConfig {
    /// Load a typed TOML route file from disk with symlink refusal and size cap.
    pub fn load(path: &Path) -> Result<Self, String> {
        let meta = fs::symlink_metadata(path)
            .map_err(|err| format!("cannot inspect Weaver TOML config {}: {err}", path.display()))?;
        if meta.file_type().is_symlink() || !meta.is_file() {
            return Err(format!(
                "Weaver TOML config {} must be a regular non-symlink file",
                path.display()
            ));
        }
        let file = File::open(path)
            .map_err(|err| format!("cannot open Weaver TOML config {}: {err}", path.display()))?;
        let mut raw = String::new();
        file.take((MAX_WEAVER_CONFIG_BYTES + 1) as u64)
            .read_to_string(&mut raw)
            .map_err(|err| format!("cannot read Weaver TOML config {}: {err}", path.display()))?;
        if raw.len() > MAX_WEAVER_CONFIG_BYTES {
            return Err(format!(
                "Weaver TOML config {} exceeds {MAX_WEAVER_CONFIG_BYTES} bytes",
                path.display()
            ));
        }
        toml::from_str(&raw)
            .map_err(|err| format!("invalid Weaver TOML config {}: {err}", path.display()))
    }

    /// Resolve the requested model alias (`"default"` or any `[models.<alias>]`)
    /// into a live [`LiveWeaverRoute`] reading credentials strictly from the
    /// configured environment variable.
    pub fn resolve_route(
        &self,
        alias: &str,
        max_stream_bytes: usize,
    ) -> Result<LiveWeaverRoute, String> {
        self.resolve_route_with(alias, max_stream_bytes, |key| env::var(key).ok())
    }

    /// Deterministic resolver variant accepting an injected environment lookup.
    pub fn resolve_route_with(
        &self,
        alias: &str,
        max_stream_bytes: usize,
        read_env: impl Fn(&str) -> Option<String>,
    ) -> Result<LiveWeaverRoute, String> {
        if self.timeout_seconds == 0 || self.timeout_seconds > MAX_WEAVER_TIMEOUT_SECONDS {
            return Err(format!(
                "Weaver timeout_seconds={} must be within 1..={MAX_WEAVER_TIMEOUT_SECONDS}",
                self.timeout_seconds
            ));
        }
        if max_stream_bytes == 0 || max_stream_bytes > MAX_HARNESS_STREAM_BYTES {
            return Err(format!(
                "Weaver max_stream_bytes={max_stream_bytes} must be within 1..={MAX_HARNESS_STREAM_BYTES}"
            ));
        }
        let profile = if alias == "default" {
            LiveWeaverModelProfile {
                provider: self.provider.clone(),
                model: self.model.clone(),
                endpoint: self.endpoint.clone(),
                protocol: self.protocol,
                credential_env: self.credential_env.clone(),
                anthropic: self.anthropic,
            }
        } else {
            let mut named = self
                .models
                .get(alias)
                .cloned()
                .ok_or_else(|| format!("unknown model alias `{alias}` in TOML config"))?;
            if named.anthropic.is_none()
                && matches!(named.protocol, LiveWeaverWireProtocol::Anthropic)
            {
                named.anthropic = self.anthropic;
            }
            named
        };

        validate_credential_env_name(&profile.credential_env)?;

        let protocol = profile.protocol.into_weaver_protocol();
        let options = profile
            .anthropic
            .map(|settings| settings.options(protocol))
            .transpose()
            .map_err(|err| format!("invalid Anthropic settings: {err}"))?
            .unwrap_or_default();

        let (resolved_endpoint, resolved_model) = weaver_llm::resolve_direct_target(
            protocol,
            profile.endpoint.as_deref(),
            profile.model.as_deref(),
            &read_env,
        )
        .map_err(|err| format!("failed to resolve Weaver target for alias `{alias}`: {err}"))?;

        let model_alias = ModelAlias::new(alias.to_owned());
        let mut identity = ModelEndpoint::new(profile.provider.clone(), resolved_model);
        if alias != "default" {
            identity = identity.with_alias(model_alias.clone());
        }
        identity
            .validate_contract()
            .map_err(|err| err.to_string())?;

        let endpoint = Endpoint::new(protocol, &resolved_endpoint)
            .map_err(|err| format!("invalid Weaver endpoint `{resolved_endpoint}`: {err}"))?;
        let credential = read_env(&profile.credential_env)
            .filter(|v| !v.is_empty())
            .ok_or_else(|| {
                format!(
                    "credential environment variable `{}` is missing or empty",
                    profile.credential_env
                )
            })?;

        let client = Client::new(Duration::from_secs(self.timeout_seconds), 8 * 1024 * 1024)
            .map_err(|err| format!("cannot initialize Weaver HTTP client: {err}"))?;
        let limit = NonZeroUsize::new(max_stream_bytes.max(4096))
            .expect("clamped stream output limit is non-zero");
        let gateway = WeaverModelGateway::new(
            client,
            profile.provider.into(),
            endpoint,
            credential,
        )
        .with_default_options(options)
        .with_stream_output_limit(ModelStreamOutputLimit::new(limit));

        Ok(LiveWeaverRoute {
            alias: model_alias,
            endpoint: identity,
            gateway: Arc::new(gateway),
        })
    }
}

fn validate_credential_env_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name.len() > 128 {
        return Err("credential_env must be a non-empty ASCII environment variable name".to_owned());
    }
    let bytes = name.as_bytes();
    if !(bytes[0].is_ascii_alphabetic() || bytes[0] == b'_')
        || !bytes
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || *b == b'_')
    {
        return Err(format!(
            "credential_env `{name}` contains invalid characters"
        ));
    }
    let upper = name.to_ascii_uppercase();
    let is_ambient_or_loader = matches!(
        upper.as_str(),
        "PATH"
            | "HOME"
            | "USER"
            | "SHELL"
            | "PWD"
            | "TERM"
            | "TMPDIR"
            | "BASH_ENV"
            | "ENV"
            | "IFS"
            | "PROMPT_COMMAND"
            | "HTTP_PROXY"
            | "HTTPS_PROXY"
            | "NO_PROXY"
    ) || upper.starts_with("LD_")
        || upper.starts_with("DYLD_")
        || upper.starts_with("SSH_")
        || upper.starts_with("GIT_");
    if is_ambient_or_loader {
        return Err(format!(
            "credential_env `{name}` references a disallowed ambient/system environment variable"
        ));
    }
    Ok(())
}
