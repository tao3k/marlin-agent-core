// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Typed terminal configuration and model route selection.

use std::{
    collections::BTreeMap,
    env, fs,
    io::Read,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use marlin_agent_protocol::{ModelAlias, ModelEndpoint};
use marlin_agent_stream::{ModelStreamOutputLimit, WeaverModelGateway};
use serde::Deserialize;
use weaver_llm::{Client, Endpoint, Protocol};

use super::model_catalog::{DEFAULT_MODEL_ALIAS, MAX_MODEL_CHOICES, ModelCatalog, ModelRoute};

/// Hard ceiling on TUI TOML configuration file reads (SM39).
const MAX_TUI_CONFIG_BYTES: usize = 256 * 1024;
/// Hard ceiling on configured HTTP client timeout seconds (SM39).
const MAX_TUI_TIMEOUT_SECONDS: u64 = 600;

fn validate_timeout_seconds(timeout_seconds: u64) -> Result<(), String> {
    if timeout_seconds == 0 || timeout_seconds > MAX_TUI_TIMEOUT_SECONDS {
        return Err(format!(
            "timeout_seconds must be within 1..={MAX_TUI_TIMEOUT_SECONDS}, got {timeout_seconds}"
        ));
    }
    Ok(())
}

fn validate_tui_credential_env(name: &str) -> Result<(), String> {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return Err("credential_env must not be empty".to_owned());
    };
    if !(first.is_ascii_alphabetic() || first == '_')
        || !chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Err(format!("credential_env '{name}' is not a valid env identifier"));
    }
    let upper = name.to_ascii_uppercase();
    const FORBIDDEN_EXACT: &[&str] = &[
        "PATH",
        "HOME",
        "USER",
        "LOGNAME",
        "SHELL",
        "PWD",
        "OLDPWD",
        "TMPDIR",
        "TERM",
        "BASH_ENV",
        "ENV",
        "IFS",
        "RUSTFLAGS",
        "NODE_OPTIONS",
        "PYTHONPATH",
    ];
    const FORBIDDEN_PREFIXES: &[&str] = &[
        "LD_",
        "DYLD_",
        "SSH_",
        "GIT_",
        "XDG_",
        "CARGO_",
        "GERBIL_",
    ];
    if FORBIDDEN_EXACT.contains(&upper.as_str())
        || FORBIDDEN_PREFIXES
            .iter()
            .any(|prefix| upper.starts_with(prefix))
    {
        return Err(format!(
            "credential_env '{name}' references a forbidden ambient system variable"
        ));
    }
    Ok(())
}

fn read_bounded_non_symlink_config(path: &Path) -> Result<String, String> {
    let meta = fs::symlink_metadata(path).map_err(|e| format!("cannot stat config: {e}"))?;
    if meta.file_type().is_symlink() || !meta.file_type().is_file() {
        return Err("config path must be a regular non-symlink file".to_owned());
    }
    if meta.len() > MAX_TUI_CONFIG_BYTES as u64 {
        return Err(format!(
            "config exceeds maximum size of {MAX_TUI_CONFIG_BYTES} bytes"
        ));
    }
    let mut opts = fs::OpenOptions::new();
    opts.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.custom_flags(libc::O_NOFOLLOW);
    }
    let file = opts
        .open(path)
        .map_err(|e| format!("cannot read config: {e}"))?;
    let mut content = String::new();
    file.take((MAX_TUI_CONFIG_BYTES + 1) as u64)
        .read_to_string(&mut content)
        .map_err(|e| format!("cannot read config: {e}"))?;
    if content.len() > MAX_TUI_CONFIG_BYTES {
        return Err(format!(
            "config exceeds maximum size of {MAX_TUI_CONFIG_BYTES} bytes"
        ));
    }
    Ok(content)
}

fn validate_tui_ui_package(pkg: &TuiUiPackageConfig) -> Result<(), String> {
    if !pkg.directory.is_absolute() {
        return Err("tui_ui_package.directory must be an absolute path".to_owned());
    }
    for (label, value) in [("module", &pkg.module), ("export", &pkg.export)] {
        if value.trim().is_empty()
            || value.contains("..")
            || value.chars().any(char::is_control)
        {
            return Err(format!("tui_ui_package.{label} is invalid"));
        }
    }
    Ok(())
}

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
    pub tui_ui_package: Option<TuiUiPackageConfig>,
    pub git_context: Option<GitContextConfig>,
}

/// Local operator approval to display committed Context from one Git owner.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GitContextConfig {
    pub repository: PathBuf,
}

/// An already compiled Scheme package declaration resolved by the SDK.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TuiUiPackageConfig {
    pub directory: PathBuf,
    pub module: String,
    pub export: String,
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
#[derive(Clone, Deserialize)]
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
        let content = read_bounded_non_symlink_config(path)?;
        let config: Self = toml::from_str(&content).map_err(|e| format!("invalid config: {e}"))?;
        validate_timeout_seconds(config.timeout_seconds)?;
        validate_tui_credential_env(&config.credential_env)?;
        for profile in config.models.values() {
            validate_tui_credential_env(&profile.credential_env)?;
        }
        if let Some(pkg) = config.tui_ui_package.as_ref() {
            validate_tui_ui_package(pkg)?;
        }
        if config
            .git_context
            .as_ref()
            .is_some_and(|approval| !approval.repository.is_absolute())
        {
            return Err("git_context.repository must be an absolute path".into());
        }
        Ok(config)
    }

    pub fn catalog(&self) -> Result<(ModelEndpoint, ModelCatalog), String> {
        self.catalog_with(|name| env::var(name).ok())
    }

    pub(crate) fn catalog_with(
        &self,
        read_env: impl Fn(&str) -> Option<String>,
    ) -> Result<(ModelEndpoint, ModelCatalog), String> {
        validate_timeout_seconds(self.timeout_seconds)?;
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
            let mut effective = profile.clone();
            if effective.anthropic.is_none() && matches!(effective.protocol, WireProtocol::Anthropic) {
                effective.anthropic = self.anthropic;
            }
            routes.push(effective.route(ModelAlias::new(name.clone()), &client, &read_env, false)?);
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
        validate_tui_credential_env(&self.credential_env)?;
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
