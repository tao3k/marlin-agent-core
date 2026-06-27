//! Typed receipt projection for config-interface loop case driver receipts.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::{GerbilLoopCaseDriverCaseId, GerbilLoopCaseDriverProfileRef};

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
    RealLlmOptIn,
    TypedLoopProjection,
}

impl GerbilLoopCaseRuntimeMode {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LoopRuntime => "loop-runtime",
            Self::RealLlmOptIn => "real-llm-opt-in",
            Self::TypedLoopProjection => "typed-loop-projection",
        }
    }
}

/// Smoke status reported by the Scheme case driver.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GerbilLoopCaseSmokeStatus {
    LiveEnabled,
    NoLiveLlmDenied,
    TypedLoopProjectionReady,
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

impl GerbilLoopCaseSchemeBoundary {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SchemeTypesToRustTypes => "scheme-types->rust-types",
        }
    }
}

/// External serialization boundary owned by Rust.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GerbilLoopCaseSerializationBoundary {
    RustOwnedCliTraceCrossProcess,
}

impl GerbilLoopCaseSerializationBoundary {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RustOwnedCliTraceCrossProcess => "rust-owned-cli-trace-cross-process",
        }
    }
}

/// POO Flow module projection kind carried by a Scheme case-driver receipt.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GerbilLoopCaseModuleKind(String);

impl GerbilLoopCaseModuleKind {
    #[must_use]
    pub fn new(kind: impl Into<String>) -> Self {
        Self(kind.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// POO Flow user module name that owns the projected case semantics.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GerbilLoopCaseModuleName(String);

impl GerbilLoopCaseModuleName {
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// POO Flow module selection capability tag projected into the Rust receipt.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GerbilLoopCaseModuleSelectionTag(String);

impl GerbilLoopCaseModuleSelectionTag {
    #[must_use]
    pub fn new(tag: impl Into<String>) -> Self {
        Self(tag.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Typed Scheme case-driver receipt consumed by Rust.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilLoopCaseDriverSchemeReceipt {
    pub kind: GerbilLoopCaseDriverSchemeReceiptKind,
    pub case_id: GerbilLoopCaseDriverCaseId,
    pub profile_ref: GerbilLoopCaseDriverProfileRef,
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
    pub module_kind: GerbilLoopCaseModuleKind,
    pub module_user_module: GerbilLoopCaseModuleName,
    pub module_selection_tags: Vec<GerbilLoopCaseModuleSelectionTag>,
    pub module_source_ref: String,
    pub module_entrypoint: String,
    pub module_enabled: bool,
    pub scheme_boundary: GerbilLoopCaseSchemeBoundary,
    pub serialization_boundary: GerbilLoopCaseSerializationBoundary,
}

/// Rust loop receipt derived from one typed Scheme policy case.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilLoopCaseDriverRustLoopReceipt {
    pub schema_id: String,
    pub case_id: GerbilLoopCaseDriverCaseId,
    pub profile_ref: GerbilLoopCaseDriverProfileRef,
    pub command_kind: GerbilLoopCaseCommandKind,
    pub command_vector: Vec<String>,
    pub input_path: PathBuf,
    pub runtime_handoff_status: GerbilLoopCaseRuntimeHandoffStatus,
    pub runtime_execution_owner: String,
    pub module_kind: GerbilLoopCaseModuleKind,
    pub module_user_module: GerbilLoopCaseModuleName,
    pub module_selection_tags: Vec<GerbilLoopCaseModuleSelectionTag>,
    pub module_source_ref: String,
    pub module_entrypoint: String,
    pub module_enabled: bool,
    pub live_llm_required: bool,
    pub live_llm_allowed: bool,
    pub stable_fixture: bool,
    pub scheme_boundary: GerbilLoopCaseSchemeBoundary,
    pub serialization_boundary: GerbilLoopCaseSerializationBoundary,
}

impl GerbilLoopCaseDriverSchemeReceipt {
    pub fn runtime_handoff_no_live_llm_fixture(
        case_id: impl Into<GerbilLoopCaseDriverCaseId>,
        input_path: impl Into<PathBuf>,
    ) -> Self {
        Self {
            kind: GerbilLoopCaseDriverSchemeReceiptKind::MarlineKernelLoopCaseDriverReceipt,
            case_id: case_id.into(),
            profile_ref: GerbilLoopCaseDriverProfileRef::new(
                "custom/marline-kernel/profiles/default-loop-runtime.ss",
            ),
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
            module_kind: GerbilLoopCaseModuleKind::new("poo-flow.modules.user-selection.v1"),
            module_user_module: GerbilLoopCaseModuleName::new("funflow"),
            module_selection_tags: vec![
                GerbilLoopCaseModuleSelectionTag::new("+functional"),
                GerbilLoopCaseModuleSelectionTag::new("+dag"),
                GerbilLoopCaseModuleSelectionTag::new("+typed-receipts"),
                GerbilLoopCaseModuleSelectionTag::new("+runtime-manifest"),
            ],
            module_source_ref: "none".to_owned(),
            module_entrypoint: "none".to_owned(),
            module_enabled: true,
            scheme_boundary: GerbilLoopCaseSchemeBoundary::SchemeTypesToRustTypes,
            serialization_boundary:
                GerbilLoopCaseSerializationBoundary::RustOwnedCliTraceCrossProcess,
        }
    }

    pub fn typed_loop_projection_fixture(
        case_id: impl Into<GerbilLoopCaseDriverCaseId>,
        profile_ref: impl Into<GerbilLoopCaseDriverProfileRef>,
        module_selection_tags: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        let profile_ref = profile_ref.into();
        Self {
            kind: GerbilLoopCaseDriverSchemeReceiptKind::MarlineKernelLoopCaseDriverReceipt,
            case_id: case_id.into(),
            profile_ref: profile_ref.clone(),
            runtime_mode: GerbilLoopCaseRuntimeMode::TypedLoopProjection,
            live_gate_env: "none".to_owned(),
            live_enabled: false,
            smoke_status: GerbilLoopCaseSmokeStatus::TypedLoopProjectionReady,
            command_kind: GerbilLoopCaseCommandKind::LoopProgramRun,
            command_vector: vec![
                "marlin".to_owned(),
                "loop".to_owned(),
                "program".to_owned(),
                "run".to_owned(),
                "--profile".to_owned(),
                profile_ref.as_str().to_owned(),
            ],
            input_path: PathBuf::from("none"),
            stable_fixture: false,
            result_protocol: "marlin.config-interface.poo-loop-program-compiler.v1".to_owned(),
            policy_owner: "marlin".to_owned(),
            control_plane_owner: "poo-flow".to_owned(),
            runtime_execution_owner: "marlin-agent-core".to_owned(),
            module_kind: GerbilLoopCaseModuleKind::new(
                "marlin.config-interface.loop-policy-profile-projection.v1",
            ),
            module_user_module: GerbilLoopCaseModuleName::new("funflow"),
            module_selection_tags: module_selection_tags
                .into_iter()
                .map(GerbilLoopCaseModuleSelectionTag::new)
                .collect(),
            module_source_ref: profile_ref.as_str().to_owned(),
            module_entrypoint: "marlinLoopPolicyProfileCompilerReceipts".to_owned(),
            module_enabled: true,
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
    let runtime_handoff_status = match receipt.smoke_status {
        GerbilLoopCaseSmokeStatus::NoLiveLlmDenied if !receipt.live_enabled => {
            GerbilLoopCaseRuntimeHandoffStatus::DeferredNoLiveLlm
        }
        GerbilLoopCaseSmokeStatus::LiveEnabled
        | GerbilLoopCaseSmokeStatus::TypedLoopProjectionReady => {
            GerbilLoopCaseRuntimeHandoffStatus::Ready
        }
        GerbilLoopCaseSmokeStatus::NoLiveLlmDenied => GerbilLoopCaseRuntimeHandoffStatus::Ready,
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
        module_kind: receipt.module_kind.clone(),
        module_user_module: receipt.module_user_module.clone(),
        module_selection_tags: receipt.module_selection_tags.clone(),
        module_source_ref: receipt.module_source_ref.clone(),
        module_entrypoint: receipt.module_entrypoint.clone(),
        module_enabled: receipt.module_enabled,
        live_llm_required,
        live_llm_allowed: receipt.live_enabled,
        stable_fixture: receipt.stable_fixture,
        scheme_boundary: receipt.scheme_boundary,
        serialization_boundary: receipt.serialization_boundary,
    }
}
