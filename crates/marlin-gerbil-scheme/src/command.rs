//! Out-of-process `Gerbil` compiler adapter.

use crate::{
    GerbilArtifactKind, GerbilCompiledArtifact, GerbilCompiler, GerbilSource,
    runtime::{
        GERBIL_ADAPTER_MODULE, GERBIL_COMMAND_ADAPTER_BATCH_PATH, GERBIL_COMMAND_ADAPTER_PATH,
        GERBIL_DECK_RUNTIME_POLICY_ADAPTER_PATH, GERBIL_HOOK_POLICY_ADAPTER_PATH,
        GERBIL_LOADPATH_ENV, default_gerbil_gxi_program, gerbil_runtime_loadpath,
        write_gerbil_runtime_assets,
    },
};
use marlin_gerbil_ir::GerbilWorkspaceContractFacts;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    env,
    ffi::OsString,
    io::{self, BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Child, Command as StdCommand, Stdio},
};

/// Environment variable carrying a JSON encoded `GerbilCommandProfile`.
pub const GERBIL_COMMAND_PROFILE_ENV: &str = "MARLIN_GERBIL_COMMAND_PROFILE";

/// JSON request sent to an external `Gerbil` compiler process on stdin.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilCompileRequest {
    pub source: GerbilSource,
    pub expected: GerbilArtifactKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contract_facts: Option<GerbilWorkspaceContractFacts>,
}

impl GerbilCompileRequest {
    /// Build a compile request with protocol defaults for the expected artifact.
    pub fn new(source: GerbilSource, expected: GerbilArtifactKind) -> Self {
        Self {
            source,
            expected,
            contract_facts: default_contract_facts_for(expected),
        }
    }

    /// Build a compile request with explicit parser-owned Org contract facts.
    pub fn with_contract_facts(
        source: GerbilSource,
        expected: GerbilArtifactKind,
        contract_facts: GerbilWorkspaceContractFacts,
    ) -> Self {
        Self {
            source,
            expected,
            contract_facts: Some(contract_facts),
        }
    }
}

/// JSON response read from an external `Gerbil` compiler process on stdout.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GerbilCompileResponse {
    pub artifact: GerbilCompiledArtifact,
}

/// Error envelope emitted by the Gerbil batch compiler adapter.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct GerbilCompileErrorResponse {
    error: GerbilCompileErrorDetail,
}

/// Error detail emitted for a single Gerbil compile request.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct GerbilCompileErrorDetail {
    message: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
pub(crate) enum GerbilCompileBatchResponse {
    Artifact(GerbilCompileResponse),
    Error(GerbilCompileErrorResponse),
}

/// Serializable command profile for configuring a `Gerbil` compiler executable.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilCommandProfile {
    pub program: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub current_dir: Option<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
}

impl GerbilCommandProfile {
    pub fn new(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            current_dir: None,
            env: BTreeMap::new(),
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

    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    /// Builds a profile for the crate-shipped `:marlin/adapter` module entrypoint.
    pub fn marlin_runtime_module(
        program: impl Into<String>,
        loadpath_root: impl Into<PathBuf>,
    ) -> Self {
        let loadpath_root = loadpath_root.into();
        let loadpath = gerbil_runtime_loadpath(&loadpath_root);
        Self::new(program)
            .env(GERBIL_LOADPATH_ENV, loadpath.to_string_lossy().into_owned())
            .arg(GERBIL_ADAPTER_MODULE)
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
        spec.env = profile
            .env
            .into_iter()
            .map(|(key, value)| (OsString::from(key), OsString::from(value)))
            .collect();
        spec
    }
}

/// Command used to invoke an external `Gerbil` compiler adapter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilCommandSpec {
    pub program: PathBuf,
    pub args: Vec<OsString>,
    pub current_dir: Option<PathBuf>,
    pub env: BTreeMap<OsString, OsString>,
}

impl GerbilCommandSpec {
    pub fn new(program: impl Into<PathBuf>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            current_dir: None,
            env: BTreeMap::new(),
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

    pub fn env(mut self, key: impl Into<OsString>, value: impl Into<OsString>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    /// Builds a command spec for the crate-shipped `:marlin/adapter` module entrypoint.
    pub fn marlin_runtime_module(
        program: impl Into<PathBuf>,
        loadpath_root: impl Into<PathBuf>,
    ) -> Self {
        let loadpath_root = loadpath_root.into();
        let loadpath = gerbil_runtime_loadpath(&loadpath_root);
        Self::new(program)
            .env(GERBIL_LOADPATH_ENV, loadpath.into_os_string())
            .arg(GERBIL_ADAPTER_MODULE)
    }

    /// Builds a command spec for the crate-shipped `command-adapter.ss` launcher.
    pub fn marlin_runtime_launcher(
        program: impl Into<PathBuf>,
        loadpath_root: impl Into<PathBuf>,
    ) -> Self {
        let loadpath_root = loadpath_root.into();
        let loadpath = gerbil_runtime_loadpath(&loadpath_root);
        let launcher = loadpath_root.join(GERBIL_COMMAND_ADAPTER_PATH);
        Self::new(program)
            .env(GERBIL_LOADPATH_ENV, loadpath.into_os_string())
            .arg(launcher.into_os_string())
    }

    /// Builds a command spec for the batched `command-adapter-batch.ss` launcher.
    pub fn marlin_runtime_batch_launcher(
        program: impl Into<PathBuf>,
        loadpath_root: impl Into<PathBuf>,
    ) -> Self {
        let loadpath_root = loadpath_root.into();
        let loadpath = gerbil_runtime_loadpath(&loadpath_root);
        let launcher = loadpath_root.join(GERBIL_COMMAND_ADAPTER_BATCH_PATH);
        Self::new(program)
            .env(GERBIL_LOADPATH_ENV, loadpath.into_os_string())
            .arg(launcher.into_os_string())
    }

    /// Builds a command spec for the `hook-policy-adapter.ss` launcher.
    pub fn marlin_hook_policy_launcher(
        program: impl Into<PathBuf>,
        loadpath_root: impl Into<PathBuf>,
    ) -> Self {
        let loadpath_root = loadpath_root.into();
        let loadpath = gerbil_runtime_loadpath(&loadpath_root);
        let launcher = loadpath_root.join(GERBIL_HOOK_POLICY_ADAPTER_PATH);
        Self::new(program)
            .env(GERBIL_LOADPATH_ENV, loadpath.into_os_string())
            .arg(launcher.into_os_string())
    }

    /// Builds a command spec for the `deck-runtime-policy-adapter.ss` launcher.
    pub fn marlin_deck_runtime_policy_launcher(
        program: impl Into<PathBuf>,
        loadpath_root: impl Into<PathBuf>,
    ) -> Self {
        let loadpath_root = loadpath_root.into();
        let loadpath = gerbil_runtime_loadpath(&loadpath_root);
        let launcher = loadpath_root.join(GERBIL_DECK_RUNTIME_POLICY_ADAPTER_PATH);
        Self::new(program)
            .env(GERBIL_LOADPATH_ENV, loadpath.into_os_string())
            .arg(launcher.into_os_string())
    }
}

/// Compiler implementation backed by a JSON stdin/stdout command protocol.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilCommandCompiler {
    spec: GerbilCommandSpec,
}

/// One-stop Rust binding for the crate-shipped `Gerbil` runtime assets and adapter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilRuntimeBinding {
    loadpath_root: PathBuf,
    written_assets: Vec<PathBuf>,
    compiler: GerbilCommandCompiler,
}

impl GerbilRuntimeBinding {
    /// Writes runtime assets under `loadpath_root` and binds the `:marlin/adapter` module.
    pub fn new(program: impl Into<PathBuf>, loadpath_root: impl Into<PathBuf>) -> io::Result<Self> {
        let loadpath_root = loadpath_root.into();
        let written_assets = write_gerbil_runtime_assets(&loadpath_root)?;
        let compiler =
            GerbilCommandCompiler::from_marlin_runtime_module(program, loadpath_root.clone());
        Ok(Self {
            loadpath_root,
            written_assets,
            compiler,
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

    pub fn compiler(&self) -> &GerbilCommandCompiler {
        &self.compiler
    }

    pub fn spec(&self) -> &GerbilCommandSpec {
        self.compiler.spec()
    }

    pub fn into_compiler(self) -> GerbilCommandCompiler {
        self.compiler
    }
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

    /// Builds a compiler for the crate-shipped `:marlin/adapter` module entrypoint.
    pub fn from_marlin_runtime_module(
        program: impl Into<PathBuf>,
        loadpath_root: impl Into<PathBuf>,
    ) -> Self {
        Self::new(GerbilCommandSpec::marlin_runtime_module(
            program,
            loadpath_root,
        ))
    }

    /// Writes crate-shipped runtime assets and builds a compiler for the default `gxi`.
    pub fn from_default_marlin_runtime_module(
        loadpath_root: impl Into<PathBuf>,
    ) -> io::Result<Self> {
        let loadpath_root = loadpath_root.into();
        write_gerbil_runtime_assets(&loadpath_root)?;
        Ok(Self::from_marlin_runtime_module(
            default_gerbil_gxi_program(),
            loadpath_root,
        ))
    }

    pub fn spec(&self) -> &GerbilCommandSpec {
        &self.spec
    }

    /// Compile with parser-owned Org contract facts included in the command request.
    pub fn compile_with_contract_facts(
        &self,
        source: GerbilSource,
        expected: GerbilArtifactKind,
        contract_facts: GerbilWorkspaceContractFacts,
    ) -> Result<GerbilCompiledArtifact, String> {
        self.compile_request(GerbilCompileRequest {
            source,
            expected,
            contract_facts: Some(contract_facts),
        })
    }

    /// Compile several requests through one command process using newline-delimited JSON.
    pub fn compile_requests(
        &self,
        requests: Vec<GerbilCompileRequest>,
    ) -> Result<Vec<GerbilCompiledArtifact>, String> {
        self.compile_request_results(requests)?
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
    }

    /// Compile several requests through one command process while preserving per-request errors.
    pub fn compile_request_results(
        &self,
        requests: Vec<GerbilCompileRequest>,
    ) -> Result<Vec<Result<GerbilCompiledArtifact, String>>, String> {
        self.compile_request_results_inner(requests)
    }

    fn compile_request_results_inner(
        &self,
        requests: Vec<GerbilCompileRequest>,
    ) -> Result<Vec<Result<GerbilCompiledArtifact, String>>, String> {
        let expected = requests
            .iter()
            .map(|request| request.expected)
            .collect::<Vec<_>>();
        let request_json = requests
            .iter()
            .map(|request| {
                let mut line = serde_json::to_vec(request)
                    .map_err(|error| format!("failed to encode gerbil compile request: {error}"))?;
                line.push(b'\n');
                Ok(line)
            })
            .collect::<Result<Vec<_>, String>>()?
            .concat();

        let output = self.run_command_with_stdin(request_json)?;
        let stdout = BufReader::new(output.stdout.as_slice());
        let results = stdout
            .lines()
            .filter_map(|line| match line {
                Ok(line) if line.trim().is_empty() => None,
                other => Some(other),
            })
            .enumerate()
            .map(|(index, line)| {
                let line = line.map_err(|error| {
                    format!("failed to read gerbil compile response line: {error}")
                })?;
                Self::decode_compile_batch_response_line(index, &line, &expected)
            })
            .collect::<Result<Vec<_>, String>>()?;

        if results.len() != expected.len() {
            return Err(format!(
                "gerbil compiler returned {} responses for {} requests",
                results.len(),
                expected.len()
            ));
        }

        Ok(results)
    }

    pub(crate) fn decode_compile_batch_response_line(
        index: usize,
        line: &str,
        expected: &[GerbilArtifactKind],
    ) -> Result<Result<GerbilCompiledArtifact, String>, String> {
        let response: GerbilCompileBatchResponse = serde_json::from_str(line)
            .map_err(|error| format!("failed to decode gerbil compile response: {error}"))?;
        let expected_kind = expected
            .get(index)
            .copied()
            .ok_or_else(|| format!("unexpected gerbil compile response at index {index}"))?;
        Ok(match response {
            GerbilCompileBatchResponse::Artifact(response) => response
                .artifact
                .ensure_kind(expected_kind)
                .map_err(|error| error.to_string()),
            GerbilCompileBatchResponse::Error(response) => Err(format!(
                "gerbil compiler command failed for request {index}: {}",
                response.error.message
            )),
        })
    }

    fn compile_request(
        &self,
        request: GerbilCompileRequest,
    ) -> Result<GerbilCompiledArtifact, String> {
        let expected = request.expected;
        let mut request_json = serde_json::to_vec(&request)
            .map_err(|error| format!("failed to encode gerbil compile request: {error}"))?;
        request_json.push(b'\n');

        let output = self.run_command_with_stdin(request_json)?;

        let response: GerbilCompileResponse = serde_json::from_slice(&output.stdout)
            .map_err(|error| format!("failed to decode gerbil compile response: {error}"))?;

        response
            .artifact
            .ensure_kind(expected)
            .map_err(|error| error.to_string())
    }

    fn run_command_with_stdin(&self, stdin_bytes: Vec<u8>) -> Result<std::process::Output, String> {
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

        let mut child = command
            .spawn()
            .map_err(|error| format!("failed to start gerbil compiler command: {error}"))?;

        {
            let mut stdin = match child.stdin.take() {
                Some(stdin) => stdin,
                None => {
                    terminate_child_after_start_failure(&mut child);
                    return Err("gerbil compiler command did not expose stdin".to_string());
                }
            };
            if let Err(error) = stdin.write_all(&stdin_bytes)
                && error.kind() != io::ErrorKind::BrokenPipe
            {
                terminate_child_after_start_failure(&mut child);
                return Err(format!("failed to write gerbil compile request: {error}"));
            }
        }

        let output = child
            .wait_with_output()
            .map_err(|error| format!("failed to read gerbil compiler command output: {error}"))?;

        validate_gerbil_command_output(output)
    }
}

fn terminate_child_after_start_failure(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn validate_gerbil_command_output(
    output: std::process::Output,
) -> Result<std::process::Output, String> {
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
        return Err(format!(
            "gerbil compiler command failed with status {}: {}",
            output.status, diagnostics
        ));
    }

    Ok(output)
}

fn default_contract_facts_for(
    expected: GerbilArtifactKind,
) -> Option<GerbilWorkspaceContractFacts> {
    matches!(expected, GerbilArtifactKind::WorkspacePatchIntent)
        .then(GerbilWorkspaceContractFacts::default)
}

impl GerbilCompiler for GerbilCommandCompiler {
    fn compile(
        &self,
        source: GerbilSource,
        expected: GerbilArtifactKind,
    ) -> Result<GerbilCompiledArtifact, String> {
        self.compile_request(GerbilCompileRequest::new(source, expected))
    }
}
