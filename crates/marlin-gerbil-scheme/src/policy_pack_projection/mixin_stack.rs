//! Typed Rust projection for Gerbil/POO mixin-stack compiler receipts.

use std::collections::BTreeSet;

use marlin_agent_protocol::{
    LoopPolicyEpoch, LoopPolicyMixinId, LoopPolicyRoleId, LoopPolicySlotId, LoopProgram,
    ResolvedLoopPolicyPack, SlotMergeAlgebra, SlotMergeReceipt, SlotMergeStatus,
};
use serde::{Deserialize, Serialize};

use crate::{
    GerbilSchemeFieldName, GerbilSchemePackageId, GerbilSchemePackageManifest,
    GerbilSchemeProjectionContract, GerbilSchemeSchemaId, GerbilSchemeTypeDecodeError,
    GerbilSchemeTypeFieldSpec, GerbilSchemeTypeId, GerbilSchemeTypeManifest,
    GerbilSchemeTypeRegistry, GerbilSchemeTypeSpec, GerbilSchemeTypedProjection,
    GerbilSchemeTypedValue, GerbilSchemeValue,
};

use super::surface::{
    GerbilPolicyPackReceiptKind, GerbilPooLoopProgramCompilerBoundary,
    GerbilPooLoopProgramCompilerOwner, GerbilPooLoopProgramCompilerReceipt,
    GerbilPooLoopProgramCompilerSerializationBoundary, GerbilPooLoopProgramProfileId,
};

/// Package id for POO mixin-stack compiler receipts.
pub const GERBIL_POLICY_MIXIN_STACK_COMPILER_PACKAGE_ID: &str =
    "marlin.config-interface.policy-pack.mixin-stack-compiler";

/// Type id for POO mixin-stack compiler receipts.
pub const GERBIL_POLICY_MIXIN_STACK_COMPILER_TYPE_ID: &str =
    "marlin.config-interface.policy-pack.mixin-stack-compiler-receipt";

/// Schema id carried by POO mixin-stack compiler receipts.
pub const GERBIL_POLICY_MIXIN_STACK_COMPILER_SCHEMA_ID: &str =
    "marlin.config-interface.policy-pack.mixin-stack-compiler-receipt.v1";

/// Schema id carried by POO mixin definition receipts.
pub const GERBIL_POLICY_MIXIN_DEFINITION_SCHEMA_ID: &str =
    "marlin.config-interface.policy-pack.mixin-definition.v1";

/// Schema id carried by POO slot-merge receipts.
pub const GERBIL_POLICY_SLOT_MERGE_RECEIPT_SCHEMA_ID: &str =
    "marlin.config-interface.policy-pack.slot-merge-receipt.v1";

/// One POO mixin definition emitted by the Scheme policy plane.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilPolicyMixinDefinition {
    pub kind: GerbilPolicyPackReceiptKind,
    #[serde(rename = "mixin-id")]
    pub mixin_id: LoopPolicyMixinId,
    pub role: LoopPolicyRoleId,
    #[serde(rename = "slot-ids")]
    pub slot_ids: Box<[LoopPolicySlotId]>,
    pub owner: String,
    #[serde(rename = "scheme-boundary")]
    pub scheme_boundary: GerbilPooLoopProgramCompilerBoundary,
    #[serde(rename = "serialization-boundary")]
    pub serialization_boundary: GerbilPooLoopProgramCompilerSerializationBoundary,
    #[serde(rename = "rust-handler-manufactured")]
    pub rust_handler_manufactured: bool,
}

/// Slot-merge receipt emitted by Scheme before Rust projection freezes the policy pack.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GerbilPolicySlotMergeReceipt {
    pub kind: GerbilPolicyPackReceiptKind,
    pub slot_id: LoopPolicySlotId,
    pub slot: String,
    pub merge: SlotMergeAlgebra,
    pub status: SlotMergeStatus,
    pub inputs: Box<[GerbilSchemeValue]>,
    pub result: GerbilSchemeValue,
    #[serde(rename = "conflict-reasons")]
    pub conflict_reasons: Box<[GerbilSchemeValue]>,
    pub owner: String,
    #[serde(rename = "scheme-boundary")]
    pub scheme_boundary: GerbilPooLoopProgramCompilerBoundary,
    #[serde(rename = "serialization-boundary")]
    pub serialization_boundary: GerbilPooLoopProgramCompilerSerializationBoundary,
    #[serde(rename = "rust-handler-manufactured")]
    pub rust_handler_manufactured: bool,
}

/// POO mixin-stack compiler receipt that proves profile/mixins/merge laws produced the program.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GerbilPolicyMixinStackCompilerReceipt {
    pub kind: GerbilPolicyPackReceiptKind,
    pub owner: String,
    #[serde(rename = "compiler-owner")]
    pub compiler_owner: GerbilPooLoopProgramCompilerOwner,
    #[serde(rename = "profile-id")]
    pub profile_id: GerbilPooLoopProgramProfileId,
    #[serde(rename = "policy-epoch")]
    pub policy_epoch: LoopPolicyEpoch,
    #[serde(rename = "policy-mixins")]
    pub policy_mixins: Box<[LoopPolicyMixinId]>,
    #[serde(rename = "mixin-definitions")]
    pub mixin_definitions: Box<[GerbilPolicyMixinDefinition]>,
    #[serde(rename = "mixin-count")]
    pub mixin_count: usize,
    pub linearization: Box<[LoopPolicyRoleId]>,
    #[serde(rename = "linearization-owner")]
    pub linearization_owner: String,
    #[serde(rename = "slot-merge-receipts")]
    pub slot_merge_receipts: Box<[GerbilPolicySlotMergeReceipt]>,
    #[serde(rename = "slot-merge-audit")]
    pub slot_merge_audit: Box<[SlotMergeReceipt]>,
    #[serde(rename = "slot-merge-laws")]
    pub slot_merge_laws: Box<[String]>,
    #[serde(rename = "slot-merge-owner")]
    pub slot_merge_owner: String,
    #[serde(rename = "profile-spec")]
    pub profile_spec: GerbilSchemeValue,
    #[serde(rename = "resolved-policy-pack")]
    pub resolved_policy_pack: ResolvedLoopPolicyPack,
    #[serde(rename = "loop-program")]
    pub loop_program: LoopProgram,
    #[serde(rename = "compiler-receipt")]
    pub compiler_receipt: GerbilPooLoopProgramCompilerReceipt,
    #[serde(rename = "scheme-boundary")]
    pub scheme_boundary: GerbilPooLoopProgramCompilerBoundary,
    #[serde(rename = "serialization-boundary")]
    pub serialization_boundary: GerbilPooLoopProgramCompilerSerializationBoundary,
    #[serde(rename = "rust-handler-manufactured")]
    pub rust_handler_manufactured: bool,
}

impl GerbilPolicyMixinDefinition {
    fn ensure_current_schema(&self) -> Result<(), GerbilSchemeTypeDecodeError> {
        if self.kind.as_str() != GERBIL_POLICY_MIXIN_DEFINITION_SCHEMA_ID {
            return Err(GerbilSchemeTypeDecodeError::RustProjection {
                message: format!(
                    "POO mixin definition kind {} does not match {}",
                    self.kind.as_str(),
                    GERBIL_POLICY_MIXIN_DEFINITION_SCHEMA_ID
                ),
            });
        }

        if self.mixin_id.as_str().trim().is_empty() || self.role.as_str().trim().is_empty() {
            return Err(GerbilSchemeTypeDecodeError::RustProjection {
                message: "POO mixin definition id and role must be non-empty".to_owned(),
            });
        }

        if self.slot_ids.is_empty() {
            return Err(GerbilSchemeTypeDecodeError::RustProjection {
                message: "POO mixin definition must carry slot ids".to_owned(),
            });
        }

        ensure_native_boundaries(
            self.scheme_boundary,
            self.serialization_boundary,
            self.rust_handler_manufactured,
            "POO mixin definition",
        )
    }
}

impl GerbilPolicySlotMergeReceipt {
    fn ensure_current_schema(&self) -> Result<(), GerbilSchemeTypeDecodeError> {
        if self.kind.as_str() != GERBIL_POLICY_SLOT_MERGE_RECEIPT_SCHEMA_ID {
            return Err(GerbilSchemeTypeDecodeError::RustProjection {
                message: format!(
                    "POO slot-merge receipt kind {} does not match {}",
                    self.kind.as_str(),
                    GERBIL_POLICY_SLOT_MERGE_RECEIPT_SCHEMA_ID
                ),
            });
        }

        if self.slot.trim().is_empty() {
            return Err(GerbilSchemeTypeDecodeError::RustProjection {
                message: "POO slot-merge receipt slot must be non-empty".to_owned(),
            });
        }

        ensure_native_boundaries(
            self.scheme_boundary,
            self.serialization_boundary,
            self.rust_handler_manufactured,
            "POO slot-merge receipt",
        )
    }

    fn law(&self) -> String {
        format!("{}={}", self.slot, slot_merge_algebra_id(&self.merge))
    }
}

impl GerbilPolicyMixinStackCompilerReceipt {
    /// Returns whether the receipt kind matches the current Rust projection.
    pub fn has_current_schema(&self) -> bool {
        self.kind.as_str() == GERBIL_POLICY_MIXIN_STACK_COMPILER_SCHEMA_ID
    }

    fn ensure_current_schema(&self) -> Result<(), GerbilSchemeTypeDecodeError> {
        if !self.has_current_schema() {
            return Err(GerbilSchemeTypeDecodeError::RustProjection {
                message: format!(
                    "POO mixin-stack compiler kind {} does not match {}",
                    self.kind.as_str(),
                    GERBIL_POLICY_MIXIN_STACK_COMPILER_SCHEMA_ID
                ),
            });
        }

        if self.compiler_owner != GerbilPooLoopProgramCompilerOwner::GerbilPooFlow {
            return Err(GerbilSchemeTypeDecodeError::RustProjection {
                message: "POO mixin-stack compiler owner must be gerbil-poo-flow".to_owned(),
            });
        }

        ensure_native_boundaries(
            self.scheme_boundary,
            self.serialization_boundary,
            self.rust_handler_manufactured,
            "POO mixin-stack compiler",
        )?;

        self.compiler_receipt.ensure_current_schema()?;
        self.ensure_compiler_payload_matches()?;
        self.ensure_mixin_stack_shape()?;
        self.ensure_slot_merge_shape()
    }

    fn ensure_compiler_payload_matches(&self) -> Result<(), GerbilSchemeTypeDecodeError> {
        if self.profile_id != self.compiler_receipt.profile_id {
            return Err(GerbilSchemeTypeDecodeError::RustProjection {
                message: format!(
                    "POO mixin-stack profile {} does not match compiler receipt profile {}",
                    self.profile_id.as_str(),
                    self.compiler_receipt.profile_id.as_str()
                ),
            });
        }

        if self.policy_epoch != self.compiler_receipt.loop_program.policy_epoch {
            return Err(GerbilSchemeTypeDecodeError::RustProjection {
                message: format!(
                    "POO mixin-stack epoch {} does not match loop program epoch {}",
                    self.policy_epoch.get(),
                    self.compiler_receipt.loop_program.policy_epoch.get()
                ),
            });
        }

        if self.resolved_policy_pack != self.compiler_receipt.resolved_policy_pack {
            return Err(GerbilSchemeTypeDecodeError::RustProjection {
                message: "POO mixin-stack resolved policy pack does not match compiler receipt"
                    .to_owned(),
            });
        }

        if self.loop_program != self.compiler_receipt.loop_program {
            return Err(GerbilSchemeTypeDecodeError::RustProjection {
                message: "POO mixin-stack loop program does not match compiler receipt".to_owned(),
            });
        }

        Ok(())
    }

    fn ensure_mixin_stack_shape(&self) -> Result<(), GerbilSchemeTypeDecodeError> {
        if self.mixin_count != self.mixin_definitions.len() {
            return Err(GerbilSchemeTypeDecodeError::RustProjection {
                message: format!(
                    "POO mixin-stack mixin count {} does not match definition count {}",
                    self.mixin_count,
                    self.mixin_definitions.len()
                ),
            });
        }

        if self.policy_mixins.len() != self.mixin_definitions.len() {
            return Err(GerbilSchemeTypeDecodeError::RustProjection {
                message: format!(
                    "POO mixin-stack policy mixins {} do not match definition count {}",
                    self.policy_mixins.len(),
                    self.mixin_definitions.len()
                ),
            });
        }

        if self.linearization.is_empty() {
            return Err(GerbilSchemeTypeDecodeError::RustProjection {
                message: "POO mixin-stack compiler must carry C3/C4 linearization".to_owned(),
            });
        }

        if self.linearization_owner != "poo-flow.c3-c4" {
            return Err(GerbilSchemeTypeDecodeError::RustProjection {
                message: format!(
                    "POO mixin-stack linearization owner {} does not match poo-flow.c3-c4",
                    self.linearization_owner
                ),
            });
        }

        let policy_mixins = self.policy_mixins.iter().cloned().collect::<BTreeSet<_>>();
        for mixin_definition in &self.mixin_definitions {
            mixin_definition.ensure_current_schema()?;
            if !policy_mixins.contains(&mixin_definition.mixin_id) {
                return Err(GerbilSchemeTypeDecodeError::RustProjection {
                    message: format!(
                        "POO mixin-stack definition {} is not present in policy mixins",
                        mixin_definition.mixin_id.as_str()
                    ),
                });
            }
        }

        Ok(())
    }

    fn ensure_slot_merge_shape(&self) -> Result<(), GerbilSchemeTypeDecodeError> {
        if self.slot_merge_owner != "poo-flow.slot-merge-algebra" {
            return Err(GerbilSchemeTypeDecodeError::RustProjection {
                message: format!(
                    "POO mixin-stack slot merge owner {} does not match poo-flow.slot-merge-algebra",
                    self.slot_merge_owner
                ),
            });
        }

        if self.slot_merge_receipts.is_empty() {
            return Err(GerbilSchemeTypeDecodeError::RustProjection {
                message: "POO mixin-stack compiler must carry slot merge receipts".to_owned(),
            });
        }

        if self.slot_merge_laws.len() != self.slot_merge_receipts.len() {
            return Err(GerbilSchemeTypeDecodeError::RustProjection {
                message: format!(
                    "POO mixin-stack slot merge laws {} do not match receipts {}",
                    self.slot_merge_laws.len(),
                    self.slot_merge_receipts.len()
                ),
            });
        }

        if self.slot_merge_audit.len() != self.slot_merge_receipts.len() {
            return Err(GerbilSchemeTypeDecodeError::RustProjection {
                message: format!(
                    "POO mixin-stack slot merge audit {} does not match receipts {}",
                    self.slot_merge_audit.len(),
                    self.slot_merge_receipts.len()
                ),
            });
        }

        if self.resolved_policy_pack.audit.merge_receipts.len() != self.slot_merge_receipts.len() {
            return Err(GerbilSchemeTypeDecodeError::RustProjection {
                message: format!(
                    "POO mixin-stack resolved pack merge receipts {} do not match stack receipts {}",
                    self.resolved_policy_pack.audit.merge_receipts.len(),
                    self.slot_merge_receipts.len()
                ),
            });
        }

        for (receipt, law) in self
            .slot_merge_receipts
            .iter()
            .zip(self.slot_merge_laws.iter())
        {
            receipt.ensure_current_schema()?;
            let expected_law = receipt.law();
            if law != &expected_law {
                return Err(GerbilSchemeTypeDecodeError::RustProjection {
                    message: format!(
                        "POO mixin-stack slot merge law {law} does not match {expected_law}",
                    ),
                });
            }
        }

        Ok(())
    }
}

impl GerbilSchemeTypedProjection for GerbilPolicyMixinStackCompilerReceipt {
    fn scheme_projection_contract() -> GerbilSchemeProjectionContract {
        gerbil_policy_mixin_stack_compiler_contract()
    }
}

/// Contract expected for a POO mixin-stack compiler receipt.
pub fn gerbil_policy_mixin_stack_compiler_contract() -> GerbilSchemeProjectionContract {
    GerbilSchemeProjectionContract::new(GerbilSchemeTypeId::new(
        GERBIL_POLICY_MIXIN_STACK_COMPILER_TYPE_ID,
    ))
    .with_schema_id(GerbilSchemeSchemaId::new(
        GERBIL_POLICY_MIXIN_STACK_COMPILER_SCHEMA_ID,
    ))
}

/// Scheme type manifest for POO mixin-stack compiler receipts.
pub fn gerbil_policy_mixin_stack_compiler_type_manifest() -> GerbilSchemeTypeManifest {
    GerbilSchemeTypeManifest {
        schema_id: GerbilSchemeSchemaId::new("marlin.scheme-types.manifest.v1"),
        types: vec![GerbilSchemeTypeSpec {
            type_id: GerbilSchemeTypeId::new(GERBIL_POLICY_MIXIN_STACK_COMPILER_TYPE_ID),
            schema_id: Some(GerbilSchemeSchemaId::new(
                GERBIL_POLICY_MIXIN_STACK_COMPILER_SCHEMA_ID,
            )),
            fields: vec![
                required_policy_mixin_stack_field("kind", "string", None),
                required_policy_mixin_stack_field("owner", "string", None),
                required_policy_mixin_stack_field("compiler-owner", "string", None),
                required_policy_mixin_stack_field("profile-id", "string", None),
                required_policy_mixin_stack_field("policy-epoch", "integer", None),
                required_policy_mixin_stack_field("policy-mixins", "array", Some("string")),
                required_policy_mixin_stack_field("mixin-definitions", "array", Some("object")),
                required_policy_mixin_stack_field("mixin-count", "integer", None),
                required_policy_mixin_stack_field("linearization", "array", Some("string")),
                required_policy_mixin_stack_field("linearization-owner", "string", None),
                required_policy_mixin_stack_field("slot-merge-receipts", "array", Some("object")),
                required_policy_mixin_stack_field("slot-merge-audit", "array", Some("object")),
                required_policy_mixin_stack_field("slot-merge-laws", "array", Some("string")),
                required_policy_mixin_stack_field("slot-merge-owner", "string", None),
                required_policy_mixin_stack_field("profile-spec", "object", None),
                required_policy_mixin_stack_field("resolved-policy-pack", "object", None),
                required_policy_mixin_stack_field("loop-program", "object", None),
                required_policy_mixin_stack_field("compiler-receipt", "object", None),
                required_policy_mixin_stack_field("scheme-boundary", "string", None),
                required_policy_mixin_stack_field("serialization-boundary", "string", None),
                required_policy_mixin_stack_field("rust-handler-manufactured", "boolean", None),
            ],
        }],
    }
}

/// Package manifest for POO mixin-stack compiler receipts.
pub fn gerbil_policy_mixin_stack_compiler_package_manifest() -> GerbilSchemePackageManifest {
    GerbilSchemePackageManifest::new(
        GerbilSchemePackageId::new(GERBIL_POLICY_MIXIN_STACK_COMPILER_PACKAGE_ID),
        gerbil_policy_mixin_stack_compiler_type_manifest(),
    )
    .with_projection_contracts([gerbil_policy_mixin_stack_compiler_contract()])
}

/// Decode a Gerbil/POO mixin-stack compiler receipt into Rust's typed projection boundary.
pub fn decode_gerbil_policy_mixin_stack_compiler_receipt(
    registry: &GerbilSchemeTypeRegistry,
    typed_value: &GerbilSchemeTypedValue,
) -> Result<GerbilPolicyMixinStackCompilerReceipt, GerbilSchemeTypeDecodeError> {
    let projection: GerbilPolicyMixinStackCompilerReceipt =
        registry.decode_projection(typed_value)?;
    projection.ensure_current_schema()?;
    Ok(projection)
}

fn ensure_native_boundaries(
    scheme_boundary: GerbilPooLoopProgramCompilerBoundary,
    serialization_boundary: GerbilPooLoopProgramCompilerSerializationBoundary,
    rust_handler_manufactured: bool,
    owner: &'static str,
) -> Result<(), GerbilSchemeTypeDecodeError> {
    if scheme_boundary != GerbilPooLoopProgramCompilerBoundary::SchemeTypesToRustTypes {
        return Err(GerbilSchemeTypeDecodeError::RustProjection {
            message: format!("{owner} must use scheme-types-to-rust-types boundary"),
        });
    }

    if serialization_boundary
        != GerbilPooLoopProgramCompilerSerializationBoundary::RustOwnedCliTraceCrossProcess
    {
        return Err(GerbilSchemeTypeDecodeError::RustProjection {
            message: format!("{owner} serialization boundary must be Rust-owned outside native"),
        });
    }

    if rust_handler_manufactured {
        return Err(GerbilSchemeTypeDecodeError::RustProjection {
            message: format!("{owner} must not manufacture Rust handlers"),
        });
    }

    Ok(())
}

fn required_policy_mixin_stack_field(
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

fn slot_merge_algebra_id(merge: &SlotMergeAlgebra) -> &'static str {
    match merge {
        SlotMergeAlgebra::Intersection => "intersection",
        SlotMergeAlgebra::Union => "union",
        SlotMergeAlgebra::Min => "min",
        SlotMergeAlgebra::OrderedAppend => "ordered_append",
        SlotMergeAlgebra::ConflictError => "conflict_error",
        SlotMergeAlgebra::Override => "override",
    }
}
