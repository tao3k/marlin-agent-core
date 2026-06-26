//! Typed plan, process, and receipt surfaces for resident `Gerbil Scheme` runtime sessions.

mod bridge;
mod execution;
mod handle;
mod plan;
mod process;
mod request;
mod session;
mod strategy;

pub use bridge::{GerbilResidentStrategyExecutor, GerbilResidentStrategyGxiSmokeBridge};
pub use execution::{
    GerbilResidentStrategyExecutionPerformanceReceipt,
    GerbilResidentStrategyExecutionPerformanceScope, GerbilResidentStrategyExecutionReceipt,
    GerbilResidentStrategyExecutionRequest, GerbilResidentStrategyExecutionResponse,
    GerbilResidentStrategyExecutionStatus,
};
pub use handle::GerbilResidentRuntimeHandle;
pub use process::{
    GerbilResidentRuntimeHealthReceipt, GerbilResidentRuntimeHealthStatus,
    GerbilResidentRuntimeProcess, GerbilResidentRuntimeProcessPlan,
    GerbilResidentRuntimeProcessReceipt, GerbilResidentRuntimeProcessStatus,
    GerbilResidentRuntimeShutdownReceipt, GerbilResidentRuntimeShutdownStatus,
};
pub use request::{
    GerbilResidentStrategyEventKind, GerbilResidentStrategyLaneId, GerbilResidentStrategyLanePlan,
    GerbilResidentStrategyLaneStatus, GerbilResidentStrategyRequest,
    GerbilResidentStrategyRequestId, GerbilResidentStrategyRequestReceipt,
    GerbilResidentStrategyRequestStatus, GerbilResidentStrategyServicePlan,
    GerbilResidentStrategyServiceReceipt,
};
pub use session::{
    GerbilResidentRuntimePlan, GerbilResidentRuntimePrepareReceipt, GerbilResidentRuntimeSessionId,
    GerbilResidentRuntimeSessionMode,
};
