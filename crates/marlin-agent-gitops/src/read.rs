// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Bounded execution for Git queries and checked GitOps effects.
use marlin_agent_protocol::{
    BoundedProcessExecutor, BoundedProcessOutput, BoundedProcessRequest, ProcessExecutionError,
};
use std::{ffi::OsString, future::Future, path::Path, pin::Pin, process::Stdio, time::Duration};
use thiserror::Error;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    process::Command,
};

pub(crate) const MAX_OUTPUT_BYTES: usize = 1024 * 1024;
const TIMEOUT: Duration = Duration::from_secs(10);
pub(crate) const NULL_DEVICE: &str = if cfg!(windows) { "NUL" } else { "/dev/null" };
/// Process-local Git configuration for operations that must not run repository hooks.
pub const DISABLED_HOOKS: &str = if cfg!(windows) {
    "core.hooksPath=NUL"
} else {
    "core.hooksPath=/dev/null"
};
/// Process-local Git configuration disabling repository-configured fsmonitor helpers.
pub const DISABLED_FSMONITOR: &str = "core.fsmonitor=false";

/// Inherited Git environment that can redirect repository, index, config, or command execution.
pub const GIT_REPOSITORY_OVERRIDES: &[&str] = &[
    "GIT_DIR",
    "GIT_COMMON_DIR",
    "GIT_WORK_TREE",
    "GIT_IMPLICIT_WORK_TREE",
    "GIT_INDEX_FILE",
    "GIT_OBJECT_DIRECTORY",
    "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    "GIT_CONFIG",
    "GIT_CONFIG_COUNT",
    "GIT_CONFIG_PARAMETERS",
    "GIT_CONFIG_GLOBAL",
    "GIT_CONFIG_SYSTEM",
    "GIT_CONFIG_NOSYSTEM",
    "GIT_EXEC_PATH",
    "GIT_TEMPLATE_DIR",
    "GIT_SSH",
    "GIT_SSH_COMMAND",
    "GIT_EXTERNAL_DIFF",
    "GIT_ASKPASS",
    "SSH_ASKPASS",
    "GIT_PAGER",
    "GIT_EDITOR",
    "GIT_SEQUENCE_EDITOR",
    "GIT_ATTR_SOURCE",
    "GIT_REPLACE_REF_BASE",
    "GIT_NO_REPLACE_OBJECTS",
    "GIT_GRAFT_FILE",
    "GIT_NAMESPACE",
    "GIT_SHALLOW_FILE",
];

#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum GitReadError {
    #[error("git read could not run: {message}")]
    Io {
        kind: std::io::ErrorKind,
        message: String,
    },
    #[error("git read exited unsuccessfully: {0:?}")]
    Failed(Option<i32>),
    #[error("git read exceeded the output limit")]
    OutputLimit,
    #[error("git read timed out")]
    Timeout,
    #[error("git read output pipe was unavailable")]
    MissingOutput,
    #[error("git returned an invalid repository root")]
    InvalidRepositoryRoot,
    #[error("git returned an invalid commit object ID")]
    InvalidCommit,
    #[error("git returned an invalid blob object ID")]
    InvalidBlob,
    #[error("the locator differs from the selected Git tree entry")]
    LocatorMismatch,
    #[error("the worktree HEAD changed while capturing its Context patch")]
    ContextCommitChanged,
    #[error("target is not a worktree of the selected Git repository: {0:?}")]
    UnrelatedWorktree(std::path::PathBuf),
    #[error("git returned invalid fetch remote URLs")]
    InvalidRemoteUrls,
}

/// Local execution for callers which have not provided a runtime owner.
pub(crate) struct LocalExecutor;
impl BoundedProcessExecutor for LocalExecutor {
    fn execute(
        &self,
        request: BoundedProcessRequest,
    ) -> Pin<
        Box<dyn Future<Output = Result<BoundedProcessOutput, ProcessExecutionError>> + Send + '_>,
    > {
        Box::pin(LocalExecutor::run(request, None))
    }
}

impl LocalExecutor {
    async fn run(
        request: BoundedProcessRequest,
        input: Option<&[u8]>,
    ) -> Result<BoundedProcessOutput, ProcessExecutionError> {
        let timeout = request.timeout;
        let operation = async move {
            let is_git_program = request.program == std::ffi::OsStr::new("git");
            let mut command = Command::new(request.program);
            if request.environment == marlin_agent_protocol::ProcessEnvironmentPolicy::Clean {
                command.env_clear();
                command.env(
                    "PATH",
                    marlin_agent_protocol::ProcessEnvironmentPolicy::CLEAN_PATH,
                );
            }
            command
                .args(request.arguments)
                .current_dir(request.working_directory)
                .stdin(if input.is_some() {
                    Stdio::piped()
                } else {
                    Stdio::null()
                })
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .kill_on_drop(true);
            for name in request.remove_environment {
                command.env_remove(name);
            }
            if is_git_program {
                command
                    .env("GIT_CONFIG_NOSYSTEM", "1")
                    .env("GIT_CONFIG_GLOBAL", NULL_DEVICE)
                    .env("GIT_TERMINAL_PROMPT", "0");
            }
            let mut child = command.spawn().map_err(ProcessExecutionError::Io)?;
            if let Some(input) = input {
                let mut stdin = child.stdin.take().ok_or_else(|| {
                    ProcessExecutionError::Io(std::io::Error::other("stdin unavailable"))
                })?;
                stdin
                    .write_all(input)
                    .await
                    .map_err(ProcessExecutionError::Io)?;
            }
            let stdout = child.stdout.take().ok_or_else(|| {
                ProcessExecutionError::Io(std::io::Error::other("stdout unavailable"))
            })?;
            let size = request.stdout_limit.checked_add(1).ok_or_else(|| {
                ProcessExecutionError::Io(std::io::Error::other("stdout limit overflow"))
            })?;
            let mut bytes = Vec::new();
            stdout
                .take(size as u64)
                .read_to_end(&mut bytes)
                .await
                .map_err(ProcessExecutionError::Io)?;
            if bytes.len() > request.stdout_limit {
                child.kill().await.map_err(ProcessExecutionError::Io)?;
                return Err(ProcessExecutionError::OutputLimit);
            }
            let status = child.wait().await.map_err(ProcessExecutionError::Io)?;
            Ok(BoundedProcessOutput {
                stdout: bytes,
                exit_code: status.code(),
                success: status.success(),
            })
        };
        tokio::time::timeout(timeout, operation)
            .await
            .map_err(|_| ProcessExecutionError::TimedOut)?
    }
}

pub(crate) async fn read_git_with(
    executor: &dyn BoundedProcessExecutor,
    path: &Path,
    args: &[OsString],
) -> Result<Vec<u8>, GitReadError> {
    checked_output(executor.execute(git_request(path, args)).await)
}

#[cfg(unix)]
pub(crate) async fn git_with_input(
    path: &Path,
    args: &[OsString],
    input: &[u8],
) -> Result<Vec<u8>, GitReadError> {
    checked_output(LocalExecutor::run(git_request(path, args), Some(input)).await)
}

fn git_request(path: &Path, args: &[OsString]) -> BoundedProcessRequest {
    let mut arguments: Vec<OsString> = [
        "--no-optional-locks",
        "--no-pager",
        "-c",
        DISABLED_HOOKS,
        "-c",
        DISABLED_FSMONITOR,
    ]
    .into_iter()
    .map(Into::into)
    .collect();
    arguments.extend_from_slice(args);
    BoundedProcessRequest {
        program: "git".into(),
        arguments,
        working_directory: path.to_owned(),
        environment: marlin_agent_protocol::ProcessEnvironmentPolicy::Inherit,
        remove_environment: [
            "GIT_LITERAL_PATHSPECS",
            "GIT_GLOB_PATHSPECS",
            "GIT_NOGLOB_PATHSPECS",
            "GIT_ICASE_PATHSPECS",
        ]
        .into_iter()
        .chain(GIT_REPOSITORY_OVERRIDES.iter().copied())
        .map(Into::into)
        .collect(),
        stdout_limit: MAX_OUTPUT_BYTES,
        timeout: TIMEOUT,
    }
}

fn checked_output(
    output: Result<BoundedProcessOutput, ProcessExecutionError>,
) -> Result<Vec<u8>, GitReadError> {
    let output = output.map_err(|error| match error {
        ProcessExecutionError::Io(error) => GitReadError::Io {
            kind: error.kind(),
            message: error.to_string(),
        },
        ProcessExecutionError::OutputLimit => GitReadError::OutputLimit,
        ProcessExecutionError::TimedOut => GitReadError::Timeout,
    })?;
    if !output.success {
        return Err(GitReadError::Failed(output.exit_code));
    }
    Ok(output.stdout)
}
