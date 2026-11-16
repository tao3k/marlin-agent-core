// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Bounded process request shared across Runtime and tool callers.
use std::{ffi::OsString, future::Future, io, path::PathBuf, pin::Pin, time::Duration};

/// Physical footprint evidence. The child total covers the supplied direct PIDs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProcessMemorySample {
    pub host_bytes: u64,
    pub child_bytes: u64,
}

/// Local OS sampling port. Implementations require neither a collector nor an
/// owning process handle.
pub trait ProcessMemorySampler: std::fmt::Debug + Send + Sync {
    fn sample(&self, direct_child_pids: &[u32]) -> io::Result<ProcessMemorySample>;
}

#[derive(Debug)]
pub struct BoundedProcessRequest {
    pub program: OsString,
    pub arguments: Vec<OsString>,
    pub working_directory: PathBuf,
    pub remove_environment: Vec<OsString>,
    pub environment: ProcessEnvironmentPolicy,
    pub stdout_limit: usize,
    pub timeout: Duration,
}
/// Whether a process sees the host environment. Model tools use a clean one.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProcessEnvironmentPolicy {
    Inherit,
    Clean,
}

impl ProcessEnvironmentPolicy {
    pub const CLEAN_PATH: &'static str =
        "/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin";
}
#[derive(Debug)]
pub struct BoundedProcessOutput {
    pub stdout: Vec<u8>,
    pub exit_code: Option<i32>,
    pub success: bool,
}
#[derive(Debug)]
pub enum ProcessExecutionError {
    Io(io::Error),
    OutputLimit,
    TimedOut,
}
/// The executor owns the child and captured output until completion or cleanup.
pub trait BoundedProcessExecutor: Send + Sync {
    fn execute(
        &self,
        request: BoundedProcessRequest,
    ) -> Pin<
        Box<dyn Future<Output = Result<BoundedProcessOutput, ProcessExecutionError>> + Send + '_>,
    >;
}
