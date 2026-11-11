//! Bounded execution for read-only Git queries.

use std::{ffi::OsString, path::Path, process::Stdio, time::Duration};
use thiserror::Error;
use tokio::{io::AsyncReadExt, process::Command};

pub(crate) const MAX_OUTPUT_BYTES: usize = 1024 * 1024;
const TIMEOUT: Duration = Duration::from_secs(10);
const DISABLED_HOOKS: &str = if cfg!(windows) {
    "core.hooksPath=NUL"
} else {
    "core.hooksPath=/dev/null"
};

/// A failed or incomplete read-only Git command.
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

pub(crate) async fn read_git(path: &Path, args: &[OsString]) -> Result<Vec<u8>, GitReadError> {
    tokio::time::timeout(TIMEOUT, read_inner(path, args))
        .await
        .map_err(|_| GitReadError::Timeout)?
}

async fn read_inner(path: &Path, args: &[OsString]) -> Result<Vec<u8>, GitReadError> {
    let mut child = Command::new("git")
        .current_dir(path)
        .args([
            "--no-optional-locks",
            "--no-pager",
            "-c",
            DISABLED_HOOKS,
            "-c",
            "core.fsmonitor=false",
        ])
        .env_remove("GIT_LITERAL_PATHSPECS")
        .env_remove("GIT_GLOB_PATHSPECS")
        .env_remove("GIT_NOGLOB_PATHSPECS")
        .env_remove("GIT_ICASE_PATHSPECS")
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()?;
    let stdout = child.stdout.take().ok_or(GitReadError::MissingOutput)?;
    let mut output = Vec::new();
    stdout
        .take((MAX_OUTPUT_BYTES + 1) as u64)
        .read_to_end(&mut output)
        .await?;
    if output.len() > MAX_OUTPUT_BYTES {
        let _ = child.kill().await;
        return Err(GitReadError::OutputLimit);
    }
    let status = child.wait().await?;
    if !status.success() {
        return Err(GitReadError::Failed(status.code()));
    }
    Ok(output)
}
