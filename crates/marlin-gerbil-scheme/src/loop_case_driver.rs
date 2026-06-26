//! Typed Rust projection for config-interface loop case driver receipts.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Scheme receipt kind emitted by the config-interface loop case driver.
pub const GERBIL_LOOP_CASE_DRIVER_SCHEME_RECEIPT_KIND: &str =
    "marline-kernel.loop-case-driver.receipt.v1";
/// Rust loop receipt schema projected from a Scheme loop policy case.
pub const GERBIL_LOOP_CASE_DRIVER_RUST_LOOP_RECEIPT_SCHEMA_ID: &str =
    "marlin-gerbil-scheme.config-interface.loop-case-rust-loop-receipt.v1";

/// Command lane selected by the Scheme case driver.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GerbilLoopCaseCommandKind {
    LoopProgramRun,
    LoopRun,
}

/// Typed receipt kind emitted by the config-interface loop case driver.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GerbilLoopCaseDriverSchemeReceiptKind {
    #[serde(rename = "marline-kernel.loop-case-driver.receipt.v1")]
    MarlineKernelLoopCaseDriverReceipt,
}

impl GerbilLoopCaseDriverSchemeReceiptKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MarlineKernelLoopCaseDriverReceipt => GERBIL_LOOP_CASE_DRIVER_SCHEME_RECEIPT_KIND,
        }
    }
}

/// Runtime mode selected by the Scheme config-interface case driver.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GerbilLoopCaseRuntimeMode {
    LoopRuntime,
}

impl GerbilLoopCaseRuntimeMode {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LoopRuntime => "loop-runtime",
        }
    }
}

/// Smoke status reported by the Scheme case driver.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GerbilLoopCaseSmokeStatus {
    LiveEnabled,
    NoLiveLlmDenied,
}

/// Runtime handoff status after Rust receives the typed Scheme case.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GerbilLoopCaseRuntimeHandoffStatus {
    Ready,
    DeferredNoLiveLlm,
}

/// Internal boundary for Scheme-to-Rust case projection.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GerbilLoopCaseSchemeBoundary {
    SchemeTypesToRustTypes,
}

/// External serialization boundary owned by Rust.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GerbilLoopCaseSerializationBoundary {
    RustOwnedCliTraceCrossProcess,
}

/// Typed Scheme case-driver receipt consumed by Rust.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilLoopCaseDriverSchemeReceipt {
    pub kind: GerbilLoopCaseDriverSchemeReceiptKind,
    pub case_id: String,
    pub profile_ref: String,
    pub runtime_mode: GerbilLoopCaseRuntimeMode,
    pub live_gate_env: String,
    pub live_enabled: bool,
    pub smoke_status: GerbilLoopCaseSmokeStatus,
    pub command_kind: GerbilLoopCaseCommandKind,
    pub command_vector: Vec<String>,
    pub input_path: PathBuf,
    pub stable_fixture: bool,
    pub result_protocol: String,
    pub policy_owner: String,
    pub control_plane_owner: String,
    pub runtime_execution_owner: String,
    pub scheme_boundary: GerbilLoopCaseSchemeBoundary,
    pub serialization_boundary: GerbilLoopCaseSerializationBoundary,
}

/// Rust loop receipt derived from one typed Scheme policy case.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilLoopCaseDriverRustLoopReceipt {
    pub schema_id: String,
    pub case_id: String,
    pub profile_ref: String,
    pub command_kind: GerbilLoopCaseCommandKind,
    pub command_vector: Vec<String>,
    pub input_path: PathBuf,
    pub runtime_handoff_status: GerbilLoopCaseRuntimeHandoffStatus,
    pub runtime_execution_owner: String,
    pub live_llm_required: bool,
    pub live_llm_allowed: bool,
    pub stable_fixture: bool,
    pub scheme_boundary: GerbilLoopCaseSchemeBoundary,
    pub serialization_boundary: GerbilLoopCaseSerializationBoundary,
}

impl GerbilLoopCaseDriverSchemeReceipt {
    pub fn runtime_handoff_no_live_llm_fixture(
        case_id: impl Into<String>,
        input_path: impl Into<PathBuf>,
    ) -> Self {
        Self {
            kind: GerbilLoopCaseDriverSchemeReceiptKind::MarlineKernelLoopCaseDriverReceipt,
            case_id: case_id.into(),
            profile_ref: "custom/marline-kernel/profiles/default-loop-runtime.ss".to_owned(),
            runtime_mode: GerbilLoopCaseRuntimeMode::LoopRuntime,
            live_gate_env: "MARLIN_LIVE_LLM".to_owned(),
            live_enabled: false,
            smoke_status: GerbilLoopCaseSmokeStatus::NoLiveLlmDenied,
            command_kind: GerbilLoopCaseCommandKind::LoopProgramRun,
            command_vector: vec!["loop-program".to_owned(), "run".to_owned()],
            input_path: input_path.into(),
            stable_fixture: true,
            result_protocol: "rust-loop-receipt".to_owned(),
            policy_owner: "gerbil-config-interface".to_owned(),
            control_plane_owner: "gerbil-config-interface".to_owned(),
            runtime_execution_owner: "rust-loop-runtime".to_owned(),
            scheme_boundary: GerbilLoopCaseSchemeBoundary::SchemeTypesToRustTypes,
            serialization_boundary:
                GerbilLoopCaseSerializationBoundary::RustOwnedCliTraceCrossProcess,
        }
    }
}

/// Project a typed Scheme policy case into a Rust loop runtime receipt.
pub fn project_gerbil_loop_case_driver_rust_loop_receipt(
    receipt: &GerbilLoopCaseDriverSchemeReceipt,
) -> GerbilLoopCaseDriverRustLoopReceipt {
    let live_llm_required = matches!(receipt.smoke_status, GerbilLoopCaseSmokeStatus::LiveEnabled);
    let runtime_handoff_status = if live_llm_required || receipt.live_enabled {
        GerbilLoopCaseRuntimeHandoffStatus::Ready
    } else {
        GerbilLoopCaseRuntimeHandoffStatus::DeferredNoLiveLlm
    };

    GerbilLoopCaseDriverRustLoopReceipt {
        schema_id: GERBIL_LOOP_CASE_DRIVER_RUST_LOOP_RECEIPT_SCHEMA_ID.to_owned(),
        case_id: receipt.case_id.clone(),
        profile_ref: receipt.profile_ref.clone(),
        command_kind: receipt.command_kind,
        command_vector: receipt.command_vector.clone(),
        input_path: receipt.input_path.clone(),
        runtime_handoff_status,
        runtime_execution_owner: receipt.runtime_execution_owner.clone(),
        live_llm_required,
        live_llm_allowed: receipt.live_enabled,
        stable_fixture: receipt.stable_fixture,
        scheme_boundary: receipt.scheme_boundary,
        serialization_boundary: receipt.serialization_boundary,
    }
}
