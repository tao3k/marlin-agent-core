//! Rust-side typed gates for Deck runtime Scheme script interfaces.

use crate::deck_runtime_policy::{
    GERBIL_DECK_RUNTIME_POO_POLICY_PROJECTION_TYPE_ID, GerbilDeckRuntimePooPolicyProjection,
    gerbil_deck_runtime_poo_policy_projection_type_manifest,
};
use crate::scheme_types::{
    GerbilSchemeFieldName, GerbilSchemeProjectionContract, GerbilSchemeSchemaId,
    GerbilSchemeTypeDecodeError, GerbilSchemeTypeFieldSpec, GerbilSchemeTypeId,
    GerbilSchemeTypeManifest, GerbilSchemeTypeRegistry, GerbilSchemeTypeSpec,
    GerbilSchemeTypedProjection, GerbilSchemeTypedValue, GerbilSchemeValue,
};
use serde::{Deserialize, Serialize};

/// Kind emitted by `:marlin/deck-runtime-script` for script objects.
pub const GERBIL_DECK_RUNTIME_SCRIPT_KIND: &str = "marlin-deck-runtime.script.v1";
/// Interface name for downstream script entrypoints that can use POO or native ABI.
pub const GERBIL_DECK_RUNTIME_SCRIPT_INTERFACE_KIND: &str = "poo-native-api-or-gxi-script";
/// Type id for script interface receipts projected into Rust.
pub const GERBIL_DECK_RUNTIME_SCRIPT_INTERFACE_RECEIPT_TYPE_ID: &str =
    "marlin.deck-runtime.script-interface-receipt";
/// Schema id emitted by Scheme script interface receipts.
pub const GERBIL_DECK_RUNTIME_SCRIPT_INTERFACE_RECEIPT_SCHEMA_ID: &str =
    "marlin-deck-runtime.script-interface-receipt.v1";
/// Type id for script batch metrics projected into Rust.
pub const GERBIL_DECK_RUNTIME_SCRIPT_BATCH_METRICS_TYPE_ID: &str =
    "marlin.deck-runtime.script-batch-metrics";
/// Schema id emitted by Scheme script batch metrics.
pub const GERBIL_DECK_RUNTIME_SCRIPT_BATCH_METRICS_SCHEMA_ID: &str =
    "marlin-deck-runtime.script-batch-metrics.v1";
/// Default script batch size used by the quick script performance gate.
pub const GERBIL_DECK_RUNTIME_SCRIPT_BATCH_DEFAULT_ITERATIONS: u64 = 128;
/// Default Rust-owned budget for one quick script batch.
pub const GERBIL_DECK_RUNTIME_SCRIPT_BATCH_MAX_ELAPSED_US: u64 = 50_000;

macro_rules! string_newtype {
    ($name:ident) => {
        #[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self::new(value)
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self::new(value)
            }
        }
    };
}

string_newtype!(GerbilDeckRuntimeScriptAction);
string_newtype!(GerbilDeckRuntimeScriptBatchMetricsKind);
string_newtype!(GerbilDeckRuntimeScriptExtensionId);
string_newtype!(GerbilDeckRuntimeScriptId);
string_newtype!(GerbilDeckRuntimeScriptInterfaceKind);
string_newtype!(GerbilDeckRuntimeScriptInterfaceReceiptKind);

/// Typed receipt for a Scheme quick script interface.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GerbilDeckRuntimeScriptInterfaceReceipt {
    pub kind: GerbilDeckRuntimeScriptInterfaceReceiptKind,
    #[serde(rename = "script-id")]
    pub script_id: GerbilDeckRuntimeScriptId,
    pub interface: GerbilDeckRuntimeScriptInterfaceKind,
    pub action: GerbilDeckRuntimeScriptAction,
    #[serde(rename = "extension-id")]
    pub extension_id: GerbilDeckRuntimeScriptExtensionId,
    pub metadata: GerbilSchemeValue,
    #[serde(rename = "native-projection")]
    pub native_projection: GerbilDeckRuntimePooPolicyProjection,
}

impl GerbilSchemeTypedProjection for GerbilDeckRuntimeScriptInterfaceReceipt {
    fn scheme_projection_contract() -> GerbilSchemeProjectionContract {
        gerbil_deck_runtime_script_interface_receipt_contract()
    }
}

impl GerbilDeckRuntimeScriptInterfaceReceipt {
    pub fn native_projection(&self) -> &GerbilDeckRuntimePooPolicyProjection {
        &self.native_projection
    }
}

/// Typed metrics emitted by a Scheme quick script batch run.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilDeckRuntimeScriptBatchMetrics {
    pub kind: GerbilDeckRuntimeScriptBatchMetricsKind,
    #[serde(rename = "script-id")]
    pub script_id: GerbilDeckRuntimeScriptId,
    pub interface: GerbilDeckRuntimeScriptInterfaceKind,
    pub iterations: u64,
    pub runs: u64,
    #[serde(rename = "elapsed-us")]
    pub elapsed_us: u64,
}

impl GerbilSchemeTypedProjection for GerbilDeckRuntimeScriptBatchMetrics {
    fn scheme_projection_contract() -> GerbilSchemeProjectionContract {
        gerbil_deck_runtime_script_batch_metrics_contract()
    }
}

/// Rust-owned script batch performance budget.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilDeckRuntimeScriptBatchPerformanceBudget {
    pub min_iterations: u64,
    pub max_elapsed_us: u64,
}

impl GerbilDeckRuntimeScriptBatchPerformanceBudget {
    pub fn new(min_iterations: u64, max_elapsed_us: u64) -> Self {
        Self {
            min_iterations,
            max_elapsed_us,
        }
    }
}

impl Default for GerbilDeckRuntimeScriptBatchPerformanceBudget {
    fn default() -> Self {
        Self::new(
            GERBIL_DECK_RUNTIME_SCRIPT_BATCH_DEFAULT_ITERATIONS,
            GERBIL_DECK_RUNTIME_SCRIPT_BATCH_MAX_ELAPSED_US,
        )
    }
}

/// Result of checking Scheme-emitted script batch metrics against a Rust budget.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GerbilDeckRuntimeScriptBatchPerformanceStatus {
    WithinBudget,
    UnderSampled,
    OverBudget,
}

/// Typed Rust receipt for script batch performance evaluation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilDeckRuntimeScriptBatchPerformanceReceipt {
    pub status: GerbilDeckRuntimeScriptBatchPerformanceStatus,
    pub metrics: GerbilDeckRuntimeScriptBatchMetrics,
    pub min_iterations: u64,
    pub max_elapsed_us: u64,
}

impl GerbilDeckRuntimeScriptBatchPerformanceReceipt {
    pub fn within_budget(&self) -> bool {
        self.status == GerbilDeckRuntimeScriptBatchPerformanceStatus::WithinBudget
    }
}

/// Contract expected for script interface receipts.
pub fn gerbil_deck_runtime_script_interface_receipt_contract() -> GerbilSchemeProjectionContract {
    GerbilSchemeProjectionContract::new(GerbilSchemeTypeId::new(
        GERBIL_DECK_RUNTIME_SCRIPT_INTERFACE_RECEIPT_TYPE_ID,
    ))
    .with_schema_id(GerbilSchemeSchemaId::new(
        GERBIL_DECK_RUNTIME_SCRIPT_INTERFACE_RECEIPT_SCHEMA_ID,
    ))
}

/// Contract expected for script batch metrics.
pub fn gerbil_deck_runtime_script_batch_metrics_contract() -> GerbilSchemeProjectionContract {
    GerbilSchemeProjectionContract::new(GerbilSchemeTypeId::new(
        GERBIL_DECK_RUNTIME_SCRIPT_BATCH_METRICS_TYPE_ID,
    ))
    .with_schema_id(GerbilSchemeSchemaId::new(
        GERBIL_DECK_RUNTIME_SCRIPT_BATCH_METRICS_SCHEMA_ID,
    ))
}

/// Scheme type manifest for the Rust-side quick script interface gate.
pub fn gerbil_deck_runtime_script_interface_type_manifest() -> GerbilSchemeTypeManifest {
    let mut types = gerbil_deck_runtime_poo_policy_projection_type_manifest().types;
    types.extend([
        GerbilSchemeTypeSpec {
            type_id: GerbilSchemeTypeId::new(GERBIL_DECK_RUNTIME_SCRIPT_INTERFACE_RECEIPT_TYPE_ID),
            schema_id: Some(GerbilSchemeSchemaId::new(
                GERBIL_DECK_RUNTIME_SCRIPT_INTERFACE_RECEIPT_SCHEMA_ID,
            )),
            fields: [
                required_script_field("kind", "string"),
                required_script_field("script-id", "string"),
                required_script_field("interface", "string"),
                required_script_field("action", "string"),
                required_script_field("extension-id", "string"),
                required_script_field("metadata", "object"),
                required_script_field(
                    "native-projection",
                    GERBIL_DECK_RUNTIME_POO_POLICY_PROJECTION_TYPE_ID,
                ),
            ]
            .into(),
        },
        GerbilSchemeTypeSpec {
            type_id: GerbilSchemeTypeId::new(GERBIL_DECK_RUNTIME_SCRIPT_BATCH_METRICS_TYPE_ID),
            schema_id: Some(GerbilSchemeSchemaId::new(
                GERBIL_DECK_RUNTIME_SCRIPT_BATCH_METRICS_SCHEMA_ID,
            )),
            fields: [
                required_script_field("kind", "string"),
                required_script_field("script-id", "string"),
                required_script_field("interface", "string"),
                required_script_field("iterations", "integer"),
                required_script_field("runs", "integer"),
                required_script_field("elapsed-us", "integer"),
            ]
            .into(),
        },
    ]);

    GerbilSchemeTypeManifest {
        schema_id: GerbilSchemeSchemaId::new("marlin.scheme-types.manifest.v1"),
        types,
    }
}

/// Decode a Scheme quick script interface receipt through the Rust type registry.
pub fn decode_gerbil_deck_runtime_script_interface_receipt(
    registry: &GerbilSchemeTypeRegistry,
    typed_value: &GerbilSchemeTypedValue,
) -> Result<GerbilDeckRuntimeScriptInterfaceReceipt, GerbilSchemeTypeDecodeError> {
    registry.decode_projection(typed_value)
}

/// Decode Scheme quick script batch metrics through the Rust type registry.
pub fn decode_gerbil_deck_runtime_script_batch_metrics(
    registry: &GerbilSchemeTypeRegistry,
    typed_value: &GerbilSchemeTypedValue,
) -> Result<GerbilDeckRuntimeScriptBatchMetrics, GerbilSchemeTypeDecodeError> {
    registry.decode_projection(typed_value)
}

/// Evaluate Scheme-emitted script batch metrics against a Rust-owned budget.
pub fn evaluate_gerbil_deck_runtime_script_batch_performance(
    metrics: GerbilDeckRuntimeScriptBatchMetrics,
    budget: &GerbilDeckRuntimeScriptBatchPerformanceBudget,
) -> GerbilDeckRuntimeScriptBatchPerformanceReceipt {
    let status = if metrics.iterations < budget.min_iterations || metrics.runs < metrics.iterations
    {
        GerbilDeckRuntimeScriptBatchPerformanceStatus::UnderSampled
    } else if metrics.elapsed_us > budget.max_elapsed_us {
        GerbilDeckRuntimeScriptBatchPerformanceStatus::OverBudget
    } else {
        GerbilDeckRuntimeScriptBatchPerformanceStatus::WithinBudget
    };

    GerbilDeckRuntimeScriptBatchPerformanceReceipt {
        status,
        metrics,
        min_iterations: budget.min_iterations,
        max_elapsed_us: budget.max_elapsed_us,
    }
}

fn required_script_field(name: &str, type_id: &str) -> GerbilSchemeTypeFieldSpec {
    GerbilSchemeTypeFieldSpec {
        name: GerbilSchemeFieldName::new(name),
        type_id: GerbilSchemeTypeId::new(type_id),
        element_type_id: None,
        required: true,
        description: None,
    }
}
