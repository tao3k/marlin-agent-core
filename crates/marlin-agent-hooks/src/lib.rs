// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Hook registry and dispatch orchestration for `marlin-agent` runtimes.

#![forbid(unsafe_code)]

mod config;
mod dispatcher;
mod dynamic_actions;
mod path_filter;

pub use config::{
    HookConfigurationEnvelope, HookConfigurationError, HookPolicyConfiguration,
    HookRegistrationDefaults,
};
pub use dispatcher::{
    HookActionContext, HookActionProvider, HookDispatchPolicy, HookDispatchReport, HookDispatcher,
    HookInvocation, HookRegistration, HookRegistry, RegisteredHookActionProvider,
    RegisteredHookRegistrationCatalog, RegisteredHookRuntime,
};
pub use dynamic_actions::HookRegistrationCatalog;
pub use path_filter::{HookPathFilter, HookPathFilterError, HookPathPattern, HookTargetPath};
