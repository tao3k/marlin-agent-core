//! Runtime state-home helpers shared by debug CLI commands.

use std::{env, path::PathBuf};

use marlin_agent_environment::{RuntimeEnvironmentRequest, RuntimeEnvironmentResolver};
use marlin_agent_protocol::RuntimeStateLayout;

pub(super) fn resolve_runtime_state_layout(
    home: Option<PathBuf>,
) -> Result<RuntimeStateLayout, String> {
    let request = match home {
        Some(home) => RuntimeEnvironmentRequest::default().with_custom_home(home),
        None => RuntimeEnvironmentRequest::default().with_home_from_host_env(env::vars()),
    };
    let resolution = RuntimeEnvironmentResolver::new().resolve_with_receipt(request);
    resolution
        .environment
        .state_layout
        .ok_or_else(|| "runtime state home is not configured; set MARLIN_HOME or HOME".to_owned())
}
