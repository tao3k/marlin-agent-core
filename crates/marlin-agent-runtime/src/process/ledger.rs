// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Persistent process ledger types and crash-safe durable storage backends.

use std::{
    collections::BTreeMap,
    fmt,
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use super::shutdown::{ProcessGracefulShutdownDurationMs, ProcessTerminationStage};
use super::types::{
    RuntimeCommandObservation, RuntimeProcessHandle, RuntimeProcessKind, RuntimeProcessObservation,
    RuntimeProcessObservationTimestampMs, RuntimeProcessStatus,
};
use crate::{RuntimeOperationId, RuntimeResourceGeneration, SessionId};

static NEXT_HOST_RUN_SEQ: AtomicU64 = AtomicU64::new(1);
static NEXT_TEMP_FILE_SEQ: AtomicU64 = AtomicU64::new(1);

/// Unique identifier for one Marlin host runtime boot.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RuntimeHostRunId(String);

impl RuntimeHostRunId {
    /// Construct an explicit host run identifier (e.g. from config or tests).
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Generate a collision-resistant identifier for the current host process boot.
    pub fn generate() -> Self {
        let pid = std::process::id();
        let boot_ns = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        let seq = NEXT_HOST_RUN_SEQ.fetch_add(1, Ordering::Relaxed);
        Self(format!("host-{pid:x}-{boot_ns:x}-{seq:x}"))
    }

    /// Borrow the underlying host run identifier string.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for RuntimeHostRunId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Typed Unix process group identifier (`pgid`).
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProcessGroupId(u32);

impl ProcessGroupId {
    /// Wrap a raw Unix process group identifier.
    pub const fn new(pgid: u32) -> Self {
        Self(pgid)
    }

    /// Return the raw Unix process group identifier.
    pub const fn as_u32(self) -> u32 {
        self.0
    }
}

/// Operating-system process start timestamp (Unix seconds) used to verify PID
/// identity across restarts and prevent PID-reuse false positives.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProcessStartTimeSeconds(u64);

impl ProcessStartTimeSeconds {
    /// Wrap an OS process start timestamp in Unix seconds.
    pub const fn new(seconds: u64) -> Self {
        Self(seconds)
    }

    /// Return the OS process start timestamp in Unix seconds.
    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

/// Named request constructing a fresh durable ledger entry from a live observation.
#[derive(Debug)]
pub struct ManagedProcessLedgerRecordRequest<'a> {
    pub host_run_id: RuntimeHostRunId,
    pub host_pid: u32,
    pub observation: &'a RuntimeProcessObservation,
    pub pgid: Option<ProcessGroupId>,
    pub os_start_time_secs: Option<ProcessStartTimeSeconds>,
    pub graceful_timeout: ProcessGracefulShutdownDurationMs,
    pub observed_at_ms: RuntimeProcessObservationTimestampMs,
}

/// Durable ledger record for one managed process spawned under Marlin.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ManagedProcessLedgerEntry {
    pub host_run_id: RuntimeHostRunId,
    pub host_pid: u32,
    pub pid: u32,
    pub pgid: Option<ProcessGroupId>,
    pub os_start_time_secs: Option<ProcessStartTimeSeconds>,
    pub session_id: Option<SessionId>,
    pub operation_id: Option<RuntimeOperationId>,
    pub generation: Option<RuntimeResourceGeneration>,
    pub handle: RuntimeProcessHandle,
    pub kind: RuntimeProcessKind,
    pub owner_reference: String,
    pub command: Option<RuntimeCommandObservation>,
    pub status: RuntimeProcessStatus,
    pub termination_stage: ProcessTerminationStage,
    pub started_at_ms: RuntimeProcessObservationTimestampMs,
    pub last_observed_at_ms: RuntimeProcessObservationTimestampMs,
    pub ended_at_ms: Option<RuntimeProcessObservationTimestampMs>,
    pub exit_code: Option<i32>,
    pub graceful_timeout: ProcessGracefulShutdownDurationMs,
    pub escalated_to_sigkill: bool,
    pub cleanup_attempts: u32,
}

impl ManagedProcessLedgerEntry {
    /// Stable composite key isolating entries across host restarts and PID reuse.
    pub fn ledger_key(&self) -> (RuntimeHostRunId, u32) {
        (self.host_run_id.clone(), self.pid)
    }

    /// True if the process is still considered live or awaiting cleanup/reconciliation.
    pub fn is_non_terminal(&self) -> bool {
        !self.termination_stage.is_terminal()
    }

    /// Construct a fresh ledger entry from an active runtime process observation.
    pub fn from_request(request: ManagedProcessLedgerRecordRequest<'_>) -> Self {
        let (session_id, operation_id, generation) = request
            .observation
            .resource_id()
            .map(|id| {
                (
                    Some(id.session().clone()),
                    Some(id.operation().clone()),
                    Some(id.generation()),
                )
            })
            .unwrap_or((None, None, None));
        let started_at_ms = request
            .observation
            .started_at_ms
            .unwrap_or(request.observed_at_ms);
        let last_observed_at_ms = request
            .observation
            .last_observed_at_ms
            .unwrap_or(request.observed_at_ms);
        Self {
            host_run_id: request.host_run_id,
            host_pid: request.host_pid,
            pid: request.observation.pid,
            pgid: request.pgid,
            os_start_time_secs: request.os_start_time_secs,
            session_id,
            operation_id,
            generation,
            handle: request.observation.handle.clone(),
            kind: request.observation.kind.clone(),
            owner_reference: request.observation.owner_reference.clone(),
            command: request.observation.command.clone(),
            status: request.observation.status.clone(),
            termination_stage: ProcessTerminationStage::Running,
            started_at_ms,
            last_observed_at_ms,
            ended_at_ms: None,
            exit_code: None,
            graceful_timeout: request.graceful_timeout,
            escalated_to_sigkill: false,
            cleanup_attempts: request.observation.cleanup_attempts,
        }
    }

    /// Project this ledger entry into an orphaned `RuntimeProcessObservation` so
    /// existing registry snapshots, TUI buffers, and gRPC streams can inspect it.
    pub fn to_orphaned_observation(&self) -> RuntimeProcessObservation {
        let mut observation = RuntimeProcessObservation::new(
            self.pid,
            self.kind.clone(),
            self.owner_reference.clone(),
        )
        .with_handle(self.handle.clone())
        .with_started_at_ms(self.started_at_ms.as_millis())
        .observed_at(self.last_observed_at_ms.as_millis());
        if let Some(command) = &self.command {
            observation = observation.with_command(command.clone());
        }
        observation.status = RuntimeProcessStatus::Orphaned;
        observation.cleanup_attempts = self.cleanup_attempts;
        observation
    }
}

/// Persistence port for the durable Marlin process ledger.
pub trait ProcessLedgerStore: fmt::Debug + Send + Sync {
    fn upsert(&self, entry: &ManagedProcessLedgerEntry) -> io::Result<()>;
    fn load_all(&self) -> io::Result<Vec<ManagedProcessLedgerEntry>>;

    fn load_non_terminal(&self) -> io::Result<Vec<ManagedProcessLedgerEntry>> {
        Ok(self
            .load_all()?
            .into_iter()
            .filter(ManagedProcessLedgerEntry::is_non_terminal)
            .collect())
    }
}

/// In-memory ledger store for deterministic unit tests and ephemeral runtimes.
#[derive(Clone, Debug, Default)]
pub struct InMemoryProcessLedgerStore {
    entries: Arc<Mutex<BTreeMap<(RuntimeHostRunId, u32), ManagedProcessLedgerEntry>>>,
}

impl InMemoryProcessLedgerStore {
    /// Create an empty in-memory process ledger store.
    pub fn new() -> Self {
        Self::default()
    }
}

impl ProcessLedgerStore for InMemoryProcessLedgerStore {
    fn upsert(&self, entry: &ManagedProcessLedgerEntry) -> io::Result<()> {
        self.entries
            .lock()
            .insert(entry.ledger_key(), entry.clone());
        Ok(())
    }

    fn load_all(&self) -> io::Result<Vec<ManagedProcessLedgerEntry>> {
        Ok(self.entries.lock().values().cloned().collect())
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct FileLedgerSnapshot {
    schema_version: u32,
    entries: Vec<ManagedProcessLedgerEntry>,
}

const FILE_LEDGER_SCHEMA_VERSION: u32 = 1;
const MAX_TERMINAL_LEDGER_ENTRIES: usize = 512;

/// Crash-safe file-backed process ledger using exclusive file locking and atomic
/// write-fsync-rename updates so abrupt `SIGKILL` of Marlin preserves prior state.
#[derive(Clone, Debug)]
pub struct FileProcessLedgerStore {
    path: PathBuf,
    lock_path: PathBuf,
    in_process_lock: Arc<Mutex<()>>,
}

impl FileProcessLedgerStore {
    /// Bind a crash-safe JSON process ledger to `path`.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let mut lock_os = path.as_os_str().to_os_string();
        lock_os.push(".lock");
        Self {
            path,
            lock_path: PathBuf::from(lock_os),
            in_process_lock: Arc::new(Mutex::new(())),
        }
    }

    /// Return the path of the backing ledger file.
    pub fn path(&self) -> &Path {
        self.path.as_path()
    }

    fn ensure_parent_dir(&self) -> io::Result<()> {
        if let Some(parent) = self.path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent)?;
        }
        Ok(())
    }

    fn acquire_file_lock(&self) -> io::Result<FileLockGuard> {
        self.ensure_parent_dir()?;
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(&self.lock_path)?;
        #[cfg(unix)]
        {
            use std::os::fd::AsRawFd;
            // SAFETY: exclusive flock on a valid owned file descriptor.
            if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) } != 0 {
                return Err(io::Error::last_os_error());
            }
        }
        Ok(FileLockGuard { file })
    }

    fn read_snapshot_unlocked(
        &self,
    ) -> io::Result<BTreeMap<(RuntimeHostRunId, u32), ManagedProcessLedgerEntry>> {
        if !self.path.exists() {
            return Ok(BTreeMap::new());
        }
        let mut raw = String::new();
        File::open(&self.path)?.read_to_string(&mut raw)?;
        if raw.trim().is_empty() {
            return Ok(BTreeMap::new());
        }
        let snapshot: FileLedgerSnapshot = serde_json::from_str(&raw).map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("corrupt process ledger {}: {error}", self.path.display()),
            )
        })?;
        Ok(snapshot
            .entries
            .into_iter()
            .map(|entry| (entry.ledger_key(), entry))
            .collect())
    }

    fn write_snapshot_unlocked(
        &self,
        entries: &BTreeMap<(RuntimeHostRunId, u32), ManagedProcessLedgerEntry>,
    ) -> io::Result<()> {
        self.ensure_parent_dir()?;
        let (mut active, mut terminal): (Vec<_>, Vec<_>) = entries
            .values()
            .cloned()
            .partition(ManagedProcessLedgerEntry::is_non_terminal);
        if terminal.len() > MAX_TERMINAL_LEDGER_ENTRIES {
            terminal.sort_by_key(|entry| entry.last_observed_at_ms);
            terminal.drain(0..(terminal.len() - MAX_TERMINAL_LEDGER_ENTRIES));
        }
        active.extend(terminal);
        let snapshot = FileLedgerSnapshot {
            schema_version: FILE_LEDGER_SCHEMA_VERSION,
            entries: active,
        };
        let encoded = serde_json::to_vec_pretty(&snapshot).map_err(io::Error::other)?;
        let seq = NEXT_TEMP_FILE_SEQ.fetch_add(1, Ordering::Relaxed);
        let mut tmp_os = self.path.as_os_str().to_os_string();
        tmp_os.push(format!(".tmp.{}.{seq}", std::process::id()));
        let tmp_path = PathBuf::from(tmp_os);
        let write_result = (|| -> io::Result<()> {
            let mut tmp = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&tmp_path)?;
            tmp.write_all(&encoded)?;
            tmp.sync_all()?;
            drop(tmp);
            fs::rename(&tmp_path, &self.path)?;
            Ok(())
        })();
        if write_result.is_err() {
            let _ = fs::remove_file(&tmp_path);
        }
        write_result
    }
}

struct FileLockGuard {
    file: File,
}

impl Drop for FileLockGuard {
    fn drop(&mut self) {
        #[cfg(unix)]
        {
            use std::os::fd::AsRawFd;
            // SAFETY: releasing flock on valid owned file descriptor.
            let _ = unsafe { libc::flock(self.file.as_raw_fd(), libc::LOCK_UN) };
        }
    }
}

impl ProcessLedgerStore for FileProcessLedgerStore {
    fn upsert(&self, entry: &ManagedProcessLedgerEntry) -> io::Result<()> {
        let _guard = self.in_process_lock.lock();
        let _file_lock = self.acquire_file_lock()?;
        let mut entries = self.read_snapshot_unlocked()?;
        entries.insert(entry.ledger_key(), entry.clone());
        self.write_snapshot_unlocked(&entries)
    }

    fn load_all(&self) -> io::Result<Vec<ManagedProcessLedgerEntry>> {
        let _guard = self.in_process_lock.lock();
        let _file_lock = self.acquire_file_lock()?;
        Ok(self.read_snapshot_unlocked()?.into_values().collect())
    }
}
