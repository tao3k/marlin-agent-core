//! Runtime environment resolution for custom homes, config layers, and sub-agents.

#![forbid(unsafe_code)]

mod activation;
mod resolver;

pub use activation::{
    DirenvCommandRunner, ProcessDirenvCommandRunner, RuntimeEnvironmentActivationError,
    RuntimeEnvironmentActivationRequest, RuntimeEnvironmentActivationResult,
    RuntimeEnvironmentActivator,
};
pub use resolver::{
    PROJECT_CONFIG_PRECEDENCE, RuntimeEnvironmentError, RuntimeEnvironmentRequest,
    RuntimeEnvironmentResolver, SESSION_FLAGS_CONFIG_PRECEDENCE, SUB_AGENT_CONFIG_PRECEDENCE,
    SYSTEM_CONFIG_PRECEDENCE, SubAgentEnvironmentRequest, USER_CONFIG_PRECEDENCE,
};
