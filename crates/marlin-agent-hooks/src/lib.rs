//! Hook registry and dispatch orchestration for `marlin-agent` runtimes.

#![forbid(unsafe_code)]

mod dispatcher;

pub use dispatcher::{
    HookDispatchReport, HookDispatcher, HookInvocation, HookRegistration, HookRegistry,
    RegisteredHookRuntime,
};
