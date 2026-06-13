//! Typed `Gerbil Scheme` boundary for complex hook policy extensions.

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    io::{self, Write},
    path::{Path, PathBuf},
    process::{Command as StdCommand, Stdio},
};

use crate::command::GerbilCommandSpec;
use crate::runtime::{default_gerbil_gxi_program, write_gerbil_runtime_assets};
use marlin_agent_protocol::{
    HookAgentScope, HookDispatchPolicyReceipt, HookEventName, HookPolicyDecision,
    HookPolicyDynamicAction, HookPolicyExtension, HookPolicyExtensionKind, HookSchemeModule,
    HookSchemeProcedure,
};
use serde::{Deserialize, Serialize};

/// Named input for building a `Gerbil Scheme` hook policy invocation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilHookPolicyInvocationInput {
    pub extension: HookPolicyExtension,
    pub event_name: HookEventName,
    pub agent_scope: HookAgentScope,
    pub policy_receipt: HookDispatchPolicyReceipt,
}

/// Prepared `Gerbil Scheme` policy invocation for a hook dispatch.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilHookPolicyInvocation {
    pub module: HookSchemeModule,
    pub procedure: HookSchemeProcedure,
    pub request_json: String,
}

/// Error raised while preparing a `Gerbil Scheme` hook policy invocation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GerbilHookPolicyInvocationError {
    UnsupportedExtension {
        kind: HookPolicyExtensionKind,
    },
    AgentScopeMismatch {
        input: HookAgentScope,
        receipt: HookAgentScope,
    },
    MissingModule,
    MissingProcedure,
    Encode {
        message: String,
    },
}

/// Named input for decoding a `Gerbil Scheme` hook policy result.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilHookPolicyEvaluationDecodeInput {
    pub invocation: GerbilHookPolicyInvocationInput,
    pub output_json: String,
}

/// Typed output shape returned by a `Gerbil Scheme` hook policy procedure.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilHookPolicyEvaluationOutput {
    pub decision: HookPolicyDecision,
    #[serde(default)]
    pub diagnostics: Vec<GerbilHookPolicyDiagnostic>,
    #[serde(default)]
    pub actions: Vec<HookPolicyDynamicAction>,
}

/// Diagnostic emitted by a `Gerbil Scheme` hook policy procedure.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilHookPolicyDiagnostic {
    pub message: String,
}

/// Receipt produced after decoding a `Gerbil Scheme` hook policy result.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilHookPolicyEvaluationReceipt {
    pub event_name: HookEventName,
    pub agent_scope: HookAgentScope,
    pub extension: HookPolicyExtension,
    pub decision: HookPolicyDecision,
    pub diagnostics: Vec<GerbilHookPolicyDiagnostic>,
    pub actions: Vec<HookPolicyDynamicAction>,
    pub policy_evaluated_count: usize,
    pub policy_rejected_count: usize,
}

impl GerbilHookPolicyEvaluationReceipt {
    /// Returns true when the decoded policy decision allows the dispatch.
    pub fn is_allowed(&self) -> bool {
        self.decision == HookPolicyDecision::Allowed
    }
}

/// Command-backed evaluator for `Gerbil Scheme` hook policy procedures.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilHookPolicyCommandEvaluator {
    spec: GerbilCommandSpec,
}

impl GerbilHookPolicyCommandEvaluator {
    pub fn new(spec: GerbilCommandSpec) -> Self {
        Self { spec }
    }

    /// Builds an evaluator for the crate-shipped `hook-policy-adapter.ss` launcher.
    pub fn from_marlin_hook_policy_launcher(
        program: impl Into<PathBuf>,
        loadpath_root: impl Into<PathBuf>,
    ) -> Self {
        Self::new(GerbilCommandSpec::marlin_hook_policy_launcher(
            program,
            loadpath_root,
        ))
    }

    pub fn spec(&self) -> &GerbilCommandSpec {
        &self.spec
    }

    /// Evaluate a hook policy by sending a typed invocation packet to a command.
    pub fn evaluate(
        &self,
        input: GerbilHookPolicyInvocationInput,
    ) -> Result<GerbilHookPolicyEvaluationReceipt, GerbilHookPolicyEvaluationError> {
        let invocation = build_gerbil_hook_policy_invocation(input.clone())
            .map_err(|source| GerbilHookPolicyEvaluationError::Invocation { source })?;
        let mut request_json = serde_json::to_vec(&invocation).map_err(|error| {
            GerbilHookPolicyEvaluationError::Encode {
                message: error.to_string(),
            }
        })?;
        request_json.push(b'\n');

        let output = self.run_command_with_stdin(request_json)?;
        let output_json = String::from_utf8(output.stdout).map_err(|error| {
            GerbilHookPolicyEvaluationError::Decode {
                message: format!("hook policy command stdout is not UTF-8: {error}"),
            }
        })?;

        decode_gerbil_hook_policy_evaluation(GerbilHookPolicyEvaluationDecodeInput {
            invocation: input,
            output_json: output_json.trim().to_owned(),
        })
    }

    fn run_command_with_stdin(
        &self,
        stdin_bytes: Vec<u8>,
    ) -> Result<std::process::Output, GerbilHookPolicyEvaluationError> {
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
                .map_err(|error| GerbilHookPolicyEvaluationError::Command {
                    message: format!("failed to start gerbil hook policy command: {error}"),
                })?;

        {
            let mut stdin =
                child
                    .stdin
                    .take()
                    .ok_or_else(|| GerbilHookPolicyEvaluationError::Command {
                        message: "gerbil hook policy command did not expose stdin".to_string(),
                    })?;
            if let Err(error) = stdin.write_all(&stdin_bytes)
                && error.kind() != io::ErrorKind::BrokenPipe
            {
                terminate_hook_policy_child_after_start_failure(&mut child);
                return Err(GerbilHookPolicyEvaluationError::Command {
                    message: format!("failed to write gerbil hook policy request: {error}"),
                });
            }
        }

        let output =
            child
                .wait_with_output()
                .map_err(|error| GerbilHookPolicyEvaluationError::Command {
                    message: format!("failed to read gerbil hook policy command output: {error}"),
                })?;

        validate_gerbil_hook_policy_command_output(output)
    }
}

/// One-stop Rust binding for the crate-shipped `Gerbil` hook policy adapter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilHookPolicyRuntimeBinding {
    loadpath_root: PathBuf,
    written_assets: Vec<PathBuf>,
    evaluator: GerbilHookPolicyCommandEvaluator,
}

impl GerbilHookPolicyRuntimeBinding {
    /// Writes runtime assets under `loadpath_root` and binds the hook policy launcher.
    pub fn new(program: impl Into<PathBuf>, loadpath_root: impl Into<PathBuf>) -> io::Result<Self> {
        let loadpath_root = loadpath_root.into();
        let written_assets = write_gerbil_runtime_assets(&loadpath_root)?;
        let evaluator = GerbilHookPolicyCommandEvaluator::from_marlin_hook_policy_launcher(
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

    pub fn evaluator(&self) -> &GerbilHookPolicyCommandEvaluator {
        &self.evaluator
    }

    pub fn spec(&self) -> &GerbilCommandSpec {
        self.evaluator.spec()
    }

    pub fn into_evaluator(self) -> GerbilHookPolicyCommandEvaluator {
        self.evaluator
    }
}

/// Error raised while decoding a `Gerbil Scheme` hook policy result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GerbilHookPolicyEvaluationError {
    Invocation {
        source: GerbilHookPolicyInvocationError,
    },
    Encode {
        message: String,
    },
    Command {
        message: String,
    },
    Decode {
        message: String,
    },
}

/// Builds a typed `Gerbil Scheme` invocation from a hook policy extension.
pub fn build_gerbil_hook_policy_invocation(
    input: GerbilHookPolicyInvocationInput,
) -> Result<GerbilHookPolicyInvocation, GerbilHookPolicyInvocationError> {
    validate_gerbil_hook_policy_invocation_input(&input)?;

    let module = input
        .extension
        .module
        .clone()
        .ok_or(GerbilHookPolicyInvocationError::MissingModule)?;
    let procedure = input
        .extension
        .procedure
        .clone()
        .ok_or(GerbilHookPolicyInvocationError::MissingProcedure)?;
    let request_json =
        serde_json::to_string(&input).map_err(|error| GerbilHookPolicyInvocationError::Encode {
            message: error.to_string(),
        })?;

    Ok(GerbilHookPolicyInvocation {
        module,
        procedure,
        request_json,
    })
}

/// Decodes a `Gerbil Scheme` hook policy result into a typed receipt.
pub fn decode_gerbil_hook_policy_evaluation(
    input: GerbilHookPolicyEvaluationDecodeInput,
) -> Result<GerbilHookPolicyEvaluationReceipt, GerbilHookPolicyEvaluationError> {
    validate_gerbil_hook_policy_invocation_input(&input.invocation)
        .map_err(|source| GerbilHookPolicyEvaluationError::Invocation { source })?;
    let output = serde_json::from_str::<GerbilHookPolicyEvaluationOutput>(&input.output_json)
        .map_err(|error| GerbilHookPolicyEvaluationError::Decode {
            message: error.to_string(),
        })?;

    Ok(GerbilHookPolicyEvaluationReceipt {
        event_name: input.invocation.event_name,
        agent_scope: input.invocation.agent_scope,
        extension: input.invocation.extension,
        decision: output.decision,
        diagnostics: output.diagnostics,
        actions: output.actions,
        policy_evaluated_count: input.invocation.policy_receipt.evaluated_count,
        policy_rejected_count: input.invocation.policy_receipt.rejected_count,
    })
}

fn validate_gerbil_hook_policy_invocation_input(
    input: &GerbilHookPolicyInvocationInput,
) -> Result<(), GerbilHookPolicyInvocationError> {
    if input.extension.kind != HookPolicyExtensionKind::GerbilScheme {
        return Err(GerbilHookPolicyInvocationError::UnsupportedExtension {
            kind: input.extension.kind.clone(),
        });
    }
    if input.agent_scope != input.policy_receipt.invocation_agent_scope {
        return Err(GerbilHookPolicyInvocationError::AgentScopeMismatch {
            input: input.agent_scope.clone(),
            receipt: input.policy_receipt.invocation_agent_scope.clone(),
        });
    }

    Ok(())
}

fn terminate_hook_policy_child_after_start_failure(child: &mut std::process::Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn validate_gerbil_hook_policy_command_output(
    output: std::process::Output,
) -> Result<std::process::Output, GerbilHookPolicyEvaluationError> {
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
        return Err(GerbilHookPolicyEvaluationError::Command {
            message: format!(
                "gerbil hook policy command failed with status {}: {}",
                output.status, diagnostics
            ),
        });
    }

    Ok(output)
}

impl Display for GerbilHookPolicyInvocationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedExtension { kind } => {
                write!(formatter, "unsupported hook policy extension `{kind:?}`")
            }
            Self::AgentScopeMismatch { input, receipt } => write!(
                formatter,
                "hook policy input scope `{input:?}` does not match receipt scope `{receipt:?}`"
            ),
            Self::MissingModule => write!(formatter, "missing `Gerbil Scheme` hook policy module"),
            Self::MissingProcedure => {
                write!(formatter, "missing `Gerbil Scheme` hook policy procedure")
            }
            Self::Encode { message } => {
                write!(
                    formatter,
                    "failed to encode hook policy request JSON: {message}"
                )
            }
        }
    }
}

impl Error for GerbilHookPolicyInvocationError {}

impl Display for GerbilHookPolicyEvaluationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invocation { source } => {
                write!(formatter, "invalid hook policy invocation: {source}")
            }
            Self::Encode { message } => {
                write!(
                    formatter,
                    "failed to encode hook policy command request JSON: {message}"
                )
            }
            Self::Command { message } => {
                write!(
                    formatter,
                    "failed to evaluate hook policy command: {message}"
                )
            }
            Self::Decode { message } => {
                write!(
                    formatter,
                    "failed to decode hook policy result JSON: {message}"
                )
            }
        }
    }
}

impl Error for GerbilHookPolicyEvaluationError {}
