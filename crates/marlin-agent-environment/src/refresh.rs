//! Refreshes runtime environment state without owning direnv/devenv caches.

use std::collections::BTreeMap;

use marlin_agent_protocol::{
    RuntimeEnvironment, RuntimeEnvironmentRefreshCachePolicy, RuntimeEnvironmentRefreshExecution,
    RuntimeEnvironmentRefreshReceipt,
};
use tokio::task::JoinHandle;

use crate::{
    DirenvCommandRunner, ProcessDirenvCommandRunner, RuntimeEnvironmentActivationRequest,
    RuntimeEnvironmentActivator,
};

/// Input used to refresh one runtime environment snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeEnvironmentRefreshRequest {
    pub environment: RuntimeEnvironment,
    pub base_environment: BTreeMap<String, String>,
    pub execution: RuntimeEnvironmentRefreshExecution,
    pub cache_policy: RuntimeEnvironmentRefreshCachePolicy,
}

impl RuntimeEnvironmentRefreshRequest {
    pub fn new(
        environment: RuntimeEnvironment,
        base_environment: BTreeMap<String, String>,
    ) -> Self {
        Self {
            environment,
            base_environment,
            execution: RuntimeEnvironmentRefreshExecution::Foreground,
            cache_policy: RuntimeEnvironmentRefreshCachePolicy::ExternalToolOwned,
        }
    }

    pub fn background(mut self) -> Self {
        self.execution = RuntimeEnvironmentRefreshExecution::Background;
        self
    }

    pub fn with_cache_policy(mut self, cache_policy: RuntimeEnvironmentRefreshCachePolicy) -> Self {
        self.cache_policy = cache_policy;
        self
    }
}

/// Refreshed environment plus typed refresh receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeEnvironmentRefreshResult {
    pub environment: BTreeMap<String, String>,
    pub receipt: RuntimeEnvironmentRefreshReceipt,
}

/// Runtime task boundary for background environment refresh work.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RuntimeEnvironmentRefreshTaskOwner;

impl RuntimeEnvironmentRefreshTaskOwner {
    /// Runtime task boundary: owns the background refresh task lifecycle and returns its handle.
    pub fn spawn<R>(
        runner: R,
        request: RuntimeEnvironmentRefreshRequest,
    ) -> JoinHandle<RuntimeEnvironmentRefreshResult>
    where
        R: Clone + DirenvCommandRunner + Send + Sync + 'static,
    {
        tokio::spawn(async move {
            RuntimeEnvironmentRefresher::with_runner(runner)
                .refresh(request.background())
                .await
        })
    }
}

/// Applies refresh policy with an injectable direnv command runner.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeEnvironmentRefresher<R = ProcessDirenvCommandRunner> {
    runner: R,
}

impl RuntimeEnvironmentRefresher<ProcessDirenvCommandRunner> {
    pub fn new() -> Self {
        Self {
            runner: ProcessDirenvCommandRunner,
        }
    }
}

impl Default for RuntimeEnvironmentRefresher<ProcessDirenvCommandRunner> {
    fn default() -> Self {
        Self::new()
    }
}

impl<R> RuntimeEnvironmentRefresher<R>
where
    R: Clone + DirenvCommandRunner,
{
    pub fn with_runner(runner: R) -> Self {
        Self { runner }
    }

    pub async fn refresh(
        &self,
        request: RuntimeEnvironmentRefreshRequest,
    ) -> RuntimeEnvironmentRefreshResult {
        let execution = request.execution.clone();
        let cache_policy = request.cache_policy.clone();
        let activation = RuntimeEnvironmentActivator::with_runner(self.runner.clone())
            .activate(RuntimeEnvironmentActivationRequest::new(
                request.environment,
                request.base_environment,
            ))
            .await;
        let receipt = RuntimeEnvironmentRefreshReceipt::from_activation(
            execution,
            cache_policy,
            activation.receipt,
        );

        RuntimeEnvironmentRefreshResult {
            environment: activation.environment,
            receipt,
        }
    }

    pub fn spawn_background_refresh(
        &self,
        request: RuntimeEnvironmentRefreshRequest,
    ) -> JoinHandle<RuntimeEnvironmentRefreshResult>
    where
        R: Send + Sync + 'static,
    {
        RuntimeEnvironmentRefreshTaskOwner::spawn(self.runner.clone(), request)
    }
}
