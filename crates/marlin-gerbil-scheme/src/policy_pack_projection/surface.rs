//! Typed Rust projection for Gerbil policy-pack receipt chains.

use marlin_agent_protocol::{
    LOOP_PROGRAM_SCHEMA_VERSION, LoopProgram, RESOLVED_LOOP_POLICY_PACK_SCHEMA_VERSION,
    ResolvedLoopPolicyPack,
};
use serde::{Deserialize, Serialize};

use crate::{
    GerbilSchemeFieldName, GerbilSchemePackageId, GerbilSchemePackageManifest,
    GerbilSchemeProjectionContract, GerbilSchemeSchemaId, GerbilSchemeTypeDecodeError,
    GerbilSchemeTypeFieldSpec, GerbilSchemeTypeId, GerbilSchemeTypeManifest,
    GerbilSchemeTypeRegistry, GerbilSchemeTypeSpec, GerbilSchemeTypedProjection,
    GerbilSchemeTypedValue,
};

use super::optional_scheme::deserialize_optional_scheme_string;

/// Package id for the policy-pack typed projection manifest.
pub const GERBIL_POLICY_PACK_PROJECTION_CHAIN_PACKAGE_ID: &str =
    "marlin.config-interface.policy-pack.projection-chain";

/// Type id for Gerbil policy-pack projection-chain receipts.
pub const GERBIL_POLICY_PACK_PROJECTION_CHAIN_TYPE_ID: &str =
    "marlin.config-interface.policy-projection-chain-receipt";

/// Schema id carried by Gerbil policy-pack projection-chain receipts.
pub const GERBIL_POLICY_PACK_PROJECTION_CHAIN_SCHEMA_ID: &str =
    "marlin.config-interface.policy-projection-chain-receipt.v1";

/// Package id for the resolved loop policy pack typed projection manifest.
pub const GERBIL_RESOLVED_LOOP_POLICY_PACK_PACKAGE_ID: &str =
    "marlin.config-interface.policy-pack.resolved-loop-policy-pack";

/// Type id for Gerbil-resolved loop policy packs.
pub const GERBIL_RESOLVED_LOOP_POLICY_PACK_TYPE_ID: &str = "marlin.loop-policy.resolved-pack";

/// Schema id carried by Gerbil-resolved loop policy packs.
pub const GERBIL_RESOLVED_LOOP_POLICY_PACK_SCHEMA_ID: &str = "marlin.loop-policy.resolved-pack.v1";

/// Package id for POO compiler receipts that carry a provider-neutral `LoopProgram`.
pub const GERBIL_POO_LOOP_PROGRAM_COMPILER_PACKAGE_ID: &str =
    "marlin.config-interface.poo.loop-program-compiler";

/// Type id for POO compiler receipts that bind a resolved policy pack to a `LoopProgram`.
pub const GERBIL_POO_LOOP_PROGRAM_COMPILER_TYPE_ID: &str =
    "marlin.config-interface.poo.loop-program-compiler-receipt";

/// Schema id carried by POO compiler receipts.
pub const GERBIL_POO_LOOP_PROGRAM_COMPILER_SCHEMA_ID: &str =
    "marlin.config-interface.poo.loop-program-compiler-receipt.v1";

/// Package id for loop policy projection modules exported by config-interface.
pub const GERBIL_LOOP_POLICY_PROJECTION_MODULE_PACKAGE_ID: &str =
    "marlin.config-interface.loop-policy.profile-projection-module";

/// Type id for loop policy projection modules exported by config-interface.
pub const GERBIL_LOOP_POLICY_PROJECTION_MODULE_TYPE_ID: &str =
    "marlin.config-interface.loop-policy.profile-projection-module";

/// Schema id carried by loop policy projection modules.
pub const GERBIL_LOOP_POLICY_PROJECTION_MODULE_SCHEMA_ID: &str =
    "marlin.config-interface.loop-policy.profile-projection-module.v1";

const POLICY_PACK_RECEIPT_FAMILY_IDS: [&str; 5] = [
    "module_evaluation_receipt",
    "policy_projection_receipt",
    "native_projection_payload",
    "budget_receipt",
    "catalog_resolution_receipt",
];

/// Typed kind identifier for policy-pack receipt projections.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GerbilPolicyPackReceiptKind(String);

impl GerbilPolicyPackReceiptKind {
    pub fn new(kind: impl Into<String>) -> Self {
        Self(kind.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for GerbilPolicyPackReceiptKind {
    fn from(kind: &str) -> Self {
        Self::new(kind)
    }
}

impl From<String> for GerbilPolicyPackReceiptKind {
    fn from(kind: String) -> Self {
        Self::new(kind)
    }
}

/// POO profile id that produced a concrete `LoopProgram`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GerbilPooLoopProgramProfileId(String);

impl GerbilPooLoopProgramProfileId {
    pub fn new(profile_id: impl Into<String>) -> Self {
        Self(profile_id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Fixed owner lane for the Scheme-side POO compiler.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GerbilPooLoopProgramCompilerOwner {
    GerbilPooFlow,
}

/// Native internal boundary for POO compiler receipts.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GerbilPooLoopProgramCompilerBoundary {
    SchemeTypesToRustTypes,
}

/// Serialization boundary retained outside native runtime internals.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GerbilPooLoopProgramCompilerSerializationBoundary {
    RustOwnedCliTraceCrossProcess,
}

/// Minimal typed view of a nested policy-pack receipt.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilPolicyPackReceiptSummary {
    pub kind: GerbilPolicyPackReceiptKind,
    #[serde(rename = "pack-id")]
    pub pack_id: String,
    pub owner: String,
    #[serde(default)]
    pub replayable: bool,
}

/// Rust projection of the Gerbil policy-pack projection-chain receipt.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilPolicyPackProjectionChainReceipt {
    pub kind: GerbilPolicyPackReceiptKind,
    #[serde(rename = "pack-id")]
    pub pack_id: String,
    #[serde(rename = "receipt-family-count")]
    pub receipt_family_count: usize,
    #[serde(rename = "receipt-family-ids")]
    pub receipt_family_ids: Vec<String>,
    #[serde(rename = "module-evaluation-receipt")]
    pub module_evaluation_receipt: GerbilPolicyPackReceiptSummary,
    #[serde(rename = "policy-projection-receipt")]
    pub policy_projection_receipt: GerbilPolicyPackReceiptSummary,
    #[serde(rename = "native-projection-payload")]
    pub native_projection_payload: GerbilPolicyPackReceiptSummary,
    #[serde(rename = "budget-receipt")]
    pub budget_receipt: GerbilPolicyPackReceiptSummary,
    #[serde(rename = "catalog-resolution-receipt")]
    pub catalog_resolution_receipt: GerbilPolicyPackReceiptSummary,
    #[serde(rename = "module-evaluation-receipt-owner")]
    pub module_evaluation_receipt_owner: String,
    #[serde(rename = "policy-projection-receipt-owner")]
    pub policy_projection_receipt_owner: String,
    #[serde(rename = "native-projection-payload-owner")]
    pub native_projection_payload_owner: String,
    #[serde(rename = "budget-receipt-owner")]
    pub budget_receipt_owner: String,
    #[serde(rename = "catalog-resolution-receipt-owner")]
    pub catalog_resolution_receipt_owner: String,
    #[serde(rename = "catalog-resolution-allowed-hook-count")]
    pub catalog_resolution_allowed_hook_count: usize,
    pub replayable: bool,
}

/// POO compiler receipt that binds the resolved hot/audit policy pack to a runtime program.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilPooLoopProgramCompilerReceipt {
    pub kind: GerbilPolicyPackReceiptKind,
    #[serde(rename = "profile-id")]
    pub profile_id: GerbilPooLoopProgramProfileId,
    #[serde(rename = "compiler-owner")]
    pub compiler_owner: GerbilPooLoopProgramCompilerOwner,
    #[serde(rename = "resolved-policy-pack")]
    pub resolved_policy_pack: ResolvedLoopPolicyPack,
    #[serde(rename = "loop-program")]
    pub loop_program: LoopProgram,
    #[serde(rename = "scheme-boundary")]
    pub scheme_boundary: GerbilPooLoopProgramCompilerBoundary,
    #[serde(rename = "serialization-boundary")]
    pub serialization_boundary: GerbilPooLoopProgramCompilerSerializationBoundary,
}

/// Config-interface module projection that carries a typed POO compiler receipt.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilLoopPolicyProjectionModule {
    pub kind: GerbilPolicyPackReceiptKind,
    #[serde(rename = "module-id")]
    pub module_id: String,
    #[serde(rename = "profile-id")]
    pub profile_id: GerbilPooLoopProgramProfileId,
    pub owner: GerbilPooLoopProgramCompilerOwner,
    #[serde(rename = "source-module")]
    pub source_module: String,
    #[serde(rename = "poo-flow-module")]
    pub poo_flow_module: String,
    #[serde(rename = "poo-flow-capability-lanes")]
    pub poo_flow_capability_lanes: Vec<String>,
    #[serde(
        rename = "vertical-case-id",
        deserialize_with = "deserialize_optional_scheme_string"
    )]
    pub vertical_case_id: Option<String>,
    #[serde(rename = "vertical-capability-tags")]
    pub vertical_capability_tags: Vec<String>,
    #[serde(rename = "vertical-mainline?")]
    pub vertical_mainline: bool,
    #[serde(rename = "rust-type")]
    pub rust_type: GerbilPolicyPackReceiptKind,
    #[serde(rename = "scheme-boundary")]
    pub scheme_boundary: GerbilPooLoopProgramCompilerBoundary,
    #[serde(rename = "serialization-boundary")]
    pub serialization_boundary: GerbilPooLoopProgramCompilerSerializationBoundary,
    #[serde(rename = "compiler-receipt")]
    pub compiler_receipt: GerbilPooLoopProgramCompilerReceipt,
}

impl GerbilPolicyPackProjectionChainReceipt {
    /// Returns whether the receipt kind matches the current Rust projection.
    pub fn has_current_schema(&self) -> bool {
        self.kind.as_str() == GERBIL_POLICY_PACK_PROJECTION_CHAIN_SCHEMA_ID
    }

    fn ensure_current_schema(&self) -> Result<(), GerbilSchemeTypeDecodeError> {
        if !self.has_current_schema() {
            return Err(GerbilSchemeTypeDecodeError::RustProjection {
                message: format!(
                    "policy-pack projection-chain kind {} does not match {}",
                    self.kind.as_str(),
                    GERBIL_POLICY_PACK_PROJECTION_CHAIN_SCHEMA_ID
                ),
            });
        }

        if self.receipt_family_count != POLICY_PACK_RECEIPT_FAMILY_IDS.len() {
            return Err(GerbilSchemeTypeDecodeError::RustProjection {
                message: format!(
                    "policy-pack projection-chain family count {} does not match {}",
                    self.receipt_family_count,
                    POLICY_PACK_RECEIPT_FAMILY_IDS.len()
                ),
            });
        }

        let expected_family_ids: Vec<_> = POLICY_PACK_RECEIPT_FAMILY_IDS
            .iter()
            .map(|family| (*family).to_owned())
            .collect();
        if self.receipt_family_ids != expected_family_ids {
            return Err(GerbilSchemeTypeDecodeError::RustProjection {
                message: format!(
                    "policy-pack projection-chain family ids {:?} do not match {:?}",
                    self.receipt_family_ids, expected_family_ids
                ),
            });
        }

        self.ensure_owner_matches(
            "module-evaluation-receipt-owner",
            &self.module_evaluation_receipt_owner,
            &self.module_evaluation_receipt,
        )?;
        self.ensure_owner_matches(
            "policy-projection-receipt-owner",
            &self.policy_projection_receipt_owner,
            &self.policy_projection_receipt,
        )?;
        self.ensure_owner_matches(
            "native-projection-payload-owner",
            &self.native_projection_payload_owner,
            &self.native_projection_payload,
        )?;
        self.ensure_owner_matches(
            "budget-receipt-owner",
            &self.budget_receipt_owner,
            &self.budget_receipt,
        )?;
        self.ensure_owner_matches(
            "catalog-resolution-receipt-owner",
            &self.catalog_resolution_receipt_owner,
            &self.catalog_resolution_receipt,
        )?;

        Ok(())
    }

    fn ensure_owner_matches(
        &self,
        field_name: &'static str,
        owner: &str,
        nested_receipt: &GerbilPolicyPackReceiptSummary,
    ) -> Result<(), GerbilSchemeTypeDecodeError> {
        if owner == nested_receipt.owner {
            Ok(())
        } else {
            Err(GerbilSchemeTypeDecodeError::RustProjection {
                message: format!(
                    "policy-pack projection-chain {field_name} {owner} does not match nested owner {}",
                    nested_receipt.owner
                ),
            })
        }
    }
}

impl GerbilPooLoopProgramCompilerReceipt {
    /// Returns whether the receipt kind matches the current Rust projection.
    pub fn has_current_schema(&self) -> bool {
        self.kind.as_str() == GERBIL_POO_LOOP_PROGRAM_COMPILER_SCHEMA_ID
    }

    fn ensure_current_schema(&self) -> Result<(), GerbilSchemeTypeDecodeError> {
        if !self.has_current_schema() {
            return Err(GerbilSchemeTypeDecodeError::RustProjection {
                message: format!(
                    "POO loop-program compiler kind {} does not match {}",
                    self.kind.as_str(),
                    GERBIL_POO_LOOP_PROGRAM_COMPILER_SCHEMA_ID
                ),
            });
        }

        if self.resolved_policy_pack.schema_version != RESOLVED_LOOP_POLICY_PACK_SCHEMA_VERSION {
            return Err(GerbilSchemeTypeDecodeError::RustProjection {
                message: format!(
                    "POO compiler resolved policy pack schema version {} does not match {}",
                    self.resolved_policy_pack.schema_version,
                    RESOLVED_LOOP_POLICY_PACK_SCHEMA_VERSION
                ),
            });
        }

        if self.loop_program.schema_version != LOOP_PROGRAM_SCHEMA_VERSION {
            return Err(GerbilSchemeTypeDecodeError::RustProjection {
                message: format!(
                    "POO compiler loop program schema version {} does not match {}",
                    self.loop_program.schema_version, LOOP_PROGRAM_SCHEMA_VERSION
                ),
            });
        }

        if self.loop_program.policy_epoch != self.resolved_policy_pack.policy_epoch {
            return Err(GerbilSchemeTypeDecodeError::RustProjection {
                message: format!(
                    "POO compiler loop program epoch {} does not match resolved policy pack epoch {}",
                    self.loop_program.policy_epoch.get(),
                    self.resolved_policy_pack.policy_epoch.get()
                ),
            });
        }

        if self.loop_program.policy_digest != self.resolved_policy_pack.policy_digest {
            return Err(GerbilSchemeTypeDecodeError::RustProjection {
                message: "POO compiler loop program policy digest does not match resolved policy pack digest".to_owned(),
            });
        }

        if self.loop_program.mechanism_policies.is_empty() {
            return Err(GerbilSchemeTypeDecodeError::RustProjection {
                message: "POO compiler loop program must carry at least one mechanism policy"
                    .to_owned(),
            });
        }

        if self.loop_program.transitions.is_empty() {
            return Err(GerbilSchemeTypeDecodeError::RustProjection {
                message: "POO compiler loop program must carry at least one transition".to_owned(),
            });
        }

        if self.resolved_policy_pack.hot.graph_nodes.is_empty() {
            return Err(GerbilSchemeTypeDecodeError::RustProjection {
                message: "POO compiler resolved policy pack must carry hot graph nodes".to_owned(),
            });
        }

        if self.resolved_policy_pack.audit.provenance.is_empty() {
            return Err(GerbilSchemeTypeDecodeError::RustProjection {
                message: "POO compiler resolved policy pack must carry policy provenance"
                    .to_owned(),
            });
        }

        if self.resolved_policy_pack.audit.linearization.is_empty() {
            return Err(GerbilSchemeTypeDecodeError::RustProjection {
                message: "POO compiler resolved policy pack must carry POO linearization"
                    .to_owned(),
            });
        }

        if self.resolved_policy_pack.audit.forced_slots.is_empty() {
            return Err(GerbilSchemeTypeDecodeError::RustProjection {
                message: "POO compiler resolved policy pack must carry forced slots".to_owned(),
            });
        }

        if self.resolved_policy_pack.audit.merge_receipts.is_empty() {
            return Err(GerbilSchemeTypeDecodeError::RustProjection {
                message: "POO compiler resolved policy pack must carry merge receipts".to_owned(),
            });
        }

        Ok(())
    }
}

impl GerbilLoopPolicyProjectionModule {
    /// Returns whether the module kind matches the current Rust projection.
    pub fn has_current_schema(&self) -> bool {
        self.kind.as_str() == GERBIL_LOOP_POLICY_PROJECTION_MODULE_SCHEMA_ID
    }

    fn ensure_current_schema(&self) -> Result<(), GerbilSchemeTypeDecodeError> {
        if !self.has_current_schema() {
            return Err(GerbilSchemeTypeDecodeError::RustProjection {
                message: format!(
                    "loop policy projection module kind {} does not match {}",
                    self.kind.as_str(),
                    GERBIL_LOOP_POLICY_PROJECTION_MODULE_SCHEMA_ID
                ),
            });
        }

        if self.owner != GerbilPooLoopProgramCompilerOwner::GerbilPooFlow {
            return Err(GerbilSchemeTypeDecodeError::RustProjection {
                message: "loop policy projection module owner must be gerbil-poo-flow".to_owned(),
            });
        }

        if self.rust_type.as_str() != GERBIL_POO_LOOP_PROGRAM_COMPILER_SCHEMA_ID {
            return Err(GerbilSchemeTypeDecodeError::RustProjection {
                message: format!(
                    "loop policy projection module rust type {} does not match {}",
                    self.rust_type.as_str(),
                    GERBIL_POO_LOOP_PROGRAM_COMPILER_SCHEMA_ID
                ),
            });
        }

        if self.profile_id != self.compiler_receipt.profile_id {
            return Err(GerbilSchemeTypeDecodeError::RustProjection {
                message: format!(
                    "loop policy projection module profile {} does not match compiler receipt profile {}",
                    self.profile_id.as_str(),
                    self.compiler_receipt.profile_id.as_str()
                ),
            });
        }

        if self.poo_flow_module.is_empty() || self.poo_flow_capability_lanes.is_empty() {
            return Err(GerbilSchemeTypeDecodeError::RustProjection {
                message: "loop policy projection module must carry a POO Flow module lane"
                    .to_owned(),
            });
        }

        if self
            .vertical_capability_tags
            .iter()
            .any(|tag| tag.trim().is_empty())
        {
            return Err(GerbilSchemeTypeDecodeError::RustProjection {
                message: "loop policy projection module vertical capability tags must be non-empty"
                    .to_owned(),
            });
        }

        if self.vertical_mainline {
            let Some(case_id) = self.vertical_case_id.as_deref() else {
                return Err(GerbilSchemeTypeDecodeError::RustProjection {
                    message: "loop policy projection module mainline requires vertical case id"
                        .to_owned(),
                });
            };

            if case_id.trim().is_empty() {
                return Err(GerbilSchemeTypeDecodeError::RustProjection {
                    message: "loop policy projection module vertical case id must be non-empty"
                        .to_owned(),
                });
            }

            if self.vertical_capability_tags.is_empty() {
                return Err(GerbilSchemeTypeDecodeError::RustProjection {
                    message:
                        "loop policy projection module mainline requires vertical capability tags"
                            .to_owned(),
                });
            }
        } else if self.vertical_case_id.is_some() || !self.vertical_capability_tags.is_empty() {
            return Err(GerbilSchemeTypeDecodeError::RustProjection {
                message:
                    "loop policy projection module non-mainline cannot carry vertical descriptor"
                        .to_owned(),
            });
        }

        self.compiler_receipt.ensure_current_schema()
    }
}

impl GerbilSchemeTypedProjection for GerbilPolicyPackProjectionChainReceipt {
    fn scheme_projection_contract() -> GerbilSchemeProjectionContract {
        gerbil_policy_pack_projection_chain_contract()
    }
}

impl GerbilSchemeTypedProjection for ResolvedLoopPolicyPack {
    fn scheme_projection_contract() -> GerbilSchemeProjectionContract {
        gerbil_resolved_loop_policy_pack_contract()
    }
}

impl GerbilSchemeTypedProjection for GerbilPooLoopProgramCompilerReceipt {
    fn scheme_projection_contract() -> GerbilSchemeProjectionContract {
        gerbil_poo_loop_program_compiler_contract()
    }
}

impl GerbilSchemeTypedProjection for GerbilLoopPolicyProjectionModule {
    fn scheme_projection_contract() -> GerbilSchemeProjectionContract {
        gerbil_loop_policy_projection_module_contract()
    }
}

/// Contract expected for the Gerbil policy-pack projection-chain receipt.
pub fn gerbil_policy_pack_projection_chain_contract() -> GerbilSchemeProjectionContract {
    GerbilSchemeProjectionContract::new(GerbilSchemeTypeId::new(
        GERBIL_POLICY_PACK_PROJECTION_CHAIN_TYPE_ID,
    ))
    .with_schema_id(GerbilSchemeSchemaId::new(
        GERBIL_POLICY_PACK_PROJECTION_CHAIN_SCHEMA_ID,
    ))
}

/// Scheme type manifest for policy-pack projection-chain receipts.
pub fn gerbil_policy_pack_projection_chain_type_manifest() -> GerbilSchemeTypeManifest {
    GerbilSchemeTypeManifest {
        schema_id: GerbilSchemeSchemaId::new("marlin.scheme-types.manifest.v1"),
        types: vec![GerbilSchemeTypeSpec {
            type_id: GerbilSchemeTypeId::new(GERBIL_POLICY_PACK_PROJECTION_CHAIN_TYPE_ID),
            schema_id: Some(GerbilSchemeSchemaId::new(
                GERBIL_POLICY_PACK_PROJECTION_CHAIN_SCHEMA_ID,
            )),
            fields: vec![
                required_policy_pack_chain_field("kind", "string", None),
                required_policy_pack_chain_field("pack-id", "string", None),
                required_policy_pack_chain_field("receipt-family-count", "integer", None),
                required_policy_pack_chain_field("receipt-family-ids", "array", Some("string")),
                required_policy_pack_chain_field("module-evaluation-receipt", "object", None),
                required_policy_pack_chain_field("policy-projection-receipt", "object", None),
                required_policy_pack_chain_field("native-projection-payload", "object", None),
                required_policy_pack_chain_field("budget-receipt", "object", None),
                required_policy_pack_chain_field("catalog-resolution-receipt", "object", None),
                required_policy_pack_chain_field("module-evaluation-receipt-owner", "string", None),
                required_policy_pack_chain_field("policy-projection-receipt-owner", "string", None),
                required_policy_pack_chain_field("native-projection-payload-owner", "string", None),
                required_policy_pack_chain_field("budget-receipt-owner", "string", None),
                required_policy_pack_chain_field(
                    "catalog-resolution-receipt-owner",
                    "string",
                    None,
                ),
                required_policy_pack_chain_field(
                    "catalog-resolution-allowed-hook-count",
                    "integer",
                    None,
                ),
                required_policy_pack_chain_field("replayable", "boolean", None),
            ],
        }],
    }
}

/// Package manifest for policy-pack typed projection receipts.
pub fn gerbil_policy_pack_projection_chain_package_manifest() -> GerbilSchemePackageManifest {
    GerbilSchemePackageManifest::new(
        GerbilSchemePackageId::new(GERBIL_POLICY_PACK_PROJECTION_CHAIN_PACKAGE_ID),
        gerbil_policy_pack_projection_chain_type_manifest(),
    )
    .with_projection_contracts([gerbil_policy_pack_projection_chain_contract()])
}

/// Contract expected for a Gerbil-resolved loop policy pack.
pub fn gerbil_resolved_loop_policy_pack_contract() -> GerbilSchemeProjectionContract {
    GerbilSchemeProjectionContract::new(GerbilSchemeTypeId::new(
        GERBIL_RESOLVED_LOOP_POLICY_PACK_TYPE_ID,
    ))
    .with_schema_id(GerbilSchemeSchemaId::new(
        GERBIL_RESOLVED_LOOP_POLICY_PACK_SCHEMA_ID,
    ))
}

/// Contract expected for a POO compiler receipt carrying a runtime loop program.
pub fn gerbil_poo_loop_program_compiler_contract() -> GerbilSchemeProjectionContract {
    GerbilSchemeProjectionContract::new(GerbilSchemeTypeId::new(
        GERBIL_POO_LOOP_PROGRAM_COMPILER_TYPE_ID,
    ))
    .with_schema_id(GerbilSchemeSchemaId::new(
        GERBIL_POO_LOOP_PROGRAM_COMPILER_SCHEMA_ID,
    ))
}

/// Contract expected for config-interface loop policy projection modules.
pub fn gerbil_loop_policy_projection_module_contract() -> GerbilSchemeProjectionContract {
    GerbilSchemeProjectionContract::new(GerbilSchemeTypeId::new(
        GERBIL_LOOP_POLICY_PROJECTION_MODULE_TYPE_ID,
    ))
    .with_schema_id(GerbilSchemeSchemaId::new(
        GERBIL_LOOP_POLICY_PROJECTION_MODULE_SCHEMA_ID,
    ))
}

/// Scheme type manifest for Gerbil-resolved loop policy packs.
pub fn gerbil_resolved_loop_policy_pack_type_manifest() -> GerbilSchemeTypeManifest {
    GerbilSchemeTypeManifest {
        schema_id: GerbilSchemeSchemaId::new("marlin.scheme-types.manifest.v1"),
        types: vec![GerbilSchemeTypeSpec {
            type_id: GerbilSchemeTypeId::new(GERBIL_RESOLVED_LOOP_POLICY_PACK_TYPE_ID),
            schema_id: Some(GerbilSchemeSchemaId::new(
                GERBIL_RESOLVED_LOOP_POLICY_PACK_SCHEMA_ID,
            )),
            fields: vec![
                required_policy_pack_chain_field("schema_version", "integer", None),
                required_policy_pack_chain_field("policy_epoch", "integer", None),
                required_policy_pack_chain_field("policy_digest", "array", Some("integer")),
                required_policy_pack_chain_field("hot", "object", None),
                required_policy_pack_chain_field("audit", "object", None),
            ],
        }],
    }
}

/// Scheme type manifest for POO compiler receipts.
pub fn gerbil_poo_loop_program_compiler_type_manifest() -> GerbilSchemeTypeManifest {
    GerbilSchemeTypeManifest {
        schema_id: GerbilSchemeSchemaId::new("marlin.scheme-types.manifest.v1"),
        types: vec![GerbilSchemeTypeSpec {
            type_id: GerbilSchemeTypeId::new(GERBIL_POO_LOOP_PROGRAM_COMPILER_TYPE_ID),
            schema_id: Some(GerbilSchemeSchemaId::new(
                GERBIL_POO_LOOP_PROGRAM_COMPILER_SCHEMA_ID,
            )),
            fields: vec![
                required_policy_pack_chain_field("kind", "string", None),
                required_policy_pack_chain_field("profile-id", "string", None),
                required_policy_pack_chain_field("compiler-owner", "string", None),
                required_policy_pack_chain_field("resolved-policy-pack", "object", None),
                required_policy_pack_chain_field("loop-program", "object", None),
                required_policy_pack_chain_field("scheme-boundary", "string", None),
                required_policy_pack_chain_field("serialization-boundary", "string", None),
            ],
        }],
    }
}

/// Scheme type manifest for config-interface loop policy projection modules.
pub fn gerbil_loop_policy_projection_module_type_manifest() -> GerbilSchemeTypeManifest {
    GerbilSchemeTypeManifest {
        schema_id: GerbilSchemeSchemaId::new("marlin.scheme-types.manifest.v1"),
        types: vec![GerbilSchemeTypeSpec {
            type_id: GerbilSchemeTypeId::new(GERBIL_LOOP_POLICY_PROJECTION_MODULE_TYPE_ID),
            schema_id: Some(GerbilSchemeSchemaId::new(
                GERBIL_LOOP_POLICY_PROJECTION_MODULE_SCHEMA_ID,
            )),
            fields: vec![
                required_policy_pack_chain_field("kind", "string", None),
                required_policy_pack_chain_field("module-id", "string", None),
                required_policy_pack_chain_field("profile-id", "string", None),
                required_policy_pack_chain_field("owner", "string", None),
                required_policy_pack_chain_field("source-module", "string", None),
                required_policy_pack_chain_field("poo-flow-module", "string", None),
                required_policy_pack_chain_field(
                    "poo-flow-capability-lanes",
                    "array",
                    Some("string"),
                ),
                required_policy_pack_chain_field("vertical-case-id", "any", None),
                required_policy_pack_chain_field(
                    "vertical-capability-tags",
                    "array",
                    Some("string"),
                ),
                required_policy_pack_chain_field("vertical-mainline?", "boolean", None),
                required_policy_pack_chain_field("rust-type", "string", None),
                required_policy_pack_chain_field("scheme-boundary", "string", None),
                required_policy_pack_chain_field("serialization-boundary", "string", None),
                required_policy_pack_chain_field("compiler-receipt", "object", None),
            ],
        }],
    }
}

/// Package manifest for Gerbil-resolved loop policy packs.
pub fn gerbil_resolved_loop_policy_pack_package_manifest() -> GerbilSchemePackageManifest {
    GerbilSchemePackageManifest::new(
        GerbilSchemePackageId::new(GERBIL_RESOLVED_LOOP_POLICY_PACK_PACKAGE_ID),
        gerbil_resolved_loop_policy_pack_type_manifest(),
    )
    .with_projection_contracts([gerbil_resolved_loop_policy_pack_contract()])
}

/// Package manifest for POO compiler receipts.
pub fn gerbil_poo_loop_program_compiler_package_manifest() -> GerbilSchemePackageManifest {
    GerbilSchemePackageManifest::new(
        GerbilSchemePackageId::new(GERBIL_POO_LOOP_PROGRAM_COMPILER_PACKAGE_ID),
        gerbil_poo_loop_program_compiler_type_manifest(),
    )
    .with_projection_contracts([gerbil_poo_loop_program_compiler_contract()])
}

/// Package manifest for config-interface loop policy projection modules.
pub fn gerbil_loop_policy_projection_module_package_manifest() -> GerbilSchemePackageManifest {
    GerbilSchemePackageManifest::new(
        GerbilSchemePackageId::new(GERBIL_LOOP_POLICY_PROJECTION_MODULE_PACKAGE_ID),
        gerbil_loop_policy_projection_module_type_manifest(),
    )
    .with_projection_contracts([gerbil_loop_policy_projection_module_contract()])
}

/// Decode a Gerbil policy-pack projection-chain receipt into Rust.
pub fn decode_gerbil_policy_pack_projection_chain_receipt(
    registry: &GerbilSchemeTypeRegistry,
    typed_value: &GerbilSchemeTypedValue,
) -> Result<GerbilPolicyPackProjectionChainReceipt, GerbilSchemeTypeDecodeError> {
    let projection: GerbilPolicyPackProjectionChainReceipt =
        registry.decode_projection(typed_value)?;
    projection.ensure_current_schema()?;
    Ok(projection)
}

/// Decode a Gerbil-resolved loop policy pack into Rust's hot/audit IR.
pub fn decode_gerbil_resolved_loop_policy_pack(
    registry: &GerbilSchemeTypeRegistry,
    typed_value: &GerbilSchemeTypedValue,
) -> Result<ResolvedLoopPolicyPack, GerbilSchemeTypeDecodeError> {
    let projection: ResolvedLoopPolicyPack = registry.decode_projection(typed_value)?;
    if projection.schema_version == RESOLVED_LOOP_POLICY_PACK_SCHEMA_VERSION {
        Ok(projection)
    } else {
        Err(GerbilSchemeTypeDecodeError::RustProjection {
            message: format!(
                "resolved loop policy pack schema version {} does not match {}",
                projection.schema_version, RESOLVED_LOOP_POLICY_PACK_SCHEMA_VERSION
            ),
        })
    }
}

/// Decode a Gerbil/POO compiler receipt into Rust's runtime `LoopProgram` boundary.
pub fn decode_gerbil_poo_loop_program_compiler_receipt(
    registry: &GerbilSchemeTypeRegistry,
    typed_value: &GerbilSchemeTypedValue,
) -> Result<GerbilPooLoopProgramCompilerReceipt, GerbilSchemeTypeDecodeError> {
    let projection: GerbilPooLoopProgramCompilerReceipt =
        registry.decode_projection(typed_value)?;
    projection.ensure_current_schema()?;
    Ok(projection)
}

/// Decode a config-interface loop policy projection module into Rust.
pub fn decode_gerbil_loop_policy_projection_module(
    registry: &GerbilSchemeTypeRegistry,
    typed_value: &GerbilSchemeTypedValue,
) -> Result<GerbilLoopPolicyProjectionModule, GerbilSchemeTypeDecodeError> {
    let projection: GerbilLoopPolicyProjectionModule = registry.decode_projection(typed_value)?;
    projection.ensure_current_schema()?;
    Ok(projection)
}

fn required_policy_pack_chain_field(
    name: &str,
    type_id: &str,
    element_type_id: Option<&str>,
) -> GerbilSchemeTypeFieldSpec {
    GerbilSchemeTypeFieldSpec {
        name: GerbilSchemeFieldName::new(name),
        type_id: GerbilSchemeTypeId::new(type_id),
        element_type_id: element_type_id.map(GerbilSchemeTypeId::new),
        required: true,
        description: None,
    }
}
