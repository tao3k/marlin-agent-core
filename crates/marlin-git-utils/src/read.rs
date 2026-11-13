// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Bounded execution for read-only Git queries.
use marlin_agent_protocol::{
    BoundedProcessExecutor, BoundedProcessOutput, BoundedProcessRequest, ProcessExecutionError,
};
use std::{ffi::OsString, future::Future, path::Path, pin::Pin, process::Stdio, time::Duration};
use thiserror::Error;
use tokio::{io::AsyncReadExt, process::Command};

pub(crate) const MAX_OUTPUT_BYTES: usize = 1024 * 1024;
const TIMEOUT: Duration = Duration::from_secs(10);
const DISABLED_HOOKS: &str = if cfg!(windows) {
    "core.hooksPath=NUL"
} else {
    "core.hooksPath=/dev/null"
};

#[derive(Debug, Error)]
pub enum GitReadError {
    #[error("git read could not run: {0}")]
    Io(#[from] std::io::Error),
    #[error("git read exited unsuccessfully: {0:?}")]
    Failed(Option<i32>),
    #[error("git read exceeded the output limit")]
    OutputLimit,
    #[error("git read timed out")]
    Timeout,
    #[error("git read output pipe was unavailable")]
    MissingOutput,
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
        Box::pin(async move {
            let timeout = request.timeout;
            let operation = async move {
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
                    .stdin(Stdio::null())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::null())
                    .kill_on_drop(true);
                for name in request.remove_environment {
                    command.env_remove(name);
                }
                let mut child = command.spawn().map_err(ProcessExecutionError::Io)?;
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
        })
    }
}

pub(crate) async fn read_git_with(
    executor: &dyn BoundedProcessExecutor,
    path: &Path,
    args: &[OsString],
) -> Result<Vec<u8>, GitReadError> {
    let mut arguments: Vec<OsString> = [
        "--no-optional-locks",
        "--no-pager",
        "-c",
        DISABLED_HOOKS,
        "-c",
        "core.fsmonitor=false",
    ]
    .into_iter()
    .map(Into::into)
    .collect();
    arguments.extend_from_slice(args);
    let request = BoundedProcessRequest {
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
        .map(Into::into)
        .collect(),
        stdout_limit: MAX_OUTPUT_BYTES,
        timeout: TIMEOUT,
    };
    let output = executor
        .execute(request)
        .await
        .map_err(|error| match error {
            ProcessExecutionError::Io(error) => GitReadError::Io(error),
            ProcessExecutionError::OutputLimit => GitReadError::OutputLimit,
            ProcessExecutionError::TimedOut => GitReadError::Timeout,
        })?;
    if !output.success {
        return Err(GitReadError::Failed(output.exit_code));
    }
    Ok(output.stdout)
}
