//! Typed Rust projection for Gerbil policy-pack receipt chains.

use marlin_agent_protocol::{RESOLVED_LOOP_POLICY_PACK_SCHEMA_VERSION, ResolvedLoopPolicyPack};
use serde::{Deserialize, Serialize};

use crate::{
    GerbilSchemeFieldName, GerbilSchemePackageId, GerbilSchemePackageManifest,
    GerbilSchemeProjectionContract, GerbilSchemeSchemaId, GerbilSchemeTypeDecodeError,
    GerbilSchemeTypeFieldSpec, GerbilSchemeTypeId, GerbilSchemeTypeManifest,
    GerbilSchemeTypeRegistry, GerbilSchemeTypeSpec, GerbilSchemeTypedProjection,
    GerbilSchemeTypedValue,
};

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

/// Package manifest for Gerbil-resolved loop policy packs.
pub fn gerbil_resolved_loop_policy_pack_package_manifest() -> GerbilSchemePackageManifest {
    GerbilSchemePackageManifest::new(
        GerbilSchemePackageId::new(GERBIL_RESOLVED_LOOP_POLICY_PACK_PACKAGE_ID),
        gerbil_resolved_loop_policy_pack_type_manifest(),
    )
    .with_projection_contracts([gerbil_resolved_loop_policy_pack_contract()])
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
