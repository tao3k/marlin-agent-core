//! Typed Rust projections for Scheme graph policy objects.

use crate::policy_pack_projection::{
    GerbilLoopPolicyProjectionModule, decode_gerbil_loop_policy_projection_module,
    gerbil_loop_policy_projection_module_contract,
    gerbil_loop_policy_projection_module_type_manifest, gerbil_resolved_loop_policy_pack_contract,
    gerbil_resolved_loop_policy_pack_type_manifest,
};
use crate::scheme_types::{
    GerbilSchemeAbiProjectionReceipt, GerbilSchemeAbiProjectionRequest, GerbilSchemeAbiSymbol,
    GerbilSchemeCAbiContract, GerbilSchemeCAbiId, GerbilSchemeCAbiReadinessPlan,
    GerbilSchemeFieldName, GerbilSchemePackageId, GerbilSchemePackageManifest,
    GerbilSchemeProjectionContract, GerbilSchemeSchemaId, GerbilSchemeTypeDecodeError,
    GerbilSchemeTypeFieldSpec, GerbilSchemeTypeId, GerbilSchemeTypeManifest,
    GerbilSchemeTypeRegistry, GerbilSchemeTypeSpec, GerbilSchemeTypedProjection,
    GerbilSchemeTypedValue, decode_gerbil_scheme_abi_projection,
    validate_gerbil_scheme_abi_projection,
};
use marlin_agent_protocol::{RESOLVED_LOOP_POLICY_PACK_SCHEMA_VERSION, ResolvedLoopPolicyPack};
use serde::{Deserialize, Serialize};

/// Native ABI id for Deck runtime typed projections.
pub const GERBIL_DECK_ROUTE_ABI_PROJECTION_ABI_ID: &str = "marlin.deck-runtime.native-projection";
/// Package id for the Deck runtime typed projection ABI manifest.
pub const GERBIL_DECK_ROUTE_ABI_PROJECTION_PACKAGE_ID: &str =
    "marlin.deck-runtime.native-projection";
/// Native ABI version for Deck runtime typed projections.
pub const GERBIL_DECK_ROUTE_ABI_PROJECTION_ABI_VERSION: u32 = 1;
/// Native ABI symbol that projects a Gerbil POO policy object into a typed Rust envelope.
pub const GERBIL_DECK_RUNTIME_PROJECT_POO_POLICY_SYMBOL: &str =
    "marlin_deck_runtime_project_poo_policy";
/// Native ABI symbol that projects a Gerbil-resolved loop policy pack into Rust IR.
pub const GERBIL_DECK_RUNTIME_PROJECT_RESOLVED_LOOP_POLICY_PACK_SYMBOL: &str =
    "marlin_deck_runtime_project_resolved_loop_policy_pack";
/// Native ABI symbol that projects a Gerbil config-interface module into Rust IR.
pub const GERBIL_DECK_RUNTIME_PROJECT_LOOP_POLICY_PROJECTION_MODULE_SYMBOL: &str =
    "marlin_deck_runtime_project_loop_policy_projection_module";
/// Type id returned by the Gerbil POO policy projection.
pub const GERBIL_DECK_RUNTIME_POO_POLICY_PROJECTION_TYPE_ID: &str =
    "marlin.deck-runtime.poo-policy-projection";
/// Schema id returned by the Gerbil POO policy projection.
pub const GERBIL_DECK_RUNTIME_POO_POLICY_PROJECTION_SCHEMA_ID: &str =
    "marlin.deck-runtime.poo-policy-projection.v1";
/// Rust projection for a Gerbil-built POO policy object.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilDeckRuntimePooPolicyProjection {
    pub schema_id: String,
    pub policy_id: String,
    pub object_system: String,
    pub package: String,
    pub module: String,
    pub action: String,
}

impl GerbilSchemeTypedProjection for GerbilDeckRuntimePooPolicyProjection {
    fn scheme_projection_contract() -> GerbilSchemeProjectionContract {
        gerbil_deck_runtime_poo_policy_projection_contract()
    }
}

/// Contract expected for the Gerbil POO policy projection.
pub fn gerbil_deck_runtime_poo_policy_projection_contract() -> GerbilSchemeProjectionContract {
    GerbilSchemeProjectionContract::new(GerbilSchemeTypeId::new(
        GERBIL_DECK_RUNTIME_POO_POLICY_PROJECTION_TYPE_ID,
    ))
    .with_schema_id(GerbilSchemeSchemaId::new(
        GERBIL_DECK_RUNTIME_POO_POLICY_PROJECTION_SCHEMA_ID,
    ))
}

/// Scheme type manifest for Deck runtime POO policy projections.
pub fn gerbil_deck_runtime_poo_policy_projection_type_manifest() -> GerbilSchemeTypeManifest {
    GerbilSchemeTypeManifest {
        schema_id: GerbilSchemeSchemaId::new("marlin.scheme-types.manifest.v1"),
        types: vec![GerbilSchemeTypeSpec {
            type_id: GerbilSchemeTypeId::new(GERBIL_DECK_RUNTIME_POO_POLICY_PROJECTION_TYPE_ID),
            schema_id: Some(GerbilSchemeSchemaId::new(
                GERBIL_DECK_RUNTIME_POO_POLICY_PROJECTION_SCHEMA_ID,
            )),
            fields: [
                "schema_id",
                "policy_id",
                "object_system",
                "package",
                "module",
                "action",
            ]
            .into_iter()
            .map(required_projection_string_field)
            .collect(),
        }],
    }
}

/// Scheme type manifest for all Deck runtime native projections.
pub fn gerbil_deck_route_abi_projection_type_manifest() -> GerbilSchemeTypeManifest {
    let mut manifest = gerbil_deck_runtime_poo_policy_projection_type_manifest();
    manifest
        .types
        .extend(gerbil_resolved_loop_policy_pack_type_manifest().types);
    manifest
        .types
        .extend(gerbil_loop_policy_projection_module_type_manifest().types);
    manifest
}

/// Native ABI projection request for the Gerbil POO policy projection.
pub fn gerbil_deck_runtime_poo_policy_projection_request() -> GerbilSchemeAbiProjectionRequest {
    GerbilSchemeAbiProjectionRequest::new(
        GerbilSchemeCAbiId::new(GERBIL_DECK_ROUTE_ABI_PROJECTION_ABI_ID),
        GERBIL_DECK_ROUTE_ABI_PROJECTION_ABI_VERSION,
        GerbilSchemeAbiSymbol::new(GERBIL_DECK_RUNTIME_PROJECT_POO_POLICY_SYMBOL),
        gerbil_deck_runtime_poo_policy_projection_contract(),
    )
}

/// Native ABI projection request for a Gerbil-resolved loop policy pack.
pub fn gerbil_deck_runtime_resolved_loop_policy_pack_projection_request()
-> GerbilSchemeAbiProjectionRequest {
    GerbilSchemeAbiProjectionRequest::new(
        GerbilSchemeCAbiId::new(GERBIL_DECK_ROUTE_ABI_PROJECTION_ABI_ID),
        GERBIL_DECK_ROUTE_ABI_PROJECTION_ABI_VERSION,
        GerbilSchemeAbiSymbol::new(GERBIL_DECK_RUNTIME_PROJECT_RESOLVED_LOOP_POLICY_PACK_SYMBOL),
        gerbil_resolved_loop_policy_pack_contract(),
    )
}

/// Native ABI projection request for a Gerbil config-interface loop policy module.
pub fn gerbil_deck_runtime_loop_policy_projection_module_request()
-> GerbilSchemeAbiProjectionRequest {
    GerbilSchemeAbiProjectionRequest::new(
        GerbilSchemeCAbiId::new(GERBIL_DECK_ROUTE_ABI_PROJECTION_ABI_ID),
        GERBIL_DECK_ROUTE_ABI_PROJECTION_ABI_VERSION,
        GerbilSchemeAbiSymbol::new(
            GERBIL_DECK_RUNTIME_PROJECT_LOOP_POLICY_PROJECTION_MODULE_SYMBOL,
        ),
        gerbil_loop_policy_projection_module_contract(),
    )
}

/// Native ABI contract declared by the Deck runtime projection package.
pub fn gerbil_deck_route_abi_projection_abi_contract() -> GerbilSchemeCAbiContract {
    GerbilSchemeCAbiContract::new(
        GerbilSchemeCAbiId::new(GERBIL_DECK_ROUTE_ABI_PROJECTION_ABI_ID),
        GERBIL_DECK_ROUTE_ABI_PROJECTION_ABI_VERSION,
    )
    .with_exported_symbols([
        GerbilSchemeAbiSymbol::new(GERBIL_DECK_RUNTIME_PROJECT_POO_POLICY_SYMBOL),
        GerbilSchemeAbiSymbol::new(GERBIL_DECK_RUNTIME_PROJECT_RESOLVED_LOOP_POLICY_PACK_SYMBOL),
        GerbilSchemeAbiSymbol::new(
            GERBIL_DECK_RUNTIME_PROJECT_LOOP_POLICY_PROJECTION_MODULE_SYMBOL,
        ),
    ])
}

/// Readiness plan expected before calling the Deck runtime native projection ABI.
pub fn gerbil_deck_route_abi_projection_readiness_plan() -> GerbilSchemeCAbiReadinessPlan {
    GerbilSchemeCAbiReadinessPlan::new(
        GerbilSchemeCAbiId::new(GERBIL_DECK_ROUTE_ABI_PROJECTION_ABI_ID),
        GERBIL_DECK_ROUTE_ABI_PROJECTION_ABI_VERSION,
    )
    .with_exported_symbols([
        GerbilSchemeAbiSymbol::new(GERBIL_DECK_RUNTIME_PROJECT_POO_POLICY_SYMBOL),
        GerbilSchemeAbiSymbol::new(GERBIL_DECK_RUNTIME_PROJECT_RESOLVED_LOOP_POLICY_PACK_SYMBOL),
        GerbilSchemeAbiSymbol::new(
            GERBIL_DECK_RUNTIME_PROJECT_LOOP_POLICY_PROJECTION_MODULE_SYMBOL,
        ),
    ])
}

/// Package manifest for the Deck runtime typed projection ABI.
pub fn gerbil_deck_route_abi_projection_package_manifest() -> GerbilSchemePackageManifest {
    GerbilSchemePackageManifest::new(
        GerbilSchemePackageId::new(GERBIL_DECK_ROUTE_ABI_PROJECTION_PACKAGE_ID),
        gerbil_deck_route_abi_projection_type_manifest(),
    )
    .with_projection_contracts([
        gerbil_deck_runtime_poo_policy_projection_contract(),
        gerbil_resolved_loop_policy_pack_contract(),
        gerbil_loop_policy_projection_module_contract(),
    ])
    .with_c_abi(gerbil_deck_route_abi_projection_abi_contract())
}

/// Decode a Gerbil POO policy projection returned by the native ABI.
pub fn decode_gerbil_deck_runtime_poo_policy_projection(
    registry: &GerbilSchemeTypeRegistry,
    typed_value: &GerbilSchemeTypedValue,
) -> Result<
    (
        GerbilSchemeAbiProjectionReceipt,
        GerbilDeckRuntimePooPolicyProjection,
    ),
    GerbilSchemeTypeDecodeError,
> {
    decode_gerbil_scheme_abi_projection(
        registry,
        &gerbil_deck_route_abi_projection_readiness_plan(),
        &gerbil_deck_runtime_poo_policy_projection_request(),
        typed_value,
    )
}

/// Decode a Gerbil-resolved loop policy pack returned by the native ABI.
pub fn decode_gerbil_deck_runtime_resolved_loop_policy_pack_projection(
    registry: &GerbilSchemeTypeRegistry,
    typed_value: &GerbilSchemeTypedValue,
) -> Result<(GerbilSchemeAbiProjectionReceipt, ResolvedLoopPolicyPack), GerbilSchemeTypeDecodeError>
{
    let (receipt, projection): (GerbilSchemeAbiProjectionReceipt, ResolvedLoopPolicyPack) =
        decode_gerbil_scheme_abi_projection(
            registry,
            &gerbil_deck_route_abi_projection_readiness_plan(),
            &gerbil_deck_runtime_resolved_loop_policy_pack_projection_request(),
            typed_value,
        )?;

    if projection.has_current_schema() {
        Ok((receipt, projection))
    } else {
        Err(GerbilSchemeTypeDecodeError::RustProjection {
            message: format!(
                "resolved loop policy pack schema version {} does not match {}",
                projection.schema_version, RESOLVED_LOOP_POLICY_PACK_SCHEMA_VERSION
            ),
        })
    }
}

/// Decode a Gerbil config-interface loop policy projection module returned by the native ABI.
pub fn decode_gerbil_deck_runtime_loop_policy_projection_module(
    registry: &GerbilSchemeTypeRegistry,
    typed_value: &GerbilSchemeTypedValue,
) -> Result<
    (
        GerbilSchemeAbiProjectionReceipt,
        GerbilLoopPolicyProjectionModule,
    ),
    GerbilSchemeTypeDecodeError,
> {
    let receipt = validate_gerbil_scheme_abi_projection(
        &gerbil_deck_route_abi_projection_readiness_plan(),
        &gerbil_deck_runtime_loop_policy_projection_module_request(),
        typed_value,
    )?;
    let projection = decode_gerbil_loop_policy_projection_module(registry, typed_value)?;
    Ok((receipt, projection))
}

fn required_projection_string_field(name: &str) -> GerbilSchemeTypeFieldSpec {
    GerbilSchemeTypeFieldSpec {
        name: GerbilSchemeFieldName::new(name),
        type_id: GerbilSchemeTypeId::new("string"),
        element_type_id: None,
        required: true,
        description: None,
    }
}
