//! Hook registry and dispatch orchestration for `marlin-agent` runtimes.

#![forbid(unsafe_code)]

mod config;
mod dispatcher;

pub use config::{
    HookConfigurationEnvelope, HookConfigurationError, HookPolicyConfiguration,
    HookRegistrationDefaults,
};
pub use dispatcher::{
    HookDispatchPolicy, HookDispatchPolicyFinalizer, HookDispatchPolicyFinalizerInput,
    HookDispatchReport, HookDispatcher, HookInvocation, HookRegistration, HookRegistry,
    RegisteredHookPolicyFinalizer, RegisteredHookRuntime,
};
