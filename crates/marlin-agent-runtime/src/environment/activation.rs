// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Applies runtime environment activation policy and produces audit receipts.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path, PathBuf},
    time::Duration,
};

use async_trait::async_trait;
use marlin_agent_protocol::{
    RuntimeEnvironment, RuntimeEnvironmentActivation, RuntimeEnvironmentActivationAction,
    RuntimeEnvironmentActivationActionReceipt, RuntimeEnvironmentActivationPolicy,
    RuntimeEnvironmentActivationReceipt, RuntimeEnvironmentDelta, RuntimeEnvrcPolicy,
    RuntimeShellIsolationPolicy,
};
use serde_json::Value;
use thiserror::Error;
use tokio::{io::AsyncReadExt, process::Command};

/// Maximum byte size accepted from `direnv` stdout / JSON export (1 MiB).
pub const MAX_DIRENV_JSON_BYTES: usize = 1024 * 1024;

/// Maximum environment variable entries accepted from a single `direnv export json` payload.
pub const MAX_DIRENV_ENV_ENTRIES: usize = 4096;

const DIRENV_COMMAND_TIMEOUT: Duration = Duration::from_secs(30);

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
pub trait DirenvCommandRunner: Send + Sync {
    async fn allow(
        &self,
        _cwd: &Path,
        _environment: &BTreeMap<String, String>,
    ) -> Result<(), RuntimeEnvironmentActivationError> {
        Ok(())
    }

    async fn reload(
        &self,
        _cwd: &Path,
        _environment: &BTreeMap<String, String>,
    ) -> Result<(), RuntimeEnvironmentActivationError> {
        Ok(())
    }

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
    async fn allow(
        &self,
        cwd: &Path,
        environment: &BTreeMap<String, String>,
    ) -> Result<(), RuntimeEnvironmentActivationError> {
        run_direnv_command(cwd, environment, &["allow", "./.envrc"]).await
    }

    async fn reload(
        &self,
        cwd: &Path,
        environment: &BTreeMap<String, String>,
    ) -> Result<(), RuntimeEnvironmentActivationError> {
        run_direnv_command(cwd, environment, &["reload"]).await
    }

    async fn export_json(
        &self,
        cwd: &Path,
        environment: &BTreeMap<String, String>,
    ) -> Result<String, RuntimeEnvironmentActivationError> {
        let output = direnv_output(cwd, environment, &["export", "json"]).await?;
        String::from_utf8(output.stdout).map_err(|error| {
            RuntimeEnvironmentActivationError::CommandOutput {
                message: error.to_string(),
            }
        })
    }
}

impl ProcessDirenvCommandRunner {
    /// Change authorization for the exact worktree file, without parent discovery.
    pub async fn authorize_project(
        &self,
        cwd: &Path,
        environment: &BTreeMap<String, String>,
        allow: bool,
    ) -> Result<(), RuntimeEnvironmentActivationError> {
        let action = if allow { "allow" } else { "deny" };
        run_direnv_command(cwd, environment, &[action, "./.envrc"]).await
    }

    /// Check that direnv selected this worktree's file and approved its contents.
    pub async fn project_allowed(
        &self,
        cwd: &Path,
        environment: &BTreeMap<String, String>,
    ) -> Result<bool, RuntimeEnvironmentActivationError> {
        let output = direnv_output(cwd, environment, &["status", "--json"]).await?;
        let status: Value = serde_json::from_slice(&output.stdout)
            .map_err(|_| RuntimeEnvironmentActivationError::InvalidJsonShape)?;
        let rc = &status["state"]["foundRC"];
        Ok(rc["allowed"].as_u64() == Some(0)
            && rc["path"].as_str().map(Path::new) == Some(cwd.join(".envrc").as_path()))
    }
}

async fn run_direnv_command(
    cwd: &Path,
    environment: &BTreeMap<String, String>,
    args: &[&str],
) -> Result<(), RuntimeEnvironmentActivationError> {
    direnv_output(cwd, environment, args).await?;
    Ok(())
}

async fn direnv_output(
    cwd: &Path,
    environment: &BTreeMap<String, String>,
    args: &[&str],
) -> Result<std::process::Output, RuntimeEnvironmentActivationError> {
    use std::process::Stdio;

    let mut command = Command::new("direnv");
    command
        .current_dir(cwd)
        .env_clear()
        .envs(environment)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    #[cfg(unix)]
    command.process_group(0);
    let mut child = command.spawn().map_err(direnv_io_error)?;
    // Dropping an in-flight load has to stop the shell running .envrc as well
    // as direnv itself. Keep the group guard alive while draining the output.
    #[cfg(unix)]
    let mut group = DirenvProcessGroup(child.id().and_then(|id| {
        i32::try_from(id)
            .ok()
            .and_then(rustix::process::Pid::from_raw)
    }));
    let mut stdout_pipe = child
        .stdout
        .take()
        .ok_or_else(|| RuntimeEnvironmentActivationError::CommandOutput {
            message: "direnv stdout pipe unavailable".to_owned(),
        })?;
    let bounded_run = tokio::time::timeout(DIRENV_COMMAND_TIMEOUT, async {
        let mut stdout = Vec::new();
        let mut limited = (&mut stdout_pipe).take((MAX_DIRENV_JSON_BYTES + 1) as u64);
        limited
            .read_to_end(&mut stdout)
            .await
            .map_err(direnv_io_error)?;
        if stdout.len() > MAX_DIRENV_JSON_BYTES {
            let _ = child.kill().await;
            return Err(RuntimeEnvironmentActivationError::OutputTooLarge {
                limit: MAX_DIRENV_JSON_BYTES,
            });
        }
        let status = child.wait().await.map_err(direnv_io_error)?;
        Ok(std::process::Output {
            status,
            stdout,
            stderr: Vec::new(),
        })
    })
    .await;
    let output = match bounded_run {
        Ok(result) => result?,
        Err(_) => {
            let _ = child.kill().await;
            return Err(RuntimeEnvironmentActivationError::CommandTimedOut);
        }
    };
    #[cfg(unix)]
    {
        group.0 = None;
    }
    if !output.status.success() {
        return Err(RuntimeEnvironmentActivationError::CommandFailed {
            status: output.status.code(),
        });
    }
    Ok(output)
}

#[cfg(unix)]
struct DirenvProcessGroup(Option<rustix::process::Pid>);

#[cfg(unix)]
impl Drop for DirenvProcessGroup {
    fn drop(&mut self) {
        if let Some(group) = self.0 {
            let _ = rustix::process::kill_process_group(group, rustix::process::Signal::KILL);
        }
    }
}

fn direnv_io_error(error: std::io::Error) -> RuntimeEnvironmentActivationError {
    RuntimeEnvironmentActivationError::CommandIo {
        message: error.to_string(),
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
        let mut actions = Vec::new();

        for action in &policy.preflight_actions {
            let result = match action {
                RuntimeEnvironmentActivationAction::DirenvReload => {
                    self.runner.reload(&cwd, &command_environment).await
                }
                RuntimeEnvironmentActivationAction::DirenvExportJson => Ok(()),
            };
            if let Err(error) = result {
                actions.push(RuntimeEnvironmentActivationActionReceipt::rejected(
                    action.clone(),
                    error.to_string(),
                ));
                return rejected_with_actions(request.base_environment, policy, error, actions);
            }
            actions.push(RuntimeEnvironmentActivationActionReceipt::applied(
                action.clone(),
            ));
        }

        let json = match self.runner.export_json(&cwd, &command_environment).await {
            Ok(json) => {
                actions.push(RuntimeEnvironmentActivationActionReceipt::applied(
                    RuntimeEnvironmentActivationAction::DirenvExportJson,
                ));
                json
            }
            Err(error) => {
                actions.push(RuntimeEnvironmentActivationActionReceipt::rejected(
                    RuntimeEnvironmentActivationAction::DirenvExportJson,
                    error.to_string(),
                ));
                return rejected_with_actions(request.base_environment, policy, error, actions);
            }
        };
        let activated = match apply_direnv_json(&request.base_environment, &policy.shell, &json) {
            Ok(activated) => activated,
            Err(error) => {
                return rejected_with_actions(request.base_environment, policy, error, actions);
            }
        };
        let delta = if capture_delta {
            RuntimeEnvironmentDelta::from_snapshots(&request.base_environment, &activated)
        } else {
            RuntimeEnvironmentDelta::default()
        };

        RuntimeEnvironmentActivationResult {
            environment: activated,
            receipt: RuntimeEnvironmentActivationReceipt::applied_with_actions(
                policy, delta, actions,
            ),
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
    #[error("unsafe envrc path escapes workspace root or traverses symlink: {}", path.display())]
    UnsafeEnvrcPath { path: PathBuf },
    #[error("direnv command timed out")]
    CommandTimedOut,
    #[error("direnv output exceeded maximum size of {limit} bytes")]
    OutputTooLarge { limit: usize },
    #[error("direnv exported more than {limit} environment entries")]
    TooManyEnvEntries { limit: usize },
    #[error("direnv exported disallowed or malformed environment variable `{name}`")]
    DisallowedEnvKey { name: String },
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

fn rejected_with_actions(
    environment: BTreeMap<String, String>,
    policy: &RuntimeEnvironmentActivationPolicy,
    error: RuntimeEnvironmentActivationError,
    actions: Vec<RuntimeEnvironmentActivationActionReceipt>,
) -> RuntimeEnvironmentActivationResult {
    RuntimeEnvironmentActivationResult {
        environment,
        receipt: RuntimeEnvironmentActivationReceipt::rejected_with_actions(
            policy,
            error.to_string(),
            actions,
        ),
    }
}

pub(crate) fn direnv_cwd(
    environment: &RuntimeEnvironment,
    envrc: &RuntimeEnvrcPolicy,
) -> Result<PathBuf, RuntimeEnvironmentActivationError> {
    match envrc {
        RuntimeEnvrcPolicy::Project => {
            let cwd = environment
                .cwd
                .clone()
                .ok_or(RuntimeEnvironmentActivationError::MissingCwd)?;
            if cwd
                .components()
                .any(|component| matches!(component, Component::ParentDir))
            {
                return Err(RuntimeEnvironmentActivationError::UnsafeEnvrcPath { path: cwd });
            }
            reject_symlink_envrc(&cwd.join(".envrc"))?;
            Ok(cwd)
        }
        RuntimeEnvrcPolicy::Explicit { file } => {
            confine_explicit_envrc(environment.cwd.as_deref(), file)
        }
    }
}

fn confine_explicit_envrc(
    base_cwd: Option<&Path>,
    file: &Path,
) -> Result<PathBuf, RuntimeEnvironmentActivationError> {
    if file.file_name().and_then(|name| name.to_str()) != Some(".envrc") {
        return Err(
            RuntimeEnvironmentActivationError::UnsupportedExplicitEnvrc {
                file: file.to_path_buf(),
            },
        );
    }
    if file
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(RuntimeEnvironmentActivationError::UnsafeEnvrcPath {
            path: file.to_path_buf(),
        });
    }
    let parent = match file.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.to_path_buf(),
        _ => {
            return Err(
                RuntimeEnvironmentActivationError::ExplicitEnvrcMissingParent {
                    file: file.to_path_buf(),
                },
            );
        }
    };
    if let Some(base_cwd) = base_cwd {
        if base_cwd
            .components()
            .any(|component| matches!(component, Component::ParentDir))
        {
            return Err(RuntimeEnvironmentActivationError::UnsafeEnvrcPath {
                path: base_cwd.to_path_buf(),
            });
        }
        if file.is_absolute() && base_cwd.is_absolute() && !file.starts_with(base_cwd) {
            return Err(RuntimeEnvironmentActivationError::UnsafeEnvrcPath {
                path: file.to_path_buf(),
            });
        }
        if let (Ok(canon_base), Ok(canon_parent)) =
            (fs::canonicalize(base_cwd), fs::canonicalize(&parent))
            && !canon_parent.starts_with(&canon_base)
        {
            return Err(RuntimeEnvironmentActivationError::UnsafeEnvrcPath {
                path: file.to_path_buf(),
            });
        }
    }
    reject_symlink_envrc(file)?;
    Ok(parent)
}

fn reject_symlink_envrc(path: &Path) -> Result<(), RuntimeEnvironmentActivationError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            Err(RuntimeEnvironmentActivationError::UnsafeEnvrcPath {
                path: path.to_path_buf(),
            })
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(direnv_io_error(error)),
    }
}

pub(crate) fn command_environment(
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

fn is_disallowed_direnv_export_key(name: &str) -> bool {
    const DISALLOWED_LOADER_AND_SHELL_KEYS: &[&str] = &[
        "LD_PRELOAD",
        "LD_LIBRARY_PATH",
        "LD_AUDIT",
        "LD_DEBUG",
        "DYLD_INSERT_LIBRARIES",
        "DYLD_LIBRARY_PATH",
        "DYLD_FRAMEWORK_PATH",
        "DYLD_FALLBACK_LIBRARY_PATH",
        "BASH_ENV",
        "ENV",
        "PROMPT_COMMAND",
        "IFS",
        "GCONV_PATH",
        "LOCPATH",
        "PYTHONSTARTUP",
        "NODE_OPTIONS",
        "PERL5OPT",
        "RUBYOPT",
    ];
    DISALLOWED_LOADER_AND_SHELL_KEYS
        .iter()
        .any(|blocked| name.eq_ignore_ascii_case(blocked))
}

fn apply_direnv_json(
    base: &BTreeMap<String, String>,
    shell: &RuntimeShellIsolationPolicy,
    json: &str,
) -> Result<BTreeMap<String, String>, RuntimeEnvironmentActivationError> {
    if json.len() > MAX_DIRENV_JSON_BYTES {
        return Err(RuntimeEnvironmentActivationError::OutputTooLarge {
            limit: MAX_DIRENV_JSON_BYTES,
        });
    }
    let value = serde_json::from_str::<Value>(json).map_err(|error| {
        RuntimeEnvironmentActivationError::InvalidJson {
            message: error.to_string(),
        }
    })?;
    let object = value
        .as_object()
        .ok_or(RuntimeEnvironmentActivationError::InvalidJsonShape)?;
    if object.len() > MAX_DIRENV_ENV_ENTRIES {
        return Err(RuntimeEnvironmentActivationError::TooManyEnvEntries {
            limit: MAX_DIRENV_ENV_ENTRIES,
        });
    }
    let denylist = shell
        .denylist
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let mut environment = base.clone();

    for (name, value) in object {
        if name.is_empty()
            || name.contains('=')
            || name.chars().any(char::is_control)
        {
            return Err(RuntimeEnvironmentActivationError::DisallowedEnvKey {
                name: name.clone(),
            });
        }
        match value {
            Value::Null => {
                environment.remove(name);
            }
            Value::String(value) => {
                if value.contains('\0') {
                    return Err(RuntimeEnvironmentActivationError::InvalidEnvValue {
                        name: name.clone(),
                    });
                }
                if denylist.contains(name.as_str()) || is_disallowed_direnv_export_key(name) {
                    return Err(RuntimeEnvironmentActivationError::DisallowedEnvKey {
                        name: name.clone(),
                    });
                }
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
