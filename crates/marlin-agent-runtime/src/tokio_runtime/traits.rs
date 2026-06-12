//! Runtime extension traits.

use std::{future::Future, pin::Pin};

use super::RuntimeContext;

/// Boxed async work item used by runtime extension traits.
pub type RuntimeFuture<T> = Pin<Box<dyn Future<Output = T> + Send + 'static>>;

/// Provider boundary for model or completion runtimes.
pub trait ProviderRuntime: Send + Sync + 'static {
    type Request: Send + 'static;
    type Response: Send + 'static;

    fn run_provider(
        &self,
        request: Self::Request,
        context: RuntimeContext,
    ) -> RuntimeFuture<Self::Response>;
}

/// Tool boundary for native or external tool execution.
pub trait ToolRuntime: Send + Sync + 'static {
    type Invocation: Send + 'static;
    type Output: Send + 'static;

    fn run_tool(
        &self,
        invocation: Self::Invocation,
        context: RuntimeContext,
    ) -> RuntimeFuture<Self::Output>;
}

/// Hook boundary for runtime interception and observation.
pub trait HookRuntime: Send + Sync + 'static {
    type Request: Send + 'static;
    type Output: Send + 'static;

    fn run_hook(
        &self,
        request: Self::Request,
        context: RuntimeContext,
    ) -> RuntimeFuture<Self::Output>;
}

/// Sub-agent boundary for delegated graph-loop work.
pub trait SubAgentRuntime: Send + Sync + 'static {
    type Input: Send + 'static;
    type Output: Send + 'static;

    fn run_sub_agent(
        &self,
        input: Self::Input,
        context: RuntimeContext,
    ) -> RuntimeFuture<Self::Output>;
}
