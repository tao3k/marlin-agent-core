//! Typed Rust projection for config-interface loop case driver receipts.

mod ids;
mod intent_case_artifact;
mod loop_program_projection;
mod projected_case;
mod real_llm_case;
mod receipt;
mod vertical_trace;

pub use ids::{
    GerbilLoopCaseDriverCapability, GerbilLoopCaseDriverCaseId, GerbilLoopCaseDriverLoopProgramId,
    GerbilLoopCaseDriverProfileRef,
};
pub use intent_case_artifact::{
    GERBIL_LOOP_CASE_DRIVER_INTENT_CASE_RUNTIME_OWNER,
    project_gerbil_loop_case_driver_intent_case_artifact_manifest,
    project_gerbil_loop_case_driver_intent_case_run_receipt,
};
pub use loop_program_projection::{
    GerbilLoopCaseDriverLoopProgramProjectionError,
    project_gerbil_loop_case_driver_loop_action_kind,
    project_gerbil_loop_case_driver_loop_event_kind, project_gerbil_loop_case_driver_loop_program,
};
pub use projected_case::{
    GerbilLoopCaseDriverProjectedLoopProgram, GerbilLoopCaseDriverProjectedLoopProgramError,
    GerbilLoopCaseDriverProjectedLoopProgramRequest, default_gerbil_config_interface_root,
    gerbil_config_interface_loadpath_with_src, load_gerbil_loop_case_driver_projected_loop_program,
    run_gerbil_config_interface_case_driver_smoke,
    run_gerbil_config_interface_case_driver_smoke_in,
};
pub use real_llm_case::{
    GerbilLoopCaseDriverRealLlmCaseReceipt, GerbilLoopCaseDriverRealLlmCaseReceiptError,
    parse_gerbil_loop_case_driver_real_llm_case_receipt,
};
pub use receipt::{
    GERBIL_LOOP_CASE_DRIVER_RUST_LOOP_RECEIPT_SCHEMA_ID,
    GERBIL_LOOP_CASE_DRIVER_SCHEME_RECEIPT_KIND, GerbilLoopCaseCommandKind,
    GerbilLoopCaseDriverRustLoopReceipt, GerbilLoopCaseDriverSchemeReceipt,
    GerbilLoopCaseDriverSchemeReceiptKind, GerbilLoopCaseRuntimeHandoffStatus,
    GerbilLoopCaseRuntimeMode, GerbilLoopCaseSchemeBoundary, GerbilLoopCaseSerializationBoundary,
    GerbilLoopCaseSmokeStatus, project_gerbil_loop_case_driver_rust_loop_receipt,
    project_gerbil_loop_case_driver_vertical_trace_rust_loop_receipt,
};
pub use vertical_trace::{
    GerbilLoopCaseDriverVerticalTraceError, GerbilLoopCaseDriverVerticalTraceReceipt,
    parse_gerbil_loop_case_driver_vertical_trace, verify_gerbil_loop_case_driver_vertical_trace,
};
