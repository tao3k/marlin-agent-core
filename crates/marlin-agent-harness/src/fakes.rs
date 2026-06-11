//! Static fake provider, tool, and sub-agent runtimes for harness scenarios.

use std::marker::PhantomData;

use marlin_agent_runtime::{
    HookRuntime, ProviderRuntime, RuntimeContext, RuntimeFuture, SubAgentRuntime, ToolRuntime,
};

/// Provider runtime that always returns the same response.
#[derive(Clone, Debug)]
pub struct StaticProviderRuntime<Request, Response> {
    response: Response,
    _request: PhantomData<fn(Request)>,
}

impl<Request, Response> StaticProviderRuntime<Request, Response> {
    pub fn new(response: Response) -> Self {
        Self {
            response,
            _request: PhantomData,
        }
    }
}

impl<Request, Response> ProviderRuntime for StaticProviderRuntime<Request, Response>
where
    Request: Send + 'static,
    Response: Clone + Send + Sync + 'static,
{
    type Request = Request;
    type Response = Response;

    fn run_provider(
        &self,
        _request: Self::Request,
        _context: RuntimeContext,
    ) -> RuntimeFuture<Self::Response> {
        let response = self.response.clone();
        Box::pin(async move { response })
    }
}

/// Tool runtime that always returns the same output.
#[derive(Clone, Debug)]
pub struct StaticToolRuntime<Invocation, Output> {
    output: Output,
    _invocation: PhantomData<fn(Invocation)>,
}

impl<Invocation, Output> StaticToolRuntime<Invocation, Output> {
    pub fn new(output: Output) -> Self {
        Self {
            output,
            _invocation: PhantomData,
        }
    }
}

impl<Invocation, Output> ToolRuntime for StaticToolRuntime<Invocation, Output>
where
    Invocation: Send + 'static,
    Output: Clone + Send + Sync + 'static,
{
    type Invocation = Invocation;
    type Output = Output;

    fn run_tool(
        &self,
        _invocation: Self::Invocation,
        _context: RuntimeContext,
    ) -> RuntimeFuture<Self::Output> {
        let output = self.output.clone();
        Box::pin(async move { output })
    }
}

/// Hook runtime that always returns the same output.
#[derive(Clone, Debug)]
pub struct StaticHookRuntime<Request, Output> {
    output: Output,
    _request: PhantomData<fn(Request)>,
}

impl<Request, Output> StaticHookRuntime<Request, Output> {
    pub fn new(output: Output) -> Self {
        Self {
            output,
            _request: PhantomData,
        }
    }
}

impl<Request, Output> HookRuntime for StaticHookRuntime<Request, Output>
where
    Request: Send + 'static,
    Output: Clone + Send + Sync + 'static,
{
    type Request = Request;
    type Output = Output;

    fn run_hook(
        &self,
        _request: Self::Request,
        _context: RuntimeContext,
    ) -> RuntimeFuture<Self::Output> {
        let output = self.output.clone();
        Box::pin(async move { output })
    }
}

/// Sub-agent runtime that always returns the same output.
#[derive(Clone, Debug)]
pub struct StaticSubAgentRuntime<Input, Output> {
    output: Output,
    _input: PhantomData<fn(Input)>,
}

impl<Input, Output> StaticSubAgentRuntime<Input, Output> {
    pub fn new(output: Output) -> Self {
        Self {
            output,
            _input: PhantomData,
        }
    }
}

impl<Input, Output> SubAgentRuntime for StaticSubAgentRuntime<Input, Output>
where
    Input: Send + 'static,
    Output: Clone + Send + Sync + 'static,
{
    type Input = Input;
    type Output = Output;

    fn run_sub_agent(
        &self,
        _input: Self::Input,
        _context: RuntimeContext,
    ) -> RuntimeFuture<Self::Output> {
        let output = self.output.clone();
        Box::pin(async move { output })
    }
}
