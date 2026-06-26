//! Typed execution ABI for resident `Gerbil Scheme` strategy requests.

use serde::{Deserialize, Serialize};

use crate::scheme_types::GerbilSchemeValue;

use super::{
    GerbilResidentRuntimeHealthStatus, GerbilResidentRuntimeSessionId,
    GerbilResidentStrategyEventKind, GerbilResidentStrategyRequest,
    GerbilResidentStrategyRequestId, GerbilResidentStrategyRequestReceipt,
};

/// Typed execution request sent behind an accepted resident strategy lane.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GerbilResidentStrategyExecutionRequest {
    pub strategy_request: GerbilResidentStrategyRequest,
    pub payload: GerbilSchemeValue,
}

impl GerbilResidentStrategyExecutionRequest {
    pub fn new(
        request_id: impl Into<GerbilResidentStrategyRequestId>,
        event_kind: GerbilResidentStrategyEventKind,
        payload: GerbilSchemeValue,
    ) -> Self {
        Self {
            strategy_request: GerbilResidentStrategyRequest::new(request_id, event_kind),
            payload,
        }
    }

    pub fn with_lane_id(mut self, lane_id: impl Into<String>) -> Self {
        self.strategy_request = self.strategy_request.with_lane_id(lane_id);
        self
    }

    pub fn with_session_id(
        mut self,
        session_id: impl Into<GerbilResidentRuntimeSessionId>,
    ) -> Self {
        self.strategy_request = self.strategy_request.with_session_id(session_id);
        self
    }

    pub fn with_policy_epoch(mut self, policy_epoch: u64) -> Self {
        self.strategy_request = self.strategy_request.with_policy_epoch(policy_epoch);
        self
    }
}

/// Execution status returned by a typed resident strategy executor.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GerbilResidentStrategyExecutionStatus {
    Executed,
    Denied,
    Deferred,
    RejectedByContract,
    RuntimeError,
    AdmissionRejected,
}

/// Typed response produced by a resident Gerbil strategy executor.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GerbilResidentStrategyExecutionResponse {
    pub status: GerbilResidentStrategyExecutionStatus,
    pub payload: Option<GerbilSchemeValue>,
    pub reason: Option<String>,
    pub derived_session_id: Option<GerbilResidentRuntimeSessionId>,
}

impl GerbilResidentStrategyExecutionResponse {
    pub fn executed(payload: GerbilSchemeValue) -> Self {
        Self {
            status: GerbilResidentStrategyExecutionStatus::Executed,
            payload: Some(payload),
            reason: None,
            derived_session_id: None,
        }
    }

    pub fn denied(reason: impl Into<String>) -> Self {
        Self::without_payload(GerbilResidentStrategyExecutionStatus::Denied, reason)
    }

    pub fn deferred(reason: impl Into<String>) -> Self {
        Self::without_payload(GerbilResidentStrategyExecutionStatus::Deferred, reason)
    }

    pub fn rejected_by_contract(reason: impl Into<String>) -> Self {
        Self::without_payload(
            GerbilResidentStrategyExecutionStatus::RejectedByContract,
            reason,
        )
    }

    pub fn runtime_error(reason: impl Into<String>) -> Self {
        Self::without_payload(GerbilResidentStrategyExecutionStatus::RuntimeError, reason)
    }

    pub fn with_derived_session_id(
        mut self,
        session_id: impl Into<GerbilResidentRuntimeSessionId>,
    ) -> Self {
        self.derived_session_id = Some(session_id.into());
        self
    }

    fn without_payload(
        status: GerbilResidentStrategyExecutionStatus,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            status,
            payload: None,
            reason: Some(reason.into()),
            derived_session_id: None,
        }
    }
}

/// Performance scope measured by a resident strategy execution receipt.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GerbilResidentStrategyExecutionPerformanceScope {
    AdmissionAndExecution,
}

/// Rust-owned performance receipt for a resident strategy execution.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilResidentStrategyExecutionPerformanceReceipt {
    pub scope: GerbilResidentStrategyExecutionPerformanceScope,
    pub executor_invoked: bool,
    pub elapsed_micros: u64,
    pub process_reuse_required: bool,
    pub process_reuse_observed: bool,
    pub child_id: Option<u32>,
}

impl GerbilResidentStrategyExecutionPerformanceReceipt {
    fn new(
        request_receipt: &GerbilResidentStrategyRequestReceipt,
        executor_invoked: bool,
        elapsed_micros: u64,
    ) -> Self {
        Self {
            scope: GerbilResidentStrategyExecutionPerformanceScope::AdmissionAndExecution,
            executor_invoked,
            elapsed_micros,
            process_reuse_required: request_receipt.process_reuse_required,
            process_reuse_observed: request_receipt.process_reuse_required
                && request_receipt.child_id.is_some()
                && request_receipt.process_health
                    == Some(GerbilResidentRuntimeHealthStatus::Running),
            child_id: request_receipt.child_id,
        }
    }
}

/// Receipt proving a typed resident strategy request crossed admission and execution.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GerbilResidentStrategyExecutionReceipt {
    pub request_receipt: GerbilResidentStrategyRequestReceipt,
    pub request_payload: GerbilSchemeValue,
    pub status: GerbilResidentStrategyExecutionStatus,
    pub response_payload: Option<GerbilSchemeValue>,
    pub reason: Option<String>,
    pub derived_session_id: Option<GerbilResidentRuntimeSessionId>,
    pub performance: GerbilResidentStrategyExecutionPerformanceReceipt,
}

impl GerbilResidentStrategyExecutionReceipt {
    pub fn admission_rejected(
        request_receipt: GerbilResidentStrategyRequestReceipt,
        request_payload: GerbilSchemeValue,
        elapsed_micros: u64,
    ) -> Self {
        let performance = GerbilResidentStrategyExecutionPerformanceReceipt::new(
            &request_receipt,
            false,
            elapsed_micros,
        );
        Self {
            request_receipt,
            request_payload,
            status: GerbilResidentStrategyExecutionStatus::AdmissionRejected,
            response_payload: None,
            reason: None,
            derived_session_id: None,
            performance,
        }
    }

    pub fn from_response(
        request_receipt: GerbilResidentStrategyRequestReceipt,
        request_payload: GerbilSchemeValue,
        response: GerbilResidentStrategyExecutionResponse,
        elapsed_micros: u64,
    ) -> Self {
        let performance = GerbilResidentStrategyExecutionPerformanceReceipt::new(
            &request_receipt,
            true,
            elapsed_micros,
        );
        Self {
            request_receipt,
            request_payload,
            status: response.status,
            response_payload: response.payload,
            reason: response.reason,
            derived_session_id: response.derived_session_id,
            performance,
        }
    }
}
