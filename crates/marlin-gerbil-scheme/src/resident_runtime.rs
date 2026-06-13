//! Typed plan and preparation receipt for resident `Gerbil Scheme` runtime sessions.

use std::{
    fmt,
    io::{self, BufRead, BufReader, Write},
    path::PathBuf,
    process::{Child, ChildStdin, ChildStdout, Command, ExitStatus, Stdio},
};

use serde::{Deserialize, Serialize};

use crate::runtime::{
    GERBIL_COMMAND_ADAPTER_BATCH_PATH, GERBIL_LOADPATH_ENV, gerbil_runtime_loadpath,
};
use crate::{
    GerbilArtifactKind, GerbilCommandCompiler, GerbilCommandProfile, GerbilCompileRequest,
    GerbilCompiledArtifact, GerbilSource,
    runtime::{default_gerbil_gxi_program, write_gerbil_runtime_assets},
};

/// Session strategy for a resident Gerbil runtime.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum GerbilResidentRuntimeSessionMode {
    #[default]
    Disabled,
    SharedContext,
    ForkedContext,
    IsolatedSession,
}

/// Stable session label for a resident Gerbil runtime.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct GerbilResidentRuntimeSessionId(String);

impl GerbilResidentRuntimeSessionId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl From<String> for GerbilResidentRuntimeSessionId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for GerbilResidentRuntimeSessionId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// Rust-owned plan for starting or reusing a resident Gerbil runtime.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilResidentRuntimePlan {
    pub session_mode: GerbilResidentRuntimeSessionMode,
    pub session_id: Option<GerbilResidentRuntimeSessionId>,
    pub command_profile: GerbilCommandProfile,
    pub loadpath_root: PathBuf,
}

/// Receipt emitted after preparing the resident runtime loadpath.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilResidentRuntimePrepareReceipt {
    pub session_mode: GerbilResidentRuntimeSessionMode,
    pub session_id: Option<GerbilResidentRuntimeSessionId>,
    pub process_reuse_required: bool,
    pub state_isolated: bool,
    pub command_profile: GerbilCommandProfile,
    pub loadpath_root: PathBuf,
    pub written_asset_count: usize,
}

/// Lifecycle status for a prepared resident process plan.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GerbilResidentRuntimeProcessStatus {
    Disabled,
    ReadyToSpawn,
}

/// Rust-owned plan for spawning the prepared resident Gerbil batch adapter.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilResidentRuntimeProcessPlan {
    pub session_mode: GerbilResidentRuntimeSessionMode,
    pub session_id: Option<GerbilResidentRuntimeSessionId>,
    pub process_reuse_required: bool,
    pub state_isolated: bool,
    pub status: GerbilResidentRuntimeProcessStatus,
    pub command_profile: Option<GerbilCommandProfile>,
    pub loadpath_root: PathBuf,
}

/// Receipt proving the prepared runtime can be projected into a process plan.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilResidentRuntimeProcessReceipt {
    pub session_mode: GerbilResidentRuntimeSessionMode,
    pub session_id: Option<GerbilResidentRuntimeSessionId>,
    pub process_reuse_required: bool,
    pub state_isolated: bool,
    pub status: GerbilResidentRuntimeProcessStatus,
    pub command_profile: Option<GerbilCommandProfile>,
    pub loadpath_root: PathBuf,
    pub written_asset_count: usize,
}

/// Health status for the owned resident Gerbil batch adapter process.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GerbilResidentRuntimeHealthStatus {
    Running,
    Exited,
}

/// Health receipt for a resident Gerbil runtime process.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilResidentRuntimeHealthReceipt {
    pub session_mode: GerbilResidentRuntimeSessionMode,
    pub session_id: Option<GerbilResidentRuntimeSessionId>,
    pub process_reuse_required: bool,
    pub state_isolated: bool,
    pub child_id: u32,
    pub status: GerbilResidentRuntimeHealthStatus,
    pub exit_code: Option<i32>,
    pub exit_success: Option<bool>,
}

/// Shutdown status for a resident Gerbil runtime process.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GerbilResidentRuntimeShutdownStatus {
    AlreadyExited,
    GracefulExit,
    Terminated,
}

/// Shutdown receipt for a resident Gerbil runtime process.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilResidentRuntimeShutdownReceipt {
    pub session_mode: GerbilResidentRuntimeSessionMode,
    pub session_id: Option<GerbilResidentRuntimeSessionId>,
    pub child_id: u32,
    pub status: GerbilResidentRuntimeShutdownStatus,
    pub exit_code: Option<i32>,
    pub exit_success: bool,
}

/// Owned child process for a prepared resident Gerbil batch adapter.
pub struct GerbilResidentRuntimeProcess {
    plan: GerbilResidentRuntimeProcessPlan,
    child: Child,
    stdin: Option<ChildStdin>,
    stdout: BufReader<ChildStdout>,
    request_count: usize,
}

/// Prepared resident runtime handle. Process ownership is layered on top of this.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilResidentRuntimeHandle {
    plan: GerbilResidentRuntimePlan,
    written_assets: Vec<PathBuf>,
}

impl GerbilResidentRuntimePlan {
    pub fn disabled(loadpath_root: impl Into<PathBuf>) -> Self {
        let loadpath_root = loadpath_root.into();
        Self {
            session_mode: GerbilResidentRuntimeSessionMode::Disabled,
            session_id: None,
            command_profile: default_command_profile(&loadpath_root),
            loadpath_root,
        }
    }

    pub fn shared_context(
        loadpath_root: impl Into<PathBuf>,
        session_id: impl Into<GerbilResidentRuntimeSessionId>,
    ) -> Self {
        Self::new(
            GerbilResidentRuntimeSessionMode::SharedContext,
            loadpath_root,
            session_id,
        )
    }

    pub fn forked_context(
        loadpath_root: impl Into<PathBuf>,
        session_id: impl Into<GerbilResidentRuntimeSessionId>,
    ) -> Self {
        Self::new(
            GerbilResidentRuntimeSessionMode::ForkedContext,
            loadpath_root,
            session_id,
        )
    }

    pub fn isolated_session(
        loadpath_root: impl Into<PathBuf>,
        session_id: impl Into<GerbilResidentRuntimeSessionId>,
    ) -> Self {
        Self::new(
            GerbilResidentRuntimeSessionMode::IsolatedSession,
            loadpath_root,
            session_id,
        )
    }

    pub fn with_command_profile(mut self, command_profile: GerbilCommandProfile) -> Self {
        self.command_profile = command_profile;
        self
    }

    /// Writes runtime assets and returns a handle ready for resident process ownership.
    pub fn prepare(self) -> io::Result<GerbilResidentRuntimeHandle> {
        let written_assets = write_gerbil_runtime_assets(&self.loadpath_root)?;
        Ok(GerbilResidentRuntimeHandle {
            plan: self,
            written_assets,
        })
    }

    pub fn requires_process_reuse(&self) -> bool {
        self.session_mode != GerbilResidentRuntimeSessionMode::Disabled
    }

    pub fn isolates_state(&self) -> bool {
        self.session_mode == GerbilResidentRuntimeSessionMode::IsolatedSession
    }

    fn new(
        session_mode: GerbilResidentRuntimeSessionMode,
        loadpath_root: impl Into<PathBuf>,
        session_id: impl Into<GerbilResidentRuntimeSessionId>,
    ) -> Self {
        let loadpath_root = loadpath_root.into();
        Self {
            session_mode,
            session_id: Some(session_id.into()),
            command_profile: default_command_profile(&loadpath_root),
            loadpath_root,
        }
    }
}

fn default_command_profile(loadpath_root: impl Into<PathBuf>) -> GerbilCommandProfile {
    GerbilCommandProfile::marlin_runtime_module(
        default_gerbil_gxi_program().to_string_lossy().into_owned(),
        loadpath_root,
    )
}

fn resident_process_command_profile(plan: &GerbilResidentRuntimePlan) -> GerbilCommandProfile {
    let loadpath = gerbil_runtime_loadpath(&plan.loadpath_root);
    let launcher = plan.loadpath_root.join(GERBIL_COMMAND_ADAPTER_BATCH_PATH);
    let mut profile = plan.command_profile.clone();
    profile.env.insert(
        GERBIL_LOADPATH_ENV.to_owned(),
        loadpath.to_string_lossy().into_owned(),
    );
    profile.args = vec![launcher.to_string_lossy().into_owned()];
    profile
}

impl GerbilResidentRuntimeHandle {
    pub fn plan(&self) -> &GerbilResidentRuntimePlan {
        &self.plan
    }

    pub fn written_assets(&self) -> &[PathBuf] {
        self.written_assets.as_slice()
    }

    pub fn receipt(&self) -> GerbilResidentRuntimePrepareReceipt {
        GerbilResidentRuntimePrepareReceipt {
            session_mode: self.plan.session_mode.clone(),
            session_id: self.plan.session_id.clone(),
            process_reuse_required: self.plan.requires_process_reuse(),
            state_isolated: self.plan.isolates_state(),
            command_profile: self.plan.command_profile.clone(),
            loadpath_root: self.plan.loadpath_root.clone(),
            written_asset_count: self.written_assets.len(),
        }
    }

    pub fn process_plan(&self) -> GerbilResidentRuntimeProcessPlan {
        let process_reuse_required = self.plan.requires_process_reuse();
        let status = if process_reuse_required {
            GerbilResidentRuntimeProcessStatus::ReadyToSpawn
        } else {
            GerbilResidentRuntimeProcessStatus::Disabled
        };
        let command_profile =
            process_reuse_required.then(|| resident_process_command_profile(&self.plan));

        GerbilResidentRuntimeProcessPlan {
            session_mode: self.plan.session_mode.clone(),
            session_id: self.plan.session_id.clone(),
            process_reuse_required,
            state_isolated: self.plan.isolates_state(),
            status,
            command_profile,
            loadpath_root: self.plan.loadpath_root.clone(),
        }
    }

    pub fn process_receipt(&self) -> GerbilResidentRuntimeProcessReceipt {
        let process_plan = self.process_plan();
        GerbilResidentRuntimeProcessReceipt {
            session_mode: process_plan.session_mode,
            session_id: process_plan.session_id,
            process_reuse_required: process_plan.process_reuse_required,
            state_isolated: process_plan.state_isolated,
            status: process_plan.status,
            command_profile: process_plan.command_profile,
            loadpath_root: process_plan.loadpath_root,
            written_asset_count: self.written_assets.len(),
        }
    }

    pub fn spawn_process(&self) -> io::Result<GerbilResidentRuntimeProcess> {
        self.process_plan().spawn()
    }
}

impl GerbilResidentRuntimeProcessPlan {
    pub fn spawn(self) -> io::Result<GerbilResidentRuntimeProcess> {
        if self.status != GerbilResidentRuntimeProcessStatus::ReadyToSpawn {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "resident Gerbil runtime process is disabled",
            ));
        }

        let child = {
            let command_profile = self.command_profile.as_ref().ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "resident Gerbil runtime process is missing command profile",
                )
            })?;
            let mut command = Command::new(&command_profile.program);
            command
                .args(&command_profile.args)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());

            if let Some(current_dir) = &command_profile.current_dir {
                command.current_dir(current_dir);
            }

            command.envs(&command_profile.env);
            command.spawn()?
        };

        let mut child = child;
        let stdin = child.stdin.take().ok_or_else(|| {
            terminate_child_after_spawn_failure(&mut child);
            io::Error::new(
                io::ErrorKind::BrokenPipe,
                "resident Gerbil runtime process did not expose stdin",
            )
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            terminate_child_after_spawn_failure(&mut child);
            io::Error::new(
                io::ErrorKind::BrokenPipe,
                "resident Gerbil runtime process did not expose stdout",
            )
        })?;

        Ok(GerbilResidentRuntimeProcess {
            plan: self,
            child,
            stdin: Some(stdin),
            stdout: BufReader::new(stdout),
            request_count: 0,
        })
    }
}

impl GerbilResidentRuntimeProcess {
    pub fn plan(&self) -> &GerbilResidentRuntimeProcessPlan {
        &self.plan
    }

    pub fn child_id(&self) -> u32 {
        self.child.id()
    }

    pub fn is_running(&mut self) -> io::Result<bool> {
        self.child.try_wait().map(|status| status.is_none())
    }

    pub fn health_receipt(&mut self) -> io::Result<GerbilResidentRuntimeHealthReceipt> {
        let child_id = self.child.id();
        let status = self.child.try_wait()?;
        Ok(GerbilResidentRuntimeHealthReceipt {
            session_mode: self.plan.session_mode.clone(),
            session_id: self.plan.session_id.clone(),
            process_reuse_required: self.plan.process_reuse_required,
            state_isolated: self.plan.state_isolated,
            child_id,
            status: if status.is_some() {
                GerbilResidentRuntimeHealthStatus::Exited
            } else {
                GerbilResidentRuntimeHealthStatus::Running
            },
            exit_code: status.and_then(|status| status.code()),
            exit_success: status.map(|status| status.success()),
        })
    }

    pub fn compile(
        &mut self,
        source: GerbilSource,
        expected: GerbilArtifactKind,
    ) -> Result<GerbilCompiledArtifact, String> {
        self.compile_request_result(GerbilCompileRequest::new(source, expected))?
            .and_then(|artifact| {
                artifact
                    .ensure_kind(expected)
                    .map_err(|error| error.to_string())
            })
    }

    pub fn compile_request_result(
        &mut self,
        request: GerbilCompileRequest,
    ) -> Result<Result<GerbilCompiledArtifact, String>, String> {
        self.compile_request_results(vec![request])
            .map(|mut results| results.remove(0))
    }

    pub fn compile_requests(
        &mut self,
        requests: Vec<GerbilCompileRequest>,
    ) -> Result<Vec<GerbilCompiledArtifact>, String> {
        self.compile_request_results(requests)?
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
    }

    pub fn compile_request_results(
        &mut self,
        requests: Vec<GerbilCompileRequest>,
    ) -> Result<Vec<Result<GerbilCompiledArtifact, String>>, String> {
        let mut results = Vec::with_capacity(requests.len());
        for request in requests {
            let expected = request.expected;
            self.write_compile_request(&request)?;
            results.push(self.read_compile_response(expected)?);
        }
        Ok(results)
    }

    fn write_compile_request(&mut self, request: &GerbilCompileRequest) -> Result<(), String> {
        let mut line = serde_json::to_vec(request).map_err(|error| {
            format!("failed to encode resident gerbil compile request: {error}")
        })?;
        line.push(b'\n');
        let stdin = self
            .stdin
            .as_mut()
            .ok_or_else(|| "resident Gerbil runtime process stdin is closed".to_owned())?;
        stdin
            .write_all(&line)
            .map_err(|error| format!("failed to write resident gerbil compile request: {error}"))?;
        stdin
            .flush()
            .map_err(|error| format!("failed to flush resident gerbil compile request: {error}"))
    }

    fn read_compile_response(
        &mut self,
        expected: GerbilArtifactKind,
    ) -> Result<Result<GerbilCompiledArtifact, String>, String> {
        let mut line = String::new();
        let bytes = self
            .stdout
            .read_line(&mut line)
            .map_err(|error| format!("failed to read resident gerbil compile response: {error}"))?;
        if bytes == 0 {
            return Err("resident Gerbil runtime process exited before response".to_owned());
        }

        let index = self.request_count;
        self.request_count += 1;
        let expected = vec![expected; index + 1];
        GerbilCommandCompiler::decode_compile_batch_response_line(index, line.trim_end(), &expected)
    }

    pub fn wait(&mut self) -> io::Result<ExitStatus> {
        self.child.wait()
    }

    pub fn shutdown(&mut self) -> io::Result<GerbilResidentRuntimeShutdownReceipt> {
        let child_id = self.child.id();
        if let Some(status) = self.child.try_wait()? {
            return Ok(self.shutdown_receipt(
                child_id,
                GerbilResidentRuntimeShutdownStatus::AlreadyExited,
                status,
            ));
        }

        drop(self.stdin.take());
        let status = self.child.wait()?;
        Ok(self.shutdown_receipt(
            child_id,
            GerbilResidentRuntimeShutdownStatus::GracefulExit,
            status,
        ))
    }

    pub fn terminate(&mut self) -> io::Result<ExitStatus> {
        if let Some(status) = self.child.try_wait()? {
            return Ok(status);
        }

        drop(self.stdin.take());
        self.child.kill()?;
        self.child.wait()
    }

    pub fn terminate_with_receipt(&mut self) -> io::Result<GerbilResidentRuntimeShutdownReceipt> {
        let child_id = self.child.id();
        if let Some(status) = self.child.try_wait()? {
            return Ok(self.shutdown_receipt(
                child_id,
                GerbilResidentRuntimeShutdownStatus::AlreadyExited,
                status,
            ));
        }

        drop(self.stdin.take());
        self.child.kill()?;
        let status = self.child.wait()?;
        Ok(self.shutdown_receipt(
            child_id,
            GerbilResidentRuntimeShutdownStatus::Terminated,
            status,
        ))
    }

    fn shutdown_receipt(
        &self,
        child_id: u32,
        status: GerbilResidentRuntimeShutdownStatus,
        exit_status: ExitStatus,
    ) -> GerbilResidentRuntimeShutdownReceipt {
        GerbilResidentRuntimeShutdownReceipt {
            session_mode: self.plan.session_mode.clone(),
            session_id: self.plan.session_id.clone(),
            child_id,
            status,
            exit_code: exit_status.code(),
            exit_success: exit_status.success(),
        }
    }
}

impl fmt::Debug for GerbilResidentRuntimeProcess {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GerbilResidentRuntimeProcess")
            .field("plan", &self.plan)
            .field("child_id", &self.child.id())
            .field("request_count", &self.request_count)
            .finish_non_exhaustive()
    }
}

impl Drop for GerbilResidentRuntimeProcess {
    fn drop(&mut self) {
        drop(self.stdin.take());
        if matches!(self.child.try_wait(), Ok(None)) {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

fn terminate_child_after_spawn_failure(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}
