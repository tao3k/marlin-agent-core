//! Native ABI projection receipts for Gerbil-built Scheme typed values.

use serde::{Deserialize, Serialize, de::DeserializeOwned};

use super::{
    error::GerbilSchemeTypeDecodeError,
    ids::{GerbilSchemeNativeAbiId, GerbilSchemeNativeSymbol, GerbilSchemeSchemaId},
    package::GerbilSchemeNativeAbiReadinessPlan,
    projection::GerbilSchemeProjectionContract,
    registry::GerbilSchemeTypeRegistry,
    typed_value::GerbilSchemeTypedValue,
};

/// Native ABI projection call requested by Rust.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilSchemeNativeProjectionRequest {
    pub abi_id: GerbilSchemeNativeAbiId,
    pub abi_version: u32,
    pub symbol: GerbilSchemeNativeSymbol,
    pub contract: GerbilSchemeProjectionContract,
}

impl GerbilSchemeNativeProjectionRequest {
    pub fn new(
        abi_id: GerbilSchemeNativeAbiId,
        abi_version: u32,
        symbol: GerbilSchemeNativeSymbol,
        contract: GerbilSchemeProjectionContract,
    ) -> Self {
        Self {
            abi_id,
            abi_version,
            symbol,
            contract,
        }
    }
}

/// Native ABI projection status after Rust validates the returned typed value.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GerbilSchemeNativeProjectionStatus {
    Projected,
}

/// Receipt proving a native ABI symbol returned the requested typed projection.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilSchemeNativeProjectionReceipt {
    pub status: GerbilSchemeNativeProjectionStatus,
    pub abi_id: GerbilSchemeNativeAbiId,
    pub abi_version: u32,
    pub symbol: GerbilSchemeNativeSymbol,
    pub type_id: super::ids::GerbilSchemeTypeId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_id: Option<GerbilSchemeSchemaId>,
}

/// Validate a native ABI projection envelope before any Rust payload decode.
pub fn validate_gerbil_scheme_native_projection(
    readiness_plan: &GerbilSchemeNativeAbiReadinessPlan,
    request: &GerbilSchemeNativeProjectionRequest,
    typed_value: &GerbilSchemeTypedValue,
) -> Result<GerbilSchemeNativeProjectionReceipt, GerbilSchemeTypeDecodeError> {
    validate_projection_readiness(readiness_plan, request)?;
    typed_value.ensure_projection_contract(&request.contract)?;

    Ok(GerbilSchemeNativeProjectionReceipt {
        status: GerbilSchemeNativeProjectionStatus::Projected,
        abi_id: request.abi_id.clone(),
        abi_version: request.abi_version,
        symbol: request.symbol.clone(),
        type_id: typed_value.type_id().clone(),
        schema_id: typed_value.schema_id().cloned(),
    })
}

/// Validate and decode a native ABI projection into a Rust type.
pub fn decode_gerbil_scheme_native_projection<T>(
    registry: &GerbilSchemeTypeRegistry,
    readiness_plan: &GerbilSchemeNativeAbiReadinessPlan,
    request: &GerbilSchemeNativeProjectionRequest,
    typed_value: &GerbilSchemeTypedValue,
) -> Result<(GerbilSchemeNativeProjectionReceipt, T), GerbilSchemeTypeDecodeError>
where
    T: DeserializeOwned,
{
    let receipt = validate_gerbil_scheme_native_projection(readiness_plan, request, typed_value)?;
    let projection = registry.decode_typed_value_with_contract(typed_value, &request.contract)?;
    Ok((receipt, projection))
}

fn validate_projection_readiness(
    readiness_plan: &GerbilSchemeNativeAbiReadinessPlan,
    request: &GerbilSchemeNativeProjectionRequest,
) -> Result<(), GerbilSchemeTypeDecodeError> {
    if readiness_plan.abi_id != request.abi_id {
        return Err(GerbilSchemeTypeDecodeError::NativeAbiMismatch {
            expected: request.abi_id.clone(),
            actual: readiness_plan.abi_id.clone(),
        });
    }

    if readiness_plan.version != request.abi_version {
        return Err(GerbilSchemeTypeDecodeError::NativeAbiVersionMismatch {
            abi_id: request.abi_id.clone(),
            expected: request.abi_version,
            actual: readiness_plan.version,
        });
    }

    if !readiness_plan
        .exported_symbols
        .iter()
        .any(|symbol| symbol == &request.symbol)
    {
        return Err(GerbilSchemeTypeDecodeError::MissingNativeSymbol {
            symbol: request.symbol.clone(),
        });
    }

    Ok(())
}
