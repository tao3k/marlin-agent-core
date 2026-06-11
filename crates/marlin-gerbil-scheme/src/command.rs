//! Out-of-process `Gerbil` compiler adapter.

use crate::{GerbilArtifactKind, GerbilCompiledArtifact, GerbilCompiler, GerbilSource};
use serde::{Deserialize, Serialize};
use std::{
    env,
    ffi::OsString,
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};

/// Environment variable carrying a JSON encoded `GerbilCommandProfile`.
pub const GERBIL_COMMAND_PROFILE_ENV: &str = "MARLIN_GERBIL_COMMAND_PROFILE";

/// JSON request sent to an external `Gerbil` compiler process on stdin.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilCompileRequest {
    pub source: GerbilSource,
    pub expected: GerbilArtifactKind,
}

/// JSON response read from an external `Gerbil` compiler process on stdout.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GerbilCompileResponse {
    pub artifact: GerbilCompiledArtifact,
}

/// Serializable command profile for configuring a `Gerbil` compiler executable.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilCommandProfile {
    pub program: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub current_dir: Option<String>,
}

impl GerbilCommandProfile {
    pub fn new(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            current_dir: None,
        }
    }

    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    pub fn current_dir(mut self, current_dir: impl Into<String>) -> Self {
        self.current_dir = Some(current_dir.into());
        self
    }

    pub fn from_json(value: &str) -> Result<Self, String> {
        serde_json::from_str(value)
            .map_err(|error| format!("failed to decode gerbil command profile: {error}"))
    }

    pub fn from_env() -> Result<Option<Self>, String> {
        match env::var(GERBIL_COMMAND_PROFILE_ENV) {
            Ok(value) => Self::from_json(&value).map(Some),
            Err(env::VarError::NotPresent) => Ok(None),
            Err(error) => Err(format!(
                "failed to read {GERBIL_COMMAND_PROFILE_ENV} environment variable: {error}"
            )),
        }
    }
}

impl From<GerbilCommandProfile> for GerbilCommandSpec {
    fn from(profile: GerbilCommandProfile) -> Self {
        let mut spec = GerbilCommandSpec::new(profile.program);
        spec.args = profile.args.into_iter().map(OsString::from).collect();
        spec.current_dir = profile.current_dir.map(PathBuf::from);
        spec
    }
}

/// Command used to invoke an external `Gerbil` compiler adapter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilCommandSpec {
    pub program: PathBuf,
    pub args: Vec<OsString>,
    pub current_dir: Option<PathBuf>,
}

impl GerbilCommandSpec {
    pub fn new(program: impl Into<PathBuf>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            current_dir: None,
        }
    }

    pub fn arg(mut self, arg: impl Into<OsString>) -> Self {
        self.args.push(arg.into());
        self
    }

    pub fn current_dir(mut self, current_dir: impl Into<PathBuf>) -> Self {
        self.current_dir = Some(current_dir.into());
        self
    }
}

/// Compiler implementation backed by a JSON stdin/stdout command protocol.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilCommandCompiler {
    spec: GerbilCommandSpec,
}

impl GerbilCommandCompiler {
    pub fn new(spec: GerbilCommandSpec) -> Self {
        Self { spec }
    }

    pub fn from_profile(profile: GerbilCommandProfile) -> Self {
        Self::new(profile.into())
    }

    pub fn from_profile_json(value: &str) -> Result<Self, String> {
        GerbilCommandProfile::from_json(value).map(Self::from_profile)
    }

    pub fn from_env() -> Result<Option<Self>, String> {
        GerbilCommandProfile::from_env().map(|profile| profile.map(Self::from_profile))
    }

    pub fn spec(&self) -> &GerbilCommandSpec {
        &self.spec
    }
}

impl GerbilCompiler for GerbilCommandCompiler {
    fn compile(
        &self,
        source: GerbilSource,
        expected: GerbilArtifactKind,
    ) -> Result<GerbilCompiledArtifact, String> {
        let request = GerbilCompileRequest { source, expected };

        let mut command = Command::new(&self.spec.program);
        command
            .args(&self.spec.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if let Some(current_dir) = &self.spec.current_dir {
            command.current_dir(current_dir);
        }

        let mut child = command
            .spawn()
            .map_err(|error| format!("failed to start gerbil compiler command: {error}"))?;

        {
            let stdin = child
                .stdin
                .as_mut()
                .ok_or_else(|| "gerbil compiler command did not expose stdin".to_string())?;
            serde_json::to_writer(&mut *stdin, &request)
                .map_err(|error| format!("failed to encode gerbil compile request: {error}"))?;
            stdin
                .write_all(b"\n")
                .map_err(|error| format!("failed to finish gerbil compile request: {error}"))?;
        }

        let output = child
            .wait_with_output()
            .map_err(|error| format!("failed to read gerbil compiler command output: {error}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!(
                "gerbil compiler command failed with status {}: {}",
                output.status,
                stderr.trim()
            ));
        }

        let response: GerbilCompileResponse = serde_json::from_slice(&output.stdout)
            .map_err(|error| format!("failed to decode gerbil compile response: {error}"))?;

        response
            .artifact
            .ensure_kind(expected)
            .map_err(|error| error.to_string())
    }
}
