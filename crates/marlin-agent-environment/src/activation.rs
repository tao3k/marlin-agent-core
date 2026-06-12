//! Applies runtime environment activation policy and produces audit receipts.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

use async_trait::async_trait;
use marlin_agent_protocol::{
    RuntimeEnvironment, RuntimeEnvironmentActivation, RuntimeEnvironmentActivationPolicy,
    RuntimeEnvironmentActivationReceipt, RuntimeEnvironmentDelta, RuntimeEnvrcPolicy,
    RuntimeShellIsolationPolicy,
};
use serde_json::Value;
use thiserror::Error;
use tokio::process::Command;

/// Input used to apply a runtime environment activation policy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeEnvironmentActivationRequest {
    pub environment: RuntimeEnvironment,
    pub base_environment: BTreeMap<String, String>,
}

impl RuntimeEnvironmentActivationRequest {
    pub fn new(
        environment: RuntimeEnvironment,
        base_environment: BTreeMap<String, String>,
    ) -> Self {
        Self {
            environment,
            base_environment,
        }
    }
}

/// Environment variables plus the receipt emitted by activation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeEnvironmentActivationResult {
    pub environment: BTreeMap<String, String>,
    pub receipt: RuntimeEnvironmentActivationReceipt,
}

/// Runner used by the activator to execute `direnv export json`.
#[async_trait]
pub trait DirenvCommandRunner {
    async fn export_json(
        &self,
        cwd: &Path,
        environment: &BTreeMap<String, String>,
    ) -> Result<String, RuntimeEnvironmentActivationError>;
}

/// Production command runner backed by `tokio::process::Command`.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProcessDirenvCommandRunner;

#[async_trait]
impl DirenvCommandRunner for ProcessDirenvCommandRunner {
    async fn export_json(
        &self,
        cwd: &Path,
        environment: &BTreeMap<String, String>,
    ) -> Result<String, RuntimeEnvironmentActivationError> {
        let output = Command::new("direnv")
            .args(["export", "json"])
            .current_dir(cwd)
            .env_clear()
            .envs(environment)
            .output()
            .await
            .map_err(|error| RuntimeEnvironmentActivationError::CommandIo {
                message: error.to_string(),
            })?;

        if !output.status.success() {
            return Err(RuntimeEnvironmentActivationError::CommandFailed {
                status: output.status.code(),
            });
        }

        String::from_utf8(output.stdout).map_err(|error| {
            RuntimeEnvironmentActivationError::CommandOutput {
                message: error.to_string(),
            }
        })
    }
}

/// Applies activation policy with an injectable command runner.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeEnvironmentActivator<R = ProcessDirenvCommandRunner> {
    runner: R,
}

impl RuntimeEnvironmentActivator<ProcessDirenvCommandRunner> {
    pub fn new() -> Self {
        Self {
            runner: ProcessDirenvCommandRunner,
        }
    }
}

impl Default for RuntimeEnvironmentActivator<ProcessDirenvCommandRunner> {
    fn default() -> Self {
        Self::new()
    }
}

impl<R> RuntimeEnvironmentActivator<R>
where
    R: DirenvCommandRunner,
{
    pub fn with_runner(runner: R) -> Self {
        Self { runner }
    }

    pub async fn activate(
        &self,
        request: RuntimeEnvironmentActivationRequest,
    ) -> RuntimeEnvironmentActivationResult {
        let activation = request.environment.activation.activation.clone();

        match activation {
            RuntimeEnvironmentActivation::Disabled => RuntimeEnvironmentActivationResult {
                environment: request.base_environment,
                receipt: RuntimeEnvironmentActivationReceipt::disabled(
                    &request.environment.activation,
                ),
            },
            RuntimeEnvironmentActivation::Direnv {
                capture_delta,
                envrc,
            } => self.activate_direnv(request, &envrc, capture_delta).await,
        }
    }

    async fn activate_direnv(
        &self,
        request: RuntimeEnvironmentActivationRequest,
        envrc: &RuntimeEnvrcPolicy,
        capture_delta: bool,
    ) -> RuntimeEnvironmentActivationResult {
        let policy = &request.environment.activation;
        let cwd = match direnv_cwd(&request.environment, envrc) {
            Ok(cwd) => cwd,
            Err(error) => return rejected(request.base_environment, policy, error),
        };
        let command_environment = command_environment(&request.base_environment, &policy.shell);

        let json = match self.runner.export_json(&cwd, &command_environment).await {
            Ok(json) => json,
            Err(error) => return rejected(request.base_environment, policy, error),
        };
        let activated = match apply_direnv_json(&request.base_environment, &json) {
            Ok(activated) => activated,
            Err(error) => return rejected(request.base_environment, policy, error),
        };
        let delta = if capture_delta {
            RuntimeEnvironmentDelta::from_snapshots(&request.base_environment, &activated)
        } else {
            RuntimeEnvironmentDelta::default()
        };

        RuntimeEnvironmentActivationResult {
            environment: activated,
            receipt: RuntimeEnvironmentActivationReceipt::applied(policy, delta),
        }
    }
}

/// Error produced while applying environment activation.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum RuntimeEnvironmentActivationError {
    #[error("runtime environment activation requires cwd")]
    MissingCwd,
    #[error("explicit envrc must point to a .envrc file")]
    UnsupportedExplicitEnvrc { file: PathBuf },
    #[error("explicit envrc file has no parent directory")]
    ExplicitEnvrcMissingParent { file: PathBuf },
    #[error("direnv command failed with status {status:?}")]
    CommandFailed { status: Option<i32> },
    #[error("direnv command could not run: {message}")]
    CommandIo { message: String },
    #[error("direnv command output is not utf8: {message}")]
    CommandOutput { message: String },
    #[error("direnv export json was invalid: {message}")]
    InvalidJson { message: String },
    #[error("direnv export json must be an object")]
    InvalidJsonShape,
    #[error("direnv exported non-string value for {name}")]
    InvalidEnvValue { name: String },
}

fn rejected(
    environment: BTreeMap<String, String>,
    policy: &RuntimeEnvironmentActivationPolicy,
    error: RuntimeEnvironmentActivationError,
) -> RuntimeEnvironmentActivationResult {
    RuntimeEnvironmentActivationResult {
        environment,
        receipt: RuntimeEnvironmentActivationReceipt::rejected(policy, error.to_string()),
    }
}

fn direnv_cwd(
    environment: &RuntimeEnvironment,
    envrc: &RuntimeEnvrcPolicy,
) -> Result<PathBuf, RuntimeEnvironmentActivationError> {
    match envrc {
        RuntimeEnvrcPolicy::Project => environment
            .cwd
            .clone()
            .ok_or(RuntimeEnvironmentActivationError::MissingCwd),
        RuntimeEnvrcPolicy::Explicit { file } => {
            if file.file_name().and_then(|name| name.to_str()) != Some(".envrc") {
                return Err(
                    RuntimeEnvironmentActivationError::UnsupportedExplicitEnvrc {
                        file: file.clone(),
                    },
                );
            }
            file.parent().map(Path::to_path_buf).ok_or_else(|| {
                RuntimeEnvironmentActivationError::ExplicitEnvrcMissingParent { file: file.clone() }
            })
        }
    }
}

fn command_environment(
    base: &BTreeMap<String, String>,
    shell: &RuntimeShellIsolationPolicy,
) -> BTreeMap<String, String> {
    let denylist = shell
        .denylist
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();

    if shell.isolate_host_environment {
        return shell
            .allowlist
            .iter()
            .filter(|name| !denylist.contains(name.as_str()))
            .filter_map(|name| base.get(name).map(|value| (name.clone(), value.clone())))
            .collect();
    }

    base.iter()
        .filter(|(name, _)| !denylist.contains(name.as_str()))
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect()
}

fn apply_direnv_json(
    base: &BTreeMap<String, String>,
    json: &str,
) -> Result<BTreeMap<String, String>, RuntimeEnvironmentActivationError> {
    let value = serde_json::from_str::<Value>(json).map_err(|error| {
        RuntimeEnvironmentActivationError::InvalidJson {
            message: error.to_string(),
        }
    })?;
    let object = value
        .as_object()
        .ok_or(RuntimeEnvironmentActivationError::InvalidJsonShape)?;
    let mut environment = base.clone();

    for (name, value) in object {
        match value {
            Value::Null => {
                environment.remove(name);
            }
            Value::String(value) => {
                environment.insert(name.clone(), value.clone());
            }
            _ => {
                return Err(RuntimeEnvironmentActivationError::InvalidEnvValue {
                    name: name.clone(),
                });
            }
        }
    }

    Ok(environment)
}
