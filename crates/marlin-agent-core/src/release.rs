// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Release receipt bridges exposed by the core facade.

use marlin_agent_context::org::store::{FileSystemReleaseStatusStore, OrgSourceStoreResult};
use marlin_agent_context::org::workflow::{
    GerbilReleaseStatusCommit, GerbilReleaseStatusCommitReceipt, GerbilReleaseStatusCommitter,
};
use marlin_agent_harness::{
    ReleaseGateExecutionReceipt, ReleaseGateExecutionStatus, release_gate_execution_receipt,
};
use marlin_gerbil_ir::{ReleaseGateSpec, ReleaseTopologySpec};
use marlin_workspace::status::{ReleaseGateReceipt, ReleaseGateState};
use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const MAX_RELEASE_GATE_OUTPUT_BYTES: usize = 256 * 1024;
const DEFAULT_RELEASE_GATE_TIMEOUT: Duration = Duration::from_secs(60);
const RELEASE_GATE_SAFE_ENV_ALLOWLIST: &[&str] = &[
    "PATH",
    "HOME",
    "TMPDIR",
    "TEMP",
    "TMP",
    "USER",
    "LOGNAME",
    "LANG",
    "LC_ALL",
    "LC_CTYPE",
    "TERM",
    "CARGO_HOME",
    "RUSTUP_HOME",
    "GERBIL_HOME",
    "GERBIL_LOADPATH",
    "NIX_SSL_CERT_FILE",
    "SSL_CERT_FILE",
];

fn apply_scrubbed_release_env(cmd: &mut Command) {
    cmd.env_clear();
    let mut has_path = false;
    for key in RELEASE_GATE_SAFE_ENV_ALLOWLIST {
        if let Some(val) = std::env::var_os(key) {
            if *key == "PATH" {
                has_path = true;
            }
            cmd.env(key, val);
        }
    }
    if !has_path {
        cmd.env("PATH", "/usr/bin:/bin");
    }
}

fn read_bounded_drain<R: Read>(mut reader: R, max_bytes: usize) -> (Vec<u8>, bool) {
    let mut kept = Vec::new();
    let mut truncated = false;
    let mut buf = [0u8; 4096];
    loop {
        match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                if kept.len() < max_bytes {
                    let take = n.min(max_bytes - kept.len());
                    kept.extend_from_slice(&buf[..take]);
                    if take < n {
                        truncated = true;
                    }
                } else {
                    truncated = true;
                }
            }
            Err(err) if err.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
    }
    (kept, truncated)
}

/// Result of executing one release gate command.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleaseGateCommandOutput {
    pub success: bool,
    pub status_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

impl ReleaseGateCommandOutput {
    /// Creates a successful command output fixture or adapter result.
    pub fn passed() -> Self {
        Self {
            success: true,
            status_code: Some(0),
            stdout: String::new(),
            stderr: String::new(),
        }
    }

    /// Creates a failed command output fixture or adapter result.
    pub fn failed(status_code: Option<i32>, stderr: impl Into<String>) -> Self {
        Self {
            success: false,
            status_code,
            stdout: String::new(),
            stderr: stderr.into(),
        }
    }
}

/// Runner abstraction used by release gate execution.
pub trait ReleaseGateCommandRunner {
    fn run_release_gate_command(&self, command: &str) -> ReleaseGateCommandOutput;
}

/// Process-backed release gate runner.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProcessReleaseGateCommandRunner;

impl ReleaseGateCommandRunner for ProcessReleaseGateCommandRunner {
    fn run_release_gate_command(&self, command: &str) -> ReleaseGateCommandOutput {
        if command.trim().is_empty() || command.contains('\0') {
            return ReleaseGateCommandOutput::failed(
                None,
                "release gate command must be non-empty without NUL bytes",
            );
        }

        let mut cmd = Command::new("sh");
        apply_scrubbed_release_env(&mut cmd);
        cmd.arg("-c")
            .arg(command)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = match cmd.spawn() {
            Ok(child) => child,
            Err(error) => return ReleaseGateCommandOutput::failed(None, error.to_string()),
        };

        let stdout_pipe = child.stdout.take();
        let stderr_pipe = child.stderr.take();
        let deadline = Instant::now() + DEFAULT_RELEASE_GATE_TIMEOUT;

        std::thread::scope(|scope| {
            let out_handle = scope.spawn(|| {
                stdout_pipe
                    .map(|p| read_bounded_drain(p, MAX_RELEASE_GATE_OUTPUT_BYTES))
                    .unwrap_or_else(|| (Vec::new(), false))
            });
            let err_handle = scope.spawn(|| {
                stderr_pipe
                    .map(|p| read_bounded_drain(p, MAX_RELEASE_GATE_OUTPUT_BYTES))
                    .unwrap_or_else(|| (Vec::new(), false))
            });

            let mut timed_out = false;
            let status = loop {
                match child.try_wait() {
                    Ok(Some(st)) => break Ok(st),
                    Ok(None) => {
                        if Instant::now() >= deadline {
                            timed_out = true;
                            let _ = child.kill();
                            break child.wait();
                        }
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    Err(err) => {
                        let _ = child.kill();
                        let _ = child.wait();
                        break Err(err);
                    }
                }
            };

            let (out_bytes, _out_trunc) = out_handle.join().unwrap_or_else(|_| (Vec::new(), false));
            let (err_bytes, _err_trunc) = err_handle.join().unwrap_or_else(|_| (Vec::new(), false));

            if timed_out {
                return ReleaseGateCommandOutput::failed(
                    None,
                    format!(
                        "release gate command exceeded {}s timeout",
                        DEFAULT_RELEASE_GATE_TIMEOUT.as_secs()
                    ),
                );
            }
            match status {
                Ok(st) => ReleaseGateCommandOutput {
                    success: st.success(),
                    status_code: st.code(),
                    stdout: String::from_utf8_lossy(&out_bytes).trim().to_owned(),
                    stderr: String::from_utf8_lossy(&err_bytes).trim().to_owned(),
                },
                Err(error) => ReleaseGateCommandOutput::failed(None, error.to_string()),
            }
        })
    }
}

/// Controls whether runner-owned execution may invoke external Gerbil/Gambit gates.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ReleaseGateRunOptions {
    pub run_local_gerbil_gates: bool,
}

impl ReleaseGateRunOptions {
    /// Creates options that allow manually requested local Gerbil/Gambit gates.
    pub fn with_local_gerbil_gates() -> Self {
        Self {
            run_local_gerbil_gates: true,
        }
    }
}

/// Request used to execute and persist one release gate.
#[derive(Clone, Copy, Debug)]
pub struct ReleaseGateRecordRequest<'a> {
    pub store: &'a FileSystemReleaseStatusStore,
    pub topology: &'a ReleaseTopologySpec,
    pub gate: &'a ReleaseGateSpec,
    pub options: ReleaseGateRunOptions,
}

impl<'a> ReleaseGateRecordRequest<'a> {
    /// Creates a request for one release gate execution and status-store write.
    pub fn new(
        store: &'a FileSystemReleaseStatusStore,
        topology: &'a ReleaseTopologySpec,
        gate: &'a ReleaseGateSpec,
    ) -> Self {
        Self {
            store,
            topology,
            gate,
            options: ReleaseGateRunOptions::default(),
        }
    }

    /// Sets runner options for this release gate execution.
    pub fn with_options(mut self, options: ReleaseGateRunOptions) -> Self {
        self.options = options;
        self
    }
}

/// Persist a Gerbil release status commit from harness execution receipts.
pub fn commit_release_gate_execution_receipts(
    store: &FileSystemReleaseStatusStore,
    topology: ReleaseTopologySpec,
    receipts: &[ReleaseGateExecutionReceipt],
) -> OrgSourceStoreResult<GerbilReleaseStatusCommitReceipt> {
    let commit = gerbil_release_status_commit_from_execution_receipts(topology, receipts);
    GerbilReleaseStatusCommitter::commit(store, &commit)
}

/// Build a Gerbil release workflow commit from harness execution receipts.
pub fn gerbil_release_status_commit_from_execution_receipts(
    topology: ReleaseTopologySpec,
    receipts: &[ReleaseGateExecutionReceipt],
) -> GerbilReleaseStatusCommit {
    receipts.iter().fold(
        GerbilReleaseStatusCommit::new(topology),
        |commit, receipt| commit.with_gate_receipt(release_gate_status_receipt(receipt)),
    )
}

/// Record a harness release gate execution receipt in a file-backed workspace status sidecar.
pub fn record_release_gate_execution_receipt(
    store: &FileSystemReleaseStatusStore,
    receipt: &ReleaseGateExecutionReceipt,
) -> OrgSourceStoreResult<bool> {
    store.record_release_gate_receipt(release_gate_status_receipt(receipt))
}

/// Execute one release gate and return a typed harness receipt.
pub fn execute_release_gate_with_runner<R>(
    topology: &ReleaseTopologySpec,
    gate: &ReleaseGateSpec,
    options: ReleaseGateRunOptions,
    runner: &R,
) -> ReleaseGateExecutionReceipt
where
    R: ReleaseGateCommandRunner,
{
    if gate.requires_local_gerbil && !options.run_local_gerbil_gates {
        return release_gate_execution_receipt(topology, gate, ReleaseGateExecutionStatus::Skipped)
            .with_diagnostics([
                "release_gate.skipped".to_owned(),
                "release_gate.requires_local_gerbil=true".to_owned(),
                "release_gate.local_gerbil_not_requested".to_owned(),
            ]);
    }

    release_gate_execution_receipt_from_output(
        topology,
        gate,
        runner.run_release_gate_command(&gate.command),
    )
}

/// Execute one release gate with the default process runner.
pub fn execute_release_gate(
    topology: &ReleaseTopologySpec,
    gate: &ReleaseGateSpec,
    options: ReleaseGateRunOptions,
) -> ReleaseGateExecutionReceipt {
    execute_release_gate_with_runner(topology, gate, options, &ProcessReleaseGateCommandRunner)
}

/// Execute and persist one release gate receipt in the release status store.
pub fn execute_and_record_release_gate_with_runner<R>(
    request: ReleaseGateRecordRequest<'_>,
    runner: &R,
) -> OrgSourceStoreResult<ReleaseGateExecutionReceipt>
where
    R: ReleaseGateCommandRunner,
{
    let receipt =
        execute_release_gate_with_runner(request.topology, request.gate, request.options, runner);
    record_release_gate_execution_receipt(request.store, &receipt)?;
    Ok(receipt)
}

/// Convert one command result into a release gate execution receipt.
pub fn release_gate_execution_receipt_from_output(
    topology: &ReleaseTopologySpec,
    gate: &ReleaseGateSpec,
    output: ReleaseGateCommandOutput,
) -> ReleaseGateExecutionReceipt {
    let status = if output.success {
        ReleaseGateExecutionStatus::Passed
    } else {
        ReleaseGateExecutionStatus::Failed
    };
    let mut diagnostics = vec![format!(
        "release_gate.status_code={}",
        output
            .status_code
            .map(|code| code.to_string())
            .unwrap_or_else(|| "none".to_owned())
    )];
    if !output.stdout.is_empty() {
        diagnostics.push(format!("release_gate.stdout={}", output.stdout));
    }
    if !output.stderr.is_empty() {
        diagnostics.push(format!("release_gate.stderr={}", output.stderr));
    }

    release_gate_execution_receipt(topology, gate, status).with_diagnostics(diagnostics)
}

/// Convert a harness release gate execution receipt into a workspace status receipt.
pub fn release_gate_status_receipt(receipt: &ReleaseGateExecutionReceipt) -> ReleaseGateReceipt {
    ReleaseGateReceipt {
        gate_id: receipt.gate_id.clone(),
        state: release_gate_state_from_execution(receipt),
        evidence_keys: receipt.evidence_keys.clone(),
        artifact_paths: receipt.artifact_paths.clone(),
        diagnostics: receipt.diagnostics.clone(),
    }
}

/// Project harness execution status into the workspace release gate state model.
pub fn release_gate_state_from_execution(
    receipt: &ReleaseGateExecutionReceipt,
) -> ReleaseGateState {
    match receipt.status {
        ReleaseGateExecutionStatus::Expected if receipt.requires_local_gerbil => {
            ReleaseGateState::RequiresLocalGerbil
        }
        ReleaseGateExecutionStatus::Expected => ReleaseGateState::Pending,
        ReleaseGateExecutionStatus::Passed => ReleaseGateState::Passed,
        ReleaseGateExecutionStatus::Failed => ReleaseGateState::Failed,
        ReleaseGateExecutionStatus::Skipped => ReleaseGateState::Skipped,
    }
}

#[cfg(test)]
mod sm26_release_gate_adversarial_tests {
    use super::*;

    #[test]
    fn sm26_adv_l4_release_gate_shell_must_scrub_host_secrets_and_bound_output() {
        // Inject canary secrets into parent process environment (edition 2024 unsafe).
        let canary_openai = "sk-live-sm26-release-canary-998877";
        let canary_bridge = "tok-sm26-bridge-canary-aabbcc";
        unsafe {
            std::env::set_var("OPENAI_API_KEY", canary_openai);
            std::env::set_var("MARLIN_BRIDGE_TOKEN", canary_bridge);
        }

        let runner = ProcessReleaseGateCommandRunner;
        let out = runner.run_release_gate_command(
            "printf 'OPENAI=%s|BRIDGE=%s' \"$OPENAI_API_KEY\" \"$MARLIN_BRIDGE_TOKEN\"",
        );

        unsafe {
            std::env::remove_var("OPENAI_API_KEY");
            std::env::remove_var("MARLIN_BRIDGE_TOKEN");
        }

        assert!(
            !out.stdout.contains(canary_openai) && !out.stdout.contains(canary_bridge),
            "ADV-L4 RELEASE SHELL SECRET EXFILTRATION PROVEN: leaked host credentials into release gate stdout: {:?}",
            out.stdout
        );
        assert!(
            out.success,
            "clean benign release command should still execute with scrubbed env: stderr={}",
            out.stderr
        );
    }
}
