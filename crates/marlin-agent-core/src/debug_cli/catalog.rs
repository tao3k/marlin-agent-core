//! Typed executor registration catalog for the `marlin` debug CLI.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use crate::{
    ExecutorName, GraphNodeInvocation, ProviderNodeAdapter, ProviderRuntime, RuntimeContext,
    RuntimeFuture, SubAgentNodeAdapter, SubAgentRuntime, TokioGraphLoopKernel, ToolNodeAdapter,
    ToolRuntime,
};

use serde::Deserialize;

use super::process_command::{
    DebugProcessCommandRuntime, ProcessCommandBinding, process_command_receipt,
};

/// Debug executor registry installed into a `TokioGraphLoopKernel`.
#[derive(Clone, Debug, Default)]
pub(super) struct DebugExecutorCatalog {
    registrations: BTreeMap<ExecutorName, DebugExecutorRegistration>,
}

impl DebugExecutorCatalog {
    /// Creates an empty debug executor catalog.
    pub(super) fn new() -> Self {
        Self::default()
    }

    /// Creates the catalog used by the current `marlin` debug CLI.
    pub(super) fn with_builtin_debug_executors() -> Self {
        Self::new()
            .register_runtime_binding(
                "debug.echo",
                DebugExecutorAdapter::Tool,
                DebugRuntimeRegistration::DebugEcho,
            )
            .register_runtime_binding(
                "debug.provider.echo",
                DebugExecutorAdapter::Provider,
                DebugRuntimeRegistration::DebugEcho,
            )
            .register_runtime_binding(
                "debug.subagent.echo",
                DebugExecutorAdapter::SubAgent,
                DebugRuntimeRegistration::DebugEcho,
            )
    }

    /// Loads a debug executor catalog from a `TOML` or `JSON` config file.
    pub(super) fn from_path(path: &Path) -> Result<Self, String> {
        let raw = fs::read_to_string(path)
            .map_err(|error| format!("failed to read catalog {}: {error}", path.display()))?;
        let config = if path.extension().and_then(|extension| extension.to_str()) == Some("json") {
            serde_json::from_str::<DebugExecutorCatalogConfig>(&raw)
                .map_err(|error| format!("invalid catalog JSON {}: {error}", path.display()))?
        } else {
            toml::from_str::<DebugExecutorCatalogConfig>(&raw)
                .map_err(|error| format!("invalid catalog TOML {}: {error}", path.display()))?
        };
        Self::from_config(config)
    }

    fn from_config(config: DebugExecutorCatalogConfig) -> Result<Self, String> {
        config
            .executors
            .into_iter()
            .try_fold(Self::new(), |catalog, executor| {
                executor.register_into(catalog)
            })
    }

    /// Registers a typed debug runtime binding for one executor.
    fn register_runtime_binding(
        mut self,
        executor: impl Into<ExecutorName>,
        adapter: DebugExecutorAdapter,
        runtime: DebugRuntimeRegistration,
    ) -> Self {
        self.registrations.insert(
            executor.into(),
            DebugExecutorRegistration { adapter, runtime },
        );
        self
    }

    /// Installs the registered executors into a graph-loop kernel.
    pub(super) fn install_into(
        self,
        kernel: TokioGraphLoopKernel,
    ) -> Result<TokioGraphLoopKernel, String> {
        self.registrations
            .into_iter()
            .try_fold(kernel, |kernel, (executor, registration)| {
                registration.install(executor, kernel)
            })
    }
}

#[derive(Clone, Debug, Deserialize)]
struct DebugExecutorCatalogConfig {
    executors: Vec<DebugExecutorConfig>,
}

#[derive(Clone, Debug, Deserialize)]
struct DebugExecutorConfig {
    executor: ExecutorName,
    adapter: DebugExecutorAdapter,
    runtime: DebugRuntimeBinding,
    #[serde(default)]
    command: Option<String>,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    cwd: Option<PathBuf>,
    #[serde(default)]
    env: BTreeMap<String, String>,
}

impl DebugExecutorConfig {
    fn register_into(self, catalog: DebugExecutorCatalog) -> Result<DebugExecutorCatalog, String> {
        let runtime = match self.runtime {
            DebugRuntimeBinding::DebugEcho => DebugRuntimeRegistration::DebugEcho,
            DebugRuntimeBinding::ProcessCommand => {
                if self.adapter != DebugExecutorAdapter::Tool {
                    return Err(
                        "process-command runtime binding requires adapter = \"tool\"".to_owned(),
                    );
                }
                let command = self
                    .command
                    .ok_or_else(|| "process-command runtime binding requires command".to_owned())?;
                if command.trim().is_empty() {
                    return Err("process-command runtime binding command is empty".to_owned());
                }
                DebugRuntimeRegistration::ProcessCommand(ProcessCommandBinding::new(
                    command, self.args, self.cwd, self.env,
                ))
            }
        };
        Ok(catalog.register_runtime_binding(self.executor, self.adapter, runtime))
    }
}

/// Adapter family used to install a debug executor.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum DebugExecutorAdapter {
    /// Install through `ToolNodeAdapter`.
    Tool,
    /// Install through `ProviderNodeAdapter`.
    Provider,
    /// Install through `SubAgentNodeAdapter`.
    SubAgent,
}

/// Runtime binding selected by a debug executor registration.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum DebugRuntimeBinding {
    /// In-memory echo runtime used by harness tests.
    DebugEcho,
    /// Process command runtime using explicit argv, never shell text.
    ProcessCommand,
}

#[derive(Clone, Debug)]
enum DebugRuntimeRegistration {
    DebugEcho,
    ProcessCommand(ProcessCommandBinding),
}

#[derive(Clone, Debug)]
struct DebugExecutorRegistration {
    adapter: DebugExecutorAdapter,
    runtime: DebugRuntimeRegistration,
}

impl DebugExecutorRegistration {
    fn install(
        self,
        executor: ExecutorName,
        kernel: TokioGraphLoopKernel,
    ) -> Result<TokioGraphLoopKernel, String> {
        match (self.adapter, self.runtime) {
            (DebugExecutorAdapter::Tool, DebugRuntimeRegistration::DebugEcho) => Ok(kernel
                .with_executor(
                    executor.into_string(),
                    ToolNodeAdapter::new(DebugEchoToolRuntime, |invocation| invocation),
                )),
            (DebugExecutorAdapter::Provider, DebugRuntimeRegistration::DebugEcho) => Ok(kernel
                .with_executor(
                    executor.into_string(),
                    ProviderNodeAdapter::new(DebugEchoProviderRuntime, |invocation| invocation),
                )),
            (DebugExecutorAdapter::SubAgent, DebugRuntimeRegistration::DebugEcho) => Ok(kernel
                .with_executor(
                    executor.into_string(),
                    SubAgentNodeAdapter::new(DebugEchoSubAgentRuntime, |invocation| invocation),
                )),
            (DebugExecutorAdapter::Tool, DebugRuntimeRegistration::ProcessCommand(binding)) => {
                Ok(kernel.with_executor(
                    executor.into_string(),
                    ToolNodeAdapter::with_receipt_mapper(
                        DebugProcessCommandRuntime::new(binding),
                        |invocation| invocation,
                        process_command_receipt,
                    ),
                ))
            }
            (_, DebugRuntimeRegistration::ProcessCommand(_)) => {
                Err("process-command runtime binding requires adapter = \"tool\"".to_owned())
            }
        }
    }
}

#[derive(Clone, Debug)]
struct DebugEchoToolRuntime;

impl ToolRuntime for DebugEchoToolRuntime {
    type Invocation = GraphNodeInvocation;
    type Output = GraphNodeInvocation;

    fn run_tool(
        &self,
        invocation: GraphNodeInvocation,
        _context: RuntimeContext,
    ) -> RuntimeFuture<Self::Output> {
        Box::pin(async move { invocation })
    }
}

#[derive(Clone, Debug)]
struct DebugEchoProviderRuntime;

impl ProviderRuntime for DebugEchoProviderRuntime {
    type Request = GraphNodeInvocation;
    type Response = GraphNodeInvocation;

    fn run_provider(
        &self,
        request: GraphNodeInvocation,
        _context: RuntimeContext,
    ) -> RuntimeFuture<Self::Response> {
        Box::pin(async move { request })
    }
}

#[derive(Clone, Debug)]
struct DebugEchoSubAgentRuntime;

impl SubAgentRuntime for DebugEchoSubAgentRuntime {
    type Input = GraphNodeInvocation;
    type Output = GraphNodeInvocation;

    fn run_sub_agent(
        &self,
        input: GraphNodeInvocation,
        _context: RuntimeContext,
    ) -> RuntimeFuture<Self::Output> {
        Box::pin(async move { input })
    }
}
