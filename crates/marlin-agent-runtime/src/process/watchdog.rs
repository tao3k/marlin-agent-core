// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Monotonic host, process activity, and command deadline observations.

use std::{future::Future, pin::Pin, sync::Arc, time::Duration};

use parking_lot::Mutex;
use tokio::time::{Instant, timeout_at};

use crate::RuntimeResourceId;

/// Observes advisory health for one admitted resource.
#[derive(Clone, Debug)]
pub struct ProcessWatchdog(pub(super) RuntimeResourceId);

/// Observes the command deadline armed by the managed process owner.
pub struct ProcessCommandWatchdog {
    pub(super) resource: RuntimeResourceId,
    pub(super) deadline: Instant,
}

/// Evidence that the observation deadline elapsed for a particular resource.
#[derive(Clone, Debug)]
pub struct ProcessDeadlineElapsed(RuntimeResourceId);

/// Heartbeats emitted by the host's own event loop.
#[derive(Clone, Debug)]
pub struct HostHeartbeat(Arc<Mutex<Instant>>);

impl HostHeartbeat {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(Instant::now())))
    }

    /// Record progress at the host, independent of task output.
    pub fn pulse(&self) {
        *self.0.lock() = Instant::now();
    }
}

impl Default for HostHeartbeat {
    fn default() -> Self {
        Self::new()
    }
}

/// Independent advisory silence thresholds for the host and a process.
#[derive(Clone, Copy, Debug)]
pub struct ProcessHealthPolicy {
    host_silence: Duration,
    output_silence: Duration,
}

impl ProcessHealthPolicy {
    pub fn new(host_silence: Duration, output_silence: Duration) -> Self {
        Self {
            host_silence,
            output_silence,
        }
    }

    pub(crate) fn sample_period(self) -> Duration {
        (self.host_silence.min(self.output_silence) / 2)
            .clamp(Duration::from_millis(1), Duration::from_millis(250))
    }
}

/// An advisory condition observed for a managed process.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProcessHealthCondition {
    HostHeartbeatMissed,
    TaskOutputIdle,
}

/// Advisory health evidence for the resource bound when monitoring began.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcessHealthReport {
    resource: RuntimeResourceId,
    condition: ProcessHealthCondition,
}

impl ProcessHealthReport {
    pub fn resource_id(&self) -> &RuntimeResourceId {
        &self.resource
    }

    pub fn condition(&self) -> ProcessHealthCondition {
        self.condition
    }
}

/// Typed watchdog evidence offered to the Runtime owner for a decision.
pub enum ProcessWatchdogReport<'a> {
    Health(&'a ProcessHealthReport),
    CommandDeadline(&'a ProcessDeadlineElapsed),
}

/// A per-generation observer for host heartbeat and process output silence.
pub struct ProcessHealthMonitor {
    resource: RuntimeResourceId,
    host: HostHeartbeat,
    policy: ProcessHealthPolicy,
    started: Instant,
    reported_host: Option<Instant>,
    reported_output: Option<Instant>,
}

impl ProcessHealthMonitor {
    /// Report each silence condition once per observed heartbeat or output timestamp.
    pub fn sample(
        &mut self,
        last_output: Option<Instant>,
    ) -> impl Iterator<Item = ProcessHealthReport> + use<> {
        let host = (*self.host.0.lock()).max(self.started);
        let output = last_output.unwrap_or(self.started).max(self.started);
        let now = Instant::now();
        let report = |evidence, limit, reported: &mut Option<Instant>, condition| {
            if now.saturating_duration_since(evidence) >= limit && *reported != Some(evidence) {
                *reported = Some(evidence);
                Some(ProcessHealthReport {
                    resource: self.resource.clone(),
                    condition,
                })
            } else {
                None
            }
        };
        [
            report(
                host,
                self.policy.host_silence,
                &mut self.reported_host,
                ProcessHealthCondition::HostHeartbeatMissed,
            ),
            report(
                output,
                self.policy.output_silence,
                &mut self.reported_output,
                ProcessHealthCondition::TaskOutputIdle,
            ),
        ]
        .into_iter()
        .flatten()
    }
}

impl ProcessDeadlineElapsed {
    /// The original resource identity captured before observation began.
    pub fn resource_id(&self) -> &RuntimeResourceId {
        &self.0
    }
}

impl ProcessWatchdog {
    /// Observe health without obtaining a process handle or cleanup capability.
    pub fn monitor_health(
        &self,
        host: HostHeartbeat,
        policy: ProcessHealthPolicy,
    ) -> ProcessHealthMonitor {
        ProcessHealthMonitor {
            resource: self.0.clone(),
            host,
            policy,
            started: Instant::now(),
            reported_host: None,
            reported_output: None,
        }
    }
}

impl ProcessCommandWatchdog {
    /// Poll borrowed work until the armed deadline. The caller retains the
    /// future and its resources when this returns an elapsed observation.
    pub async fn observe_until<F: Future>(
        self,
        work: Pin<&mut F>,
    ) -> Result<F::Output, ProcessDeadlineElapsed> {
        timeout_at(self.deadline, work)
            .await
            .map_err(|_| ProcessDeadlineElapsed(self.resource))
    }
}
