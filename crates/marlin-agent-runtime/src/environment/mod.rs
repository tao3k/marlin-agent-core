// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Runtime environment resolution for custom homes, config layers, and sub-agents.

#![forbid(unsafe_code)]

mod activation;
mod import;
mod refresh;
mod resolver;
mod state_storage;

pub use activation::{
    DirenvCommandRunner, ProcessDirenvCommandRunner, RuntimeEnvironmentActivationError,
    RuntimeEnvironmentActivationRequest, RuntimeEnvironmentActivationResult,
    RuntimeEnvironmentActivator,
};
pub use import::{
    RuntimeWorkspaceProjectImportRequest, RuntimeWorkspaceProjectImportResult,
    RuntimeWorkspaceProjectImporter,
};
pub use refresh::{
    RuntimeEnvironmentRefreshRequest, RuntimeEnvironmentRefreshResult, RuntimeEnvironmentRefresher,
};
pub use resolver::{
    HOST_HOME_ENV_VAR, PROJECT_CONFIG_PRECEDENCE, RuntimeEnvironmentError,
    RuntimeEnvironmentRequest, RuntimeEnvironmentResolver, SESSION_FLAGS_CONFIG_PRECEDENCE,
    SUB_AGENT_CONFIG_PRECEDENCE, SYSTEM_CONFIG_PRECEDENCE, SubAgentEnvironmentRequest,
    USER_CONFIG_PRECEDENCE,
};
pub use state_storage::RuntimeStateStorageInitializer;
