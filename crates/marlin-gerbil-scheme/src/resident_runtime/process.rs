//! Child process lifecycle for prepared resident `Gerbil Scheme` runtimes.

use std::{
    fmt, io,
    path::PathBuf,
    process::{Child, Command, ExitStatus, Stdio},
    time::Instant,
};

use serde::{Deserialize, Serialize};

use crate::GerbilCommandProfile;

use super::{
    GerbilResidentRuntimeSessionMode, GerbilResidentStrategyEventKind,
    GerbilResidentStrategyExecutionReceipt, GerbilResidentStrategyExecutionRequest,
    GerbilResidentStrategyExecutionResponse, GerbilResidentStrategyExecutor,
    GerbilResidentStrategyLanePlan, GerbilResidentStrategyLaneStatus,
    GerbilResidentStrategyRequest, GerbilResidentStrategyRequestReceipt,
    GerbilResidentStrategyRequestStatus, GerbilResidentStrategyServicePlan,
};

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
    pub session_id: Option<super::GerbilResidentRuntimeSessionId>,
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
    pub session_id: Option<super::GerbilResidentRuntimeSessionId>,
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
    pub session_id: Option<super::GerbilResidentRuntimeSessionId>,
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
    pub session_id: Option<super::GerbilResidentRuntimeSessionId>,
    pub child_id: u32,
    pub status: GerbilResidentRuntimeShutdownStatus,
    pub exit_code: Option<i32>,
    pub exit_success: bool,
}

/// Owned child process for a prepared resident Gerbil batch adapter.
pub struct GerbilResidentRuntimeProcess {
    pub(crate) plan: GerbilResidentRuntimeProcessPlan,
    pub(crate) child: Child,
}

impl GerbilResidentRuntimeProcessPlan {
    pub fn strategy_service_plan(&self) -> GerbilResidentStrategyServicePlan {
        let status = if self.status == GerbilResidentRuntimeProcessStatus::ReadyToSpawn {
            GerbilResidentStrategyLaneStatus::ReadyToServe
        } else {
            GerbilResidentStrategyLaneStatus::Disabled
        };
        let lanes = [
            GerbilResidentStrategyEventKind::DynamicReplan,
            GerbilResidentStrategyEventKind::PolicyChange,
        ]
        .into_iter()
        .map(|event_kind| GerbilResidentStrategyLanePlan {
            lane_id: event_kind.lane_id(),
            event_kind,
            status: status.clone(),
            session_mode: self.session_mode.clone(),
            session_id: self.session_id.clone(),
            process_reuse_required: self.process_reuse_required,
            state_isolated: self.state_isolated,
            command_profile: self.command_profile.clone(),
            loadpath_root: self.loadpath_root.clone(),
        })
        .collect();

        GerbilResidentStrategyServicePlan {
            session_mode: self.session_mode.clone(),
            session_id: self.session_id.clone(),
            process_reuse_required: self.process_reuse_required,
            state_isolated: self.state_isolated,
            loadpath_root: self.loadpath_root.clone(),
            lanes,
        }
    }

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
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null());

            if let Some(current_dir) = &command_profile.current_dir {
                command.current_dir(current_dir);
            }

            command.envs(&command_profile.env);
            command.spawn()?
        };

        Ok(GerbilResidentRuntimeProcess { plan: self, child })
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

    pub fn strategy_request_receipt(
        &mut self,
        request: GerbilResidentStrategyRequest,
    ) -> io::Result<GerbilResidentStrategyRequestReceipt> {
        let health = self.health_receipt()?;
        let mut receipt = self.plan.strategy_service_plan().request_receipt(request);
        receipt.child_id = Some(health.child_id);
        receipt.process_health = Some(health.status.clone());

        if health.status != GerbilResidentRuntimeHealthStatus::Running {
            receipt.status = GerbilResidentStrategyRequestStatus::ProcessNotRunning;
        }

        Ok(receipt)
    }

    pub fn strategy_execution_receipt<F>(
        &mut self,
        request: GerbilResidentStrategyExecutionRequest,
        execute: F,
    ) -> io::Result<GerbilResidentStrategyExecutionReceipt>
    where
        F: FnOnce(
            &GerbilResidentStrategyExecutionRequest,
        ) -> GerbilResidentStrategyExecutionResponse,
    {
        let mut execute = Some(execute);
        let mut closure_executor = |request: &GerbilResidentStrategyExecutionRequest| {
            let execute = execute
                .take()
                .expect("resident strategy closure executor called at most once");
            execute(request)
        };
        self.strategy_execution_receipt_with_executor(request, &mut closure_executor)
    }

    pub fn strategy_execution_receipt_with_executor<E>(
        &mut self,
        request: GerbilResidentStrategyExecutionRequest,
        executor: &mut E,
    ) -> io::Result<GerbilResidentStrategyExecutionReceipt>
    where
        E: GerbilResidentStrategyExecutor,
    {
        let started_at = Instant::now();
        let request_receipt = self.strategy_request_receipt(request.strategy_request.clone())?;
        if request_receipt.status != GerbilResidentStrategyRequestStatus::Accepted {
            return Ok(GerbilResidentStrategyExecutionReceipt::admission_rejected(
                request_receipt,
                request.payload,
                duration_micros_u64(started_at.elapsed()),
            ));
        }

        let response = executor.execute(&request)?;
        Ok(GerbilResidentStrategyExecutionReceipt::from_response(
            request_receipt,
            request.payload,
            response,
            duration_micros_u64(started_at.elapsed()),
        ))
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

        self.child.kill()?;
        let status = self.child.wait()?;
        Ok(self.shutdown_receipt(
            child_id,
            GerbilResidentRuntimeShutdownStatus::Terminated,
            status,
        ))
    }

    pub fn terminate(&mut self) -> io::Result<ExitStatus> {
        if let Some(status) = self.child.try_wait()? {
            return Ok(status);
        }

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

fn duration_micros_u64(duration: std::time::Duration) -> u64 {
    duration.as_micros().try_into().unwrap_or(u64::MAX)
}

impl fmt::Debug for GerbilResidentRuntimeProcess {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GerbilResidentRuntimeProcess")
            .field("plan", &self.plan)
            .field("child_id", &self.child.id())
            .finish_non_exhaustive()
    }
}

impl Drop for GerbilResidentRuntimeProcess {
    fn drop(&mut self) {
        if matches!(self.child.try_wait(), Ok(None)) {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}
