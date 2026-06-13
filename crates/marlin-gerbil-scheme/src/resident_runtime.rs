//! Typed plan for resident `Gerbil Scheme` runtime sessions.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{GerbilCommandProfile, runtime::default_gerbil_gxi_program};

/// Session strategy for a resident Gerbil runtime.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum GerbilResidentRuntimeSessionMode {
    #[default]
    Disabled,
    SharedContext,
    ForkedContext,
    IsolatedSession,
}

/// Stable session label for a resident Gerbil runtime.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct GerbilResidentRuntimeSessionId(String);

impl GerbilResidentRuntimeSessionId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl From<String> for GerbilResidentRuntimeSessionId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for GerbilResidentRuntimeSessionId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// Rust-owned plan for starting or reusing a resident Gerbil runtime.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilResidentRuntimePlan {
    pub session_mode: GerbilResidentRuntimeSessionMode,
    pub session_id: Option<GerbilResidentRuntimeSessionId>,
    pub command_profile: GerbilCommandProfile,
    pub loadpath_root: PathBuf,
}

impl GerbilResidentRuntimePlan {
    pub fn disabled(loadpath_root: impl Into<PathBuf>) -> Self {
        Self {
            session_mode: GerbilResidentRuntimeSessionMode::Disabled,
            session_id: None,
            command_profile: default_command_profile(),
            loadpath_root: loadpath_root.into(),
        }
    }

    pub fn shared_context(
        loadpath_root: impl Into<PathBuf>,
        session_id: impl Into<GerbilResidentRuntimeSessionId>,
    ) -> Self {
        Self::new(
            GerbilResidentRuntimeSessionMode::SharedContext,
            loadpath_root,
            session_id,
        )
    }

    pub fn forked_context(
        loadpath_root: impl Into<PathBuf>,
        session_id: impl Into<GerbilResidentRuntimeSessionId>,
    ) -> Self {
        Self::new(
            GerbilResidentRuntimeSessionMode::ForkedContext,
            loadpath_root,
            session_id,
        )
    }

    pub fn isolated_session(
        loadpath_root: impl Into<PathBuf>,
        session_id: impl Into<GerbilResidentRuntimeSessionId>,
    ) -> Self {
        Self::new(
            GerbilResidentRuntimeSessionMode::IsolatedSession,
            loadpath_root,
            session_id,
        )
    }

    pub fn with_command_profile(mut self, command_profile: GerbilCommandProfile) -> Self {
        self.command_profile = command_profile;
        self
    }

    pub fn requires_process_reuse(&self) -> bool {
        self.session_mode != GerbilResidentRuntimeSessionMode::Disabled
    }

    pub fn isolates_state(&self) -> bool {
        self.session_mode == GerbilResidentRuntimeSessionMode::IsolatedSession
    }

    fn new(
        session_mode: GerbilResidentRuntimeSessionMode,
        loadpath_root: impl Into<PathBuf>,
        session_id: impl Into<GerbilResidentRuntimeSessionId>,
    ) -> Self {
        Self {
            session_mode,
            session_id: Some(session_id.into()),
            command_profile: default_command_profile(),
            loadpath_root: loadpath_root.into(),
        }
    }
}

fn default_command_profile() -> GerbilCommandProfile {
    GerbilCommandProfile::new(default_gerbil_gxi_program().to_string_lossy().into_owned())
}
