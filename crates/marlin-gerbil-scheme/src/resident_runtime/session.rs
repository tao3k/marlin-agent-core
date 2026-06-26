//! Session plan types for resident `Gerbil Scheme` runtime ownership.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::GerbilCommandProfile;

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

/// Receipt emitted after preparing the resident runtime loadpath.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilResidentRuntimePrepareReceipt {
    pub session_mode: GerbilResidentRuntimeSessionMode,
    pub session_id: Option<GerbilResidentRuntimeSessionId>,
    pub process_reuse_required: bool,
    pub state_isolated: bool,
    pub command_profile: GerbilCommandProfile,
    pub loadpath_root: PathBuf,
    pub written_asset_count: usize,
}
