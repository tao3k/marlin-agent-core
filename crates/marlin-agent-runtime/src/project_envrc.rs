// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! One Rust-owned environment binding for an admitted GitOps worktree.

use std::{collections::BTreeMap, path::Path, sync::Arc, time::Duration};

use crate::environment::{
    ProcessDirenvCommandRunner, RuntimeEnvironmentActivationRequest, RuntimeEnvironmentActivator,
};
use marlin_agent_gitops::working_copy::WorkingCopyGitTopLevel;
use marlin_agent_protocol::{
    RuntimeEnvironment, RuntimeEnvironmentActivationPolicy, RuntimeEnvironmentActivationStatus,
};
use parking_lot::Mutex;
use tokio::sync::watch;
use tracing::Instrument;

use crate::{CancellationToken, TokioAgentRuntime, observability::TARGET_RUNTIME_SPAN};

const LOAD_TIMEOUT: Duration = Duration::from_secs(30);

/// An intent authored by startup or an explicit host command.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EnvrcIntent {
    Activate,
    Allow,
    Reload,
    Deny,
}

impl EnvrcIntent {
    pub const fn symbol(self) -> &'static str {
        match self {
            Self::Activate => "activate",
            Self::Allow => "allow",
            Self::Reload => "reload",
            Self::Deny => "deny",
        }
    }
}

/// Typed plan returned by the Scheme ENVRC Module. Admission binds it to a worktree.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EnvrcPlan {
    Absent,
    Export,
    AllowAndExport,
    ReloadAndExport,
    Deny,
}

impl EnvrcPlan {
    pub fn decode(intent: EnvrcIntent, exists: bool, symbol: &str) -> Option<Self> {
        match (intent, exists, symbol) {
            (_, false, "absent") => Some(Self::Absent),
            (EnvrcIntent::Activate, true, "export") => Some(Self::Export),
            (EnvrcIntent::Allow, true, "allow-and-export") => Some(Self::AllowAndExport),
            (EnvrcIntent::Reload, true, "reload-and-export") => Some(Self::ReloadAndExport),
            (EnvrcIntent::Deny, true, "deny") => Some(Self::Deny),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EnvrcStatus {
    Absent,
    Loading,
    Ready,
    Blocked,
    Denied,
    TimedOut,
    Cancelled,
    Failed,
}

impl EnvrcStatus {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Absent => "absent",
            Self::Loading => "loading",
            Self::Ready => "ready",
            Self::Blocked => "blocked",
            Self::Denied => "denied",
            Self::TimedOut => "timed out",
            Self::Cancelled => "cancelled",
            Self::Failed => "failed",
        }
    }
}

pub(crate) type EnvrcChanges = Arc<BTreeMap<String, Option<String>>>;

#[derive(Clone)]
enum Snapshot {
    Status(EnvrcStatus),
    Ready(EnvrcChanges),
}

impl Snapshot {
    fn status(&self) -> EnvrcStatus {
        match self {
            Self::Status(status) => *status,
            Self::Ready(_) => EnvrcStatus::Ready,
        }
    }
}

struct State {
    generation: u64,
    cancellation: Option<CancellationToken>,
}

#[derive(Clone, Debug)]
enum Discovery {
    Pending,
    Resolved(Option<ProjectEnvrc>),
}

/// Shares startup worktree discovery with agent tools without blocking the UI.
#[derive(Clone, Debug)]
pub struct ProjectEnvrcDiscovery(watch::Sender<Discovery>);

impl ProjectEnvrcDiscovery {
    pub fn pending() -> Self {
        Self(watch::channel(Discovery::Pending).0)
    }

    pub(crate) fn resolved(binding: ProjectEnvrc) -> Self {
        Self(watch::channel(Discovery::Resolved(Some(binding))).0)
    }

    pub fn complete(&self, binding: Option<ProjectEnvrc>) {
        self.0.send_if_modified(|current| {
            if !matches!(current, Discovery::Pending) {
                return false;
            }
            *current = Discovery::Resolved(binding);
            true
        });
    }

    pub(crate) fn binding(&self) -> Option<ProjectEnvrc> {
        match &*self.0.borrow() {
            Discovery::Resolved(binding) => binding.clone(),
            Discovery::Pending => None,
        }
    }

    pub(crate) fn status(&self) -> Option<EnvrcStatus> {
        match &*self.0.borrow() {
            Discovery::Resolved(binding) => binding.as_ref().map(ProjectEnvrc::status),
            Discovery::Pending => Some(EnvrcStatus::Loading),
        }
    }

    pub(crate) async fn wait(
        &self,
        cancellation: &CancellationToken,
    ) -> Result<Option<ProjectEnvrc>, EnvrcStatus> {
        let mut updates = self.0.subscribe();
        loop {
            if let Discovery::Resolved(binding) = updates.borrow_and_update().clone() {
                return Ok(binding);
            }
            tokio::select! {
                biased;
                () = cancellation.cancelled() => return Err(EnvrcStatus::Cancelled),
                changed = updates.changed() => {
                    if changed.is_err() {
                        return Err(EnvrcStatus::Cancelled);
                    }
                }
            }
        }
    }
}

struct Binding {
    root: WorkingCopyGitTopLevel,
    base: BTreeMap<String, String>,
    state: Mutex<State>,
    serial: tokio::sync::Mutex<()>,
    output: watch::Sender<Snapshot>,
}

/// An isolated worktree environment. Clones share only this worktree's state.
#[derive(Clone)]
pub struct ProjectEnvrc(Arc<Binding>);

impl std::fmt::Debug for ProjectEnvrc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProjectEnvrc")
            .field("root", &self.0.root)
            .field("status", &self.status())
            .finish()
    }
}

impl ProjectEnvrc {
    pub fn new(root: WorkingCopyGitTopLevel, mut base: BTreeMap<String, String>) -> Self {
        // A shell which already loaded an adjacent project's .envrc cannot
        // supply direnv's bookkeeping to this independently resolved snapshot.
        base.retain(|name, _| !name.starts_with("DIRENV_"));
        let (output, _) = watch::channel(Snapshot::Status(EnvrcStatus::Absent));
        Self(Arc::new(Binding {
            root,
            base,
            state: Mutex::new(State {
                generation: 0,
                cancellation: None,
            }),
            serial: tokio::sync::Mutex::new(()),
            output,
        }))
    }

    pub fn root(&self) -> &Path {
        self.0.root.as_path()
    }

    pub fn has_file(&self) -> std::io::Result<bool> {
        match self.root().join(".envrc").symlink_metadata() {
            Ok(metadata) if metadata.is_file() => Ok(true),
            Ok(_) => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "invalid worktree .envrc",
            )),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(error),
        }
    }

    pub fn status(&self) -> EnvrcStatus {
        self.0.output.borrow().status()
    }

    pub fn reject(&self, intent: EnvrcIntent) {
        let span = load_span(intent);
        record_outcome(&span, EnvrcStatus::Failed, std::time::Instant::now());
        let mut state = self.0.state.lock();
        if let Some(token) = state.cancellation.take() {
            token.cancel();
        }
        state.generation = state
            .generation
            .checked_add(1)
            .expect("ENVRC generation exhausted");
        self.publish(Snapshot::Status(EnvrcStatus::Failed));
    }

    pub fn cancel_pending(&self) -> bool {
        let state = self.0.state.lock();
        if self.status() == EnvrcStatus::Loading
            && let Some(token) = &state.cancellation
        {
            token.cancel();
            true
        } else {
            false
        }
    }

    /// Schedule a previously admitted Scheme plan, superseding the old load.
    pub fn submit(&self, runtime: &TokioAgentRuntime, intent: EnvrcIntent, plan: EnvrcPlan) {
        if !matches!(
            (intent, plan),
            (_, EnvrcPlan::Absent)
                | (EnvrcIntent::Activate, EnvrcPlan::Export)
                | (EnvrcIntent::Allow, EnvrcPlan::AllowAndExport)
                | (EnvrcIntent::Reload, EnvrcPlan::ReloadAndExport)
                | (EnvrcIntent::Deny, EnvrcPlan::Deny)
        ) {
            self.reject(intent);
            return;
        }
        let token = runtime.cancellation_token().child_token();
        let generation = {
            let mut state = self.0.state.lock();
            if let Some(previous) = state.cancellation.replace(token.clone()) {
                previous.cancel();
            }
            state.generation = state
                .generation
                .checked_add(1)
                .expect("ENVRC generation exhausted");
            self.publish(Snapshot::Status(EnvrcStatus::Loading));
            state.generation
        };
        let binding = self.clone();
        let span = load_span(intent);
        let started = std::time::Instant::now();
        let task = runtime.spawn(
            async move {
                let mut snapshot = tokio::select! {
                    biased;
                    () = token.cancelled() => Snapshot::Status(EnvrcStatus::Cancelled),
                    result = tokio::time::timeout(LOAD_TIMEOUT, binding.execute(plan)) => {
                        result.unwrap_or(Snapshot::Status(EnvrcStatus::TimedOut))
                    }
                };
                let state = binding.0.state.lock();
                if token.is_cancelled() {
                    snapshot = Snapshot::Status(EnvrcStatus::Cancelled);
                }
                let status = snapshot.status();
                let outcome = if state.generation == generation {
                    binding.publish(snapshot);
                    status
                } else {
                    EnvrcStatus::Cancelled
                };
                record_outcome(&tracing::Span::current(), outcome, started);
            }
            .instrument(span.clone()),
        );
        if task.admission().is_err() {
            let state = self.0.state.lock();
            if state.generation == generation {
                self.publish(Snapshot::Status(EnvrcStatus::Cancelled));
            }
            record_outcome(&span, EnvrcStatus::Cancelled, started);
        }
    }

    pub(crate) async fn changes_for(
        &self,
        root: &Path,
        cancellation: &CancellationToken,
    ) -> Result<Option<EnvrcChanges>, EnvrcStatus> {
        if self.root() != root {
            return Ok(None);
        }
        let mut output = self.0.output.subscribe();
        loop {
            match output.borrow_and_update().clone() {
                Snapshot::Ready(changes) => return Ok(Some(changes)),
                Snapshot::Status(EnvrcStatus::Absent) => return Ok(None),
                Snapshot::Status(EnvrcStatus::Loading) => {}
                Snapshot::Status(status) => return Err(status),
            }
            tokio::select! {
                biased;
                () = cancellation.cancelled() => return Err(EnvrcStatus::Cancelled),
                changed = output.changed() => {
                    if changed.is_err() {
                        return Err(EnvrcStatus::Cancelled);
                    }
                }
            }
        }
    }

    fn publish(&self, snapshot: Snapshot) {
        self.0.output.send_replace(snapshot);
    }

    async fn execute(&self, plan: EnvrcPlan) -> Snapshot {
        let _serial = self.0.serial.lock().await;
        match (self.has_file(), plan) {
            (Ok(false), _) => return Snapshot::Status(EnvrcStatus::Absent),
            (Ok(true), EnvrcPlan::Absent) | (Err(_), _) => {
                return Snapshot::Status(EnvrcStatus::Failed);
            }
            _ => {}
        }
        let root = self.root();
        let base = &self.0.base;
        let runner = ProcessDirenvCommandRunner;
        if matches!(plan, EnvrcPlan::AllowAndExport | EnvrcPlan::Deny) {
            if runner
                .authorize_project(root, base, plan != EnvrcPlan::Deny)
                .await
                .is_err()
            {
                return Snapshot::Status(EnvrcStatus::Failed);
            }
            if plan == EnvrcPlan::Deny {
                return Snapshot::Status(EnvrcStatus::Denied);
            }
        }
        match runner.project_allowed(root, base).await {
            Ok(true) => {}
            Ok(false) => return Snapshot::Status(EnvrcStatus::Blocked),
            Err(_) => return Snapshot::Status(EnvrcStatus::Failed),
        }
        let mut policy = RuntimeEnvironmentActivationPolicy::direnv_file(root.join(".envrc"));
        if plan == EnvrcPlan::ReloadAndExport {
            policy = policy.with_direnv_reload();
        }
        let environment = RuntimeEnvironment::default()
            .with_cwd(root)
            .with_activation(policy);
        let result = RuntimeEnvironmentActivator::new()
            .activate(RuntimeEnvironmentActivationRequest::new(
                environment,
                base.clone(),
            ))
            .await;
        if result.receipt.status != RuntimeEnvironmentActivationStatus::Applied {
            return Snapshot::Status(EnvrcStatus::Failed);
        }
        let mut changes = BTreeMap::new();
        for (name, value) in result.environment {
            if !name.starts_with("DIRENV_") && base.get(&name) != Some(&value) {
                changes.insert(name, Some(value));
            }
        }
        for name in result.receipt.delta.removed {
            if !name.starts_with("DIRENV_") {
                changes.insert(name, None);
            }
        }
        Snapshot::Ready(Arc::new(changes))
    }
}

fn load_span(intent: EnvrcIntent) -> tracing::Span {
    tracing::info_span!(
        target: TARGET_RUNTIME_SPAN, "envrc.load", intent = intent.symbol(),
        outcome = tracing::field::Empty, elapsed_ms = tracing::field::Empty,
    )
}

fn record_outcome(span: &tracing::Span, status: EnvrcStatus, started: std::time::Instant) {
    span.record("outcome", status.label());
    span.record(
        "elapsed_ms",
        u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
    );
}

#[cfg(all(test, unix))]
#[path = "../tests/unit/project_envrc.rs"]
mod tests;
