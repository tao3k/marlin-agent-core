//! Rust binding for the Deck runtime Scheme model-route policy selector.

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    io::{self, Write},
    path::{Path, PathBuf},
    process::{Command as StdCommand, Stdio},
};

use marlin_agent_protocol::{ModelRouteAgentScope, ModelRouteRequest};
use serde::{Deserialize, Serialize};

use crate::{
    GerbilCommandSpec,
    runtime::{default_gerbil_gxi_program, write_gerbil_runtime_assets},
};

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

/// Command-backed evaluator for the Deck runtime Scheme policy selector.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilDeckRuntimeModelRoutePolicyEvaluator {
    spec: GerbilCommandSpec,
}

impl GerbilDeckRuntimeModelRoutePolicyEvaluator {
    pub fn new(spec: GerbilCommandSpec) -> Self {
        Self { spec }
    }

    /// Builds an evaluator for the crate-shipped `deck-runtime-policy-adapter.ss` launcher.
    pub fn from_marlin_deck_runtime_policy_launcher(
        program: impl Into<PathBuf>,
        loadpath_root: impl Into<PathBuf>,
    ) -> Self {
        Self::new(GerbilCommandSpec::marlin_deck_runtime_policy_launcher(
            program,
            loadpath_root,
        ))
    }

    pub fn spec(&self) -> &GerbilCommandSpec {
        &self.spec
    }

    /// Evaluate a model-route policy request through the Scheme runtime selector.
    pub fn evaluate(
        &self,
        request: GerbilDeckRuntimeModelRoutePolicyRequest,
    ) -> Result<GerbilDeckRuntimeModelRouteSelectionReceipt, GerbilDeckRuntimeModelRoutePolicyError>
    {
        let mut request_json = serde_json::to_vec(&request).map_err(|error| {
            GerbilDeckRuntimeModelRoutePolicyError::Encode {
                message: error.to_string(),
            }
        })?;
        request_json.push(b'\n');

        let output = self.run_command_with_stdin(request_json)?;
        let output_json = String::from_utf8(output.stdout).map_err(|error| {
            GerbilDeckRuntimeModelRoutePolicyError::Decode {
                message: format!("deck runtime policy stdout is not UTF-8: {error}"),
            }
        })?;

        decode_gerbil_deck_runtime_model_route_selection(output_json.trim())
    }

    fn run_command_with_stdin(
        &self,
        stdin_bytes: Vec<u8>,
    ) -> Result<std::process::Output, GerbilDeckRuntimeModelRoutePolicyError> {
        let mut command = StdCommand::new(&self.spec.program);
        command
            .args(&self.spec.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if let Some(current_dir) = &self.spec.current_dir {
            command.current_dir(current_dir);
        }

        command.envs(&self.spec.env);

        let mut child =
            command
                .spawn()
                .map_err(|error| GerbilDeckRuntimeModelRoutePolicyError::Command {
                    message: format!("failed to start deck runtime policy command: {error}"),
                })?;

        {
            let mut stdin = child.stdin.take().ok_or_else(|| {
                GerbilDeckRuntimeModelRoutePolicyError::Command {
                    message: "deck runtime policy command did not expose stdin".to_string(),
                }
            })?;
            if let Err(error) = stdin.write_all(&stdin_bytes)
                && error.kind() != io::ErrorKind::BrokenPipe
            {
                terminate_deck_runtime_policy_child_after_start_failure(&mut child);
                return Err(GerbilDeckRuntimeModelRoutePolicyError::Command {
                    message: format!("failed to write deck runtime policy request: {error}"),
                });
            }
        }

        let output = child.wait_with_output().map_err(|error| {
            GerbilDeckRuntimeModelRoutePolicyError::Command {
                message: format!("failed to read deck runtime policy command output: {error}"),
            }
        })?;

        validate_deck_runtime_policy_command_output(output)
    }
}

/// One-stop Rust binding for the crate-shipped Deck runtime policy selector.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilDeckRuntimeModelRoutePolicyRuntimeBinding {
    loadpath_root: PathBuf,
    written_assets: Vec<PathBuf>,
    evaluator: GerbilDeckRuntimeModelRoutePolicyEvaluator,
}

impl GerbilDeckRuntimeModelRoutePolicyRuntimeBinding {
    /// Writes runtime assets under `loadpath_root` and binds the policy selector launcher.
    pub fn new(program: impl Into<PathBuf>, loadpath_root: impl Into<PathBuf>) -> io::Result<Self> {
        let loadpath_root = loadpath_root.into();
        let written_assets = write_gerbil_runtime_assets(&loadpath_root)?;
        let evaluator =
            GerbilDeckRuntimeModelRoutePolicyEvaluator::from_marlin_deck_runtime_policy_launcher(
                program,
                loadpath_root.clone(),
            );
        Ok(Self {
            loadpath_root,
            written_assets,
            evaluator,
        })
    }

    /// Writes runtime assets and binds the default `gxi` executable.
    pub fn from_default_gxi(loadpath_root: impl Into<PathBuf>) -> io::Result<Self> {
        Self::new(default_gerbil_gxi_program(), loadpath_root)
    }

    pub fn loadpath_root(&self) -> &Path {
        &self.loadpath_root
    }

    pub fn written_assets(&self) -> &[PathBuf] {
        &self.written_assets
    }

    pub fn evaluator(&self) -> &GerbilDeckRuntimeModelRoutePolicyEvaluator {
        &self.evaluator
    }

    pub fn spec(&self) -> &GerbilCommandSpec {
        self.evaluator.spec()
    }

    pub fn into_evaluator(self) -> GerbilDeckRuntimeModelRoutePolicyEvaluator {
        self.evaluator
    }
}

/// Error raised while evaluating a Deck runtime model-route policy selector.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GerbilDeckRuntimeModelRoutePolicyError {
    Encode { message: String },
    Command { message: String },
    Decode { message: String },
}

/// Decode a Scheme selection JSON receipt.
pub fn decode_gerbil_deck_runtime_model_route_selection(
    output_json: &str,
) -> Result<GerbilDeckRuntimeModelRouteSelectionReceipt, GerbilDeckRuntimeModelRoutePolicyError> {
    serde_json::from_str(output_json).map_err(|error| {
        GerbilDeckRuntimeModelRoutePolicyError::Decode {
            message: error.to_string(),
        }
    })
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

fn terminate_deck_runtime_policy_child_after_start_failure(child: &mut std::process::Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn validate_deck_runtime_policy_command_output(
    output: std::process::Output,
) -> Result<std::process::Output, GerbilDeckRuntimeModelRoutePolicyError> {
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = stderr.trim();
        let stdout = stdout.trim();
        let diagnostics = match (stderr.is_empty(), stdout.is_empty()) {
            (false, false) => format!("stderr: {stderr}\nstdout: {stdout}"),
            (false, true) => stderr.to_string(),
            (true, false) => stdout.to_string(),
            (true, true) => String::new(),
        };
        return Err(GerbilDeckRuntimeModelRoutePolicyError::Command {
            message: format!(
                "deck runtime policy command failed with status {}: {}",
                output.status, diagnostics
            ),
        });
    }

    Ok(output)
}

impl Display for GerbilDeckRuntimeModelRoutePolicyError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Encode { message } => {
                write!(
                    formatter,
                    "failed to encode deck runtime policy request JSON: {message}"
                )
            }
            Self::Command { message } => {
                write!(
                    formatter,
                    "failed to evaluate deck runtime policy command: {message}"
                )
            }
            Self::Decode { message } => {
                write!(
                    formatter,
                    "failed to decode deck runtime policy result JSON: {message}"
                )
            }
        }
    }
}

impl Error for GerbilDeckRuntimeModelRoutePolicyError {}
