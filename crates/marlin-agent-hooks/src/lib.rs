//! Hook registry and dispatch orchestration for `marlin-agent` runtimes.

#![forbid(unsafe_code)]

mod config;
mod dispatcher;
mod dynamic_actions;

pub use config::{
    HookConfigurationEnvelope, HookConfigurationError, HookPolicyConfiguration,
    HookRegistrationDefaults,
};
pub use dispatcher::{
    HookDispatchPolicy, HookDispatchPolicyFinalizer, HookDispatchPolicyFinalizerInput,
    HookDispatchReport, HookDispatcher, HookInvocation, HookRegistration, HookRegistry,
    RegisteredHookPolicyFinalizer, RegisteredHookRegistrationCatalog, RegisteredHookRuntime,
};
pub use dynamic_actions::HookRegistrationCatalog;
