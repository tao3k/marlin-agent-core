//! Native Git operations used by runtime adapters.

use std::{
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
};

use thiserror::Error;
use tokio::process::Command;

const DISABLED_HOOKS_PATH: &str = if cfg!(windows) { "NUL" } else { "/dev/null" };

/// Absolute Git repository root resolved from native Git.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GitRepositoryRoot(PathBuf);

impl GitRepositoryRoot {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self(path.into())
    }

    pub fn as_path(&self) -> &Path {
        self.0.as_path()
    }

    pub fn into_path_buf(self) -> PathBuf {
        self.0
    }
}

/// Production Git helper backed by the `git` executable.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProcessGitTooling;

impl ProcessGitTooling {
    /// Resolves the canonical Git worktree root for any path inside a worktree.
    pub async fn resolve_repository_root(
        path: &Path,
    ) -> Result<GitRepositoryRoot, GitToolingError> {
        let root = run_git_for_stdout(
            path,
            [
                OsString::from("rev-parse"),
                OsString::from("--show-toplevel"),
            ],
        )
        .await?;

        Ok(GitRepositoryRoot::new(root))
    }
}

async fn run_git_for_stdout<I, S>(dir: &Path, args: I) -> Result<String, GitToolingError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let args = git_args(args);
    let command = command_string(&args);
    let output = Command::new("git")
        .current_dir(dir)
        .args(&args)
        .output()
        .await
        .map_err(|error| GitToolingError::CommandIo {
            command: command.clone(),
            message: error.to_string(),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8(output.stderr).unwrap_or_else(|error| error.to_string());
        return Err(GitToolingError::CommandFailed {
            command,
            status_code: output.status.code(),
            stderr,
        });
    }

    String::from_utf8(output.stdout)
        .map(|value| value.trim().to_owned())
        .map_err(|error| GitToolingError::OutputUtf8 {
            command,
            message: error.to_string(),
        })
}

fn git_args<I, S>(args: I) -> Vec<OsString>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let iterator = args.into_iter();
    let (lower, upper) = iterator.size_hint();
    let mut args = Vec::with_capacity(upper.unwrap_or(lower) + 2);
    args.push(OsString::from("-c"));
    args.push(OsString::from(format!(
        "core.hooksPath={DISABLED_HOOKS_PATH}"
    )));
    args.extend(iterator.map(|arg| OsString::from(arg.as_ref())));
    args
}

fn command_string(args: &[OsString]) -> String {
    let mut parts = Vec::with_capacity(args.len() + 1);
    parts.push("git".to_owned());
    parts.extend(args.iter().map(|arg| arg.to_string_lossy().into_owned()));
    parts.join(" ")
}

/// Error raised by Git utility execution.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum GitToolingError {
    #[error("Git command {command} could not run: {message}")]
    CommandIo { command: String, message: String },
    #[error("Git command {command} failed with status {status_code:?}: {stderr}")]
    CommandFailed {
        command: String,
        status_code: Option<i32>,
        stderr: String,
    },
    #[error("Git command {command} output is not utf8: {message}")]
    OutputUtf8 { command: String, message: String },
}
