use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use marlin_agent_environment::{DirenvCommandRunner, RuntimeEnvironmentActivationError};

mod basic;
mod preflight;
mod real_native;

#[derive(Clone, Debug, Eq, PartialEq)]
enum FakeDirenvRunner {
    Success {
        cwd: PathBuf,
        environment: BTreeMap<String, String>,
        json: String,
    },
    Error(RuntimeEnvironmentActivationError),
}

#[async_trait]
impl DirenvCommandRunner for FakeDirenvRunner {
    async fn export_json(
        &self,
        cwd: &Path,
        environment: &BTreeMap<String, String>,
    ) -> Result<String, RuntimeEnvironmentActivationError> {
        match self {
            Self::Success {
                cwd: expected_cwd,
                environment: expected_environment,
                json,
            } => {
                assert_eq!(cwd, expected_cwd.as_path());
                assert_eq!(environment, expected_environment);
                Ok(json.clone())
            }
            Self::Error(error) => Err(error.clone()),
        }
    }
}

#[derive(Clone, Debug)]
struct RecordingDirenvRunner {
    cwd: PathBuf,
    environment: BTreeMap<String, String>,
    actions: Arc<Mutex<Vec<&'static str>>>,
    fail_reload: bool,
    json: String,
}

#[async_trait]
impl DirenvCommandRunner for RecordingDirenvRunner {
    async fn reload(
        &self,
        cwd: &Path,
        environment: &BTreeMap<String, String>,
    ) -> Result<(), RuntimeEnvironmentActivationError> {
        assert_eq!(cwd, self.cwd.as_path());
        assert_eq!(environment, &self.environment);
        self.actions.lock().expect("actions lock").push("reload");
        if self.fail_reload {
            return Err(RuntimeEnvironmentActivationError::CommandFailed { status: Some(1) });
        }
        Ok(())
    }

    async fn export_json(
        &self,
        cwd: &Path,
        environment: &BTreeMap<String, String>,
    ) -> Result<String, RuntimeEnvironmentActivationError> {
        assert_eq!(cwd, self.cwd.as_path());
        assert_eq!(environment, &self.environment);
        self.actions
            .lock()
            .expect("actions lock")
            .push("export_json");
        Ok(self.json.clone())
    }
}

fn command_stdout(command: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(command).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8(output.stdout).ok()?;
    let stdout = stdout.trim();
    if stdout.is_empty() {
        return None;
    }
    Some(stdout.to_owned())
}

fn shell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}
