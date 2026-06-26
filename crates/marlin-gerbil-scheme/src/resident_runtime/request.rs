//! Strategy lane and request types for resident `Gerbil Scheme` services.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::GerbilCommandProfile;

use super::{
    GerbilResidentRuntimeHealthStatus, GerbilResidentRuntimeSessionId,
    GerbilResidentRuntimeSessionMode,
};

/// Strategy event handled by a dedicated resident Gerbil lane.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GerbilResidentStrategyEventKind {
    DynamicReplan,
    PolicyChange,
}

impl GerbilResidentStrategyEventKind {
    pub fn lane_id(&self) -> GerbilResidentStrategyLaneId {
        match self {
            Self::DynamicReplan => GerbilResidentStrategyLaneId::new("dynamic-replan"),
            Self::PolicyChange => GerbilResidentStrategyLaneId::new("policy-change"),
        }
    }
}

/// Stable lane label for a resident Gerbil strategy service.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct GerbilResidentStrategyLaneId(String);

impl GerbilResidentStrategyLaneId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// Lifecycle status for a resident Gerbil strategy lane.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GerbilResidentStrategyLaneStatus {
    Disabled,
    ReadyToServe,
}

/// Rust-owned lane plan for a resident Gerbil strategy service.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilResidentStrategyLanePlan {
    pub lane_id: GerbilResidentStrategyLaneId,
    pub event_kind: GerbilResidentStrategyEventKind,
    pub status: GerbilResidentStrategyLaneStatus,
    pub session_mode: GerbilResidentRuntimeSessionMode,
    pub session_id: Option<GerbilResidentRuntimeSessionId>,
    pub process_reuse_required: bool,
    pub state_isolated: bool,
    pub command_profile: Option<GerbilCommandProfile>,
    pub loadpath_root: PathBuf,
}

/// Rust-owned plan for dedicated resident Gerbil strategy lanes.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilResidentStrategyServicePlan {
    pub session_mode: GerbilResidentRuntimeSessionMode,
    pub session_id: Option<GerbilResidentRuntimeSessionId>,
    pub process_reuse_required: bool,
    pub state_isolated: bool,
    pub loadpath_root: PathBuf,
    pub lanes: Vec<GerbilResidentStrategyLanePlan>,
}

/// Receipt proving the resident runtime exposes strategy lanes before execution.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilResidentStrategyServiceReceipt {
    pub session_mode: GerbilResidentRuntimeSessionMode,
    pub session_id: Option<GerbilResidentRuntimeSessionId>,
    pub process_reuse_required: bool,
    pub state_isolated: bool,
    pub loadpath_root: PathBuf,
    pub lane_count: usize,
    pub ready_lane_count: usize,
    pub disabled_lane_count: usize,
    pub dynamic_replan_lane_count: usize,
    pub policy_change_lane_count: usize,
    pub lanes: Vec<GerbilResidentStrategyLanePlan>,
}

/// Stable request label for a resident Gerbil strategy lane.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct GerbilResidentStrategyRequestId(String);

impl GerbilResidentStrategyRequestId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl From<String> for GerbilResidentStrategyRequestId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for GerbilResidentStrategyRequestId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// Rust-owned request for a resident Gerbil strategy lane.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilResidentStrategyRequest {
    pub request_id: GerbilResidentStrategyRequestId,
    pub lane_id: GerbilResidentStrategyLaneId,
    pub event_kind: GerbilResidentStrategyEventKind,
    pub session_id: Option<GerbilResidentRuntimeSessionId>,
    pub policy_epoch: Option<u64>,
}

impl GerbilResidentStrategyRequest {
    pub fn new(
        request_id: impl Into<GerbilResidentStrategyRequestId>,
        event_kind: GerbilResidentStrategyEventKind,
    ) -> Self {
        let lane_id = event_kind.lane_id();
        Self {
            request_id: request_id.into(),
            lane_id,
            event_kind,
            session_id: None,
            policy_epoch: None,
        }
    }

    pub fn with_lane_id(mut self, lane_id: impl Into<String>) -> Self {
        self.lane_id = GerbilResidentStrategyLaneId::new(lane_id);
        self
    }

    pub fn with_session_id(
        mut self,
        session_id: impl Into<GerbilResidentRuntimeSessionId>,
    ) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn with_policy_epoch(mut self, policy_epoch: u64) -> Self {
        self.policy_epoch = Some(policy_epoch);
        self
    }
}

/// Request admission status for a resident Gerbil strategy lane.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GerbilResidentStrategyRequestStatus {
    Accepted,
    RuntimeDisabled,
    LaneUnavailable,
    EventLaneMismatch,
    ProcessNotRunning,
}

/// Receipt emitted after routing a typed strategy request to a resident lane.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilResidentStrategyRequestReceipt {
    pub request_id: GerbilResidentStrategyRequestId,
    pub lane_id: GerbilResidentStrategyLaneId,
    pub event_kind: GerbilResidentStrategyEventKind,
    pub status: GerbilResidentStrategyRequestStatus,
    pub session_mode: GerbilResidentRuntimeSessionMode,
    pub session_id: Option<GerbilResidentRuntimeSessionId>,
    pub process_reuse_required: bool,
    pub state_isolated: bool,
    pub policy_epoch: Option<u64>,
    pub child_id: Option<u32>,
    pub process_health: Option<GerbilResidentRuntimeHealthStatus>,
}
