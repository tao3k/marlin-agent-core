//! Process helpers for Gerbil package bootstrap commands.

use super::GerbilDepsError;
use std::{
    env,
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
    process::Command,
};

pub(super) struct BootstrapCommand {
    command: Command,
}

impl BootstrapCommand {
    pub(super) fn new(command: Command) -> Self {
        Self { command }
    }

    pub(super) fn args<I, S>(&mut self, args: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.command.args(args);
        self
    }

    pub(super) fn current_dir<P: AsRef<Path>>(&mut self, directory: P) -> &mut Self {
        self.command.current_dir(directory);
        self
    }

    pub(super) fn run(&mut self, label: impl AsRef<str>) -> Result<(), GerbilDepsError> {
        let status = self.command.status().map_err(|error| {
            GerbilDepsError::message(format!("failed to run {}: {error}", label.as_ref()))
        })?;
        if status.success() {
            Ok(())
        } else {
            Err(GerbilDepsError::message(format!(
                "{} failed with status {status}",
                label.as_ref()
            )))
        }
    }
}

pub(super) fn command_stdout<const N: usize>(program: &str, args: [&str; N]) -> Option<String> {
    command_stdout_path(Path::new(program), args)
}

pub(super) fn command_stdout_path<const N: usize>(
    program: &Path,
    args: [&str; N],
) -> Option<String> {
    let output = Command::new(program).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8(output.stdout).ok()?;
    let trimmed = stdout.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

pub(super) fn find_program(program: &str) -> Option<PathBuf> {
    let path = env::var_os("PATH")?;
    env::split_paths(&path)
        .map(|directory| directory.join(program))
        .find(|candidate| candidate.is_file())
}

pub(super) fn prepend_path(path: &Path) -> OsString {
    prepend_env_path(path, "PATH")
}

pub(super) fn prepend_library_path(path: &Path) -> OsString {
    prepend_env_path(path, "LIBRARY_PATH")
}

fn prepend_env_path(path: &Path, variable: &str) -> OsString {
    let mut paths = vec![path.to_path_buf()];
    if let Some(existing) = env::var_os(variable) {
        paths.extend(env::split_paths(&existing));
    }
    env::join_paths(paths).unwrap_or_else(|_| path.as_os_str().to_os_string())
}
