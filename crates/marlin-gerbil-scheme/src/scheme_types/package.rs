//! Downstream Scheme package manifests consumed by Rust.

use marlin_agent_protocol::{GraphNativeAbiRequirement, GraphNativeSymbol};
use serde::{Deserialize, Serialize};

use super::{
    error::GerbilSchemeTypeDecodeError,
    ids::{
        GerbilSchemeNativeAbiId, GerbilSchemeNativeSymbol, GerbilSchemePackageId,
        GerbilSchemeSchemaId,
    },
    manifest::{GerbilSchemeTypeManifest, GerbilSchemeTypeManifestValidationReceipt},
    projection::GerbilSchemeProjectionContract,
    validation::validate_gerbil_scheme_type_manifest,
};

/// Native ABI contract declared by a downstream Scheme package.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilSchemeNativeAbiContract {
    pub abi_id: GerbilSchemeNativeAbiId,
    pub version: u32,
    #[serde(default)]
    pub exported_symbols: Vec<GerbilSchemeNativeSymbol>,
}

impl GerbilSchemeNativeAbiContract {
    pub fn new(abi_id: GerbilSchemeNativeAbiId, version: u32) -> Self {
        Self {
            abi_id,
            version,
            exported_symbols: Vec::new(),
        }
    }

    pub fn with_exported_symbols(
        mut self,
        exported_symbols: impl IntoIterator<Item = GerbilSchemeNativeSymbol>,
    ) -> Self {
        self.exported_symbols = exported_symbols.into_iter().collect();
        self
    }

    pub fn graph_native_abi_requirement(&self) -> GraphNativeAbiRequirement {
        GraphNativeAbiRequirement::new(self.abi_id.as_str(), self.version).with_required_symbols(
            self.exported_symbols
                .iter()
                .map(|symbol| GraphNativeSymbol::new(symbol.as_str())),
        )
    }
}

/// Native ABI exports made available by a concrete build or link plan.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilSchemeNativeAbiReadinessPlan {
    pub abi_id: GerbilSchemeNativeAbiId,
    pub version: u32,
    #[serde(default)]
    pub exported_symbols: Vec<GerbilSchemeNativeSymbol>,
}

impl GerbilSchemeNativeAbiReadinessPlan {
    pub fn new(abi_id: GerbilSchemeNativeAbiId, version: u32) -> Self {
        Self {
            abi_id,
            version,
            exported_symbols: Vec::new(),
        }
    }

    pub fn with_exported_symbols(
        mut self,
        exported_symbols: impl IntoIterator<Item = GerbilSchemeNativeSymbol>,
    ) -> Self {
        self.exported_symbols = exported_symbols.into_iter().collect();
        self
    }
}

/// Manifest emitted by a pure Scheme downstream package.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilSchemePackageManifest {
    pub schema_id: GerbilSchemeSchemaId,
    pub package_id: GerbilSchemePackageId,
    pub type_manifest: GerbilSchemeTypeManifest,
    #[serde(default)]
    pub projection_contracts: Vec<GerbilSchemeProjectionContract>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_abi: Option<GerbilSchemeNativeAbiContract>,
}

impl GerbilSchemePackageManifest {
    pub fn new(package_id: GerbilSchemePackageId, type_manifest: GerbilSchemeTypeManifest) -> Self {
        Self {
            schema_id: GerbilSchemeSchemaId::new("marlin.scheme-package.manifest.v1"),
            package_id,
            type_manifest,
            projection_contracts: Vec::new(),
            native_abi: None,
        }
    }

    pub fn with_projection_contracts(
        mut self,
        projection_contracts: impl IntoIterator<Item = GerbilSchemeProjectionContract>,
    ) -> Self {
        self.projection_contracts = projection_contracts.into_iter().collect();
        self
    }

    pub fn with_native_abi(mut self, native_abi: GerbilSchemeNativeAbiContract) -> Self {
        self.native_abi = Some(native_abi);
        self
    }
}

/// Package validation receipt used by tests and quality gates.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilSchemePackageManifestValidationReceipt {
    pub schema_id: GerbilSchemeSchemaId,
    pub package_id: GerbilSchemePackageId,
    pub type_count: usize,
    pub field_count: usize,
    pub projection_contract_count: usize,
    pub native_abi_version: Option<u32>,
    pub native_symbol_count: usize,
}

/// Native readiness receipt used before linking a downstream Scheme package.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilSchemePackageNativeReadinessReceipt {
    pub package_id: GerbilSchemePackageId,
    pub abi_id: GerbilSchemeNativeAbiId,
    pub abi_version: u32,
    pub required_symbol_count: usize,
    pub available_symbol_count: usize,
    pub matched_symbol_count: usize,
}

/// Decode a Scheme-emitted downstream package manifest.
pub fn decode_gerbil_scheme_package_manifest(
    manifest_json: &str,
) -> Result<GerbilSchemePackageManifest, GerbilSchemeTypeDecodeError> {
    serde_json::from_str(manifest_json).map_err(|error| GerbilSchemeTypeDecodeError::Json {
        message: error.to_string(),
    })
}

/// Validate a downstream Scheme package manifest without inspecting payload shapes.
pub fn validate_gerbil_scheme_package_manifest(
    manifest: &GerbilSchemePackageManifest,
) -> Result<GerbilSchemePackageManifestValidationReceipt, GerbilSchemeTypeDecodeError> {
    let type_receipt = validate_gerbil_scheme_type_manifest(&manifest.type_manifest)?;
    validate_projection_contracts(manifest)?;
    let native_symbol_count = validate_native_abi(manifest.native_abi.as_ref())?;

    Ok(package_validation_receipt(
        manifest,
        &type_receipt,
        native_symbol_count,
    ))
}

/// Validate that a downstream Scheme package can bind to the available native ABI exports.
pub fn validate_gerbil_scheme_package_native_readiness(
    manifest: &GerbilSchemePackageManifest,
    readiness_plan: &GerbilSchemeNativeAbiReadinessPlan,
) -> Result<GerbilSchemePackageNativeReadinessReceipt, GerbilSchemeTypeDecodeError> {
    validate_gerbil_scheme_package_manifest(manifest)?;

    let native_abi = manifest.native_abi.as_ref().ok_or_else(|| {
        GerbilSchemeTypeDecodeError::MissingNativeAbi {
            package_id: manifest.package_id.clone(),
        }
    })?;

    validate_native_readiness_identity(native_abi, readiness_plan)?;
    let matched_symbol_count = validate_native_readiness_symbols(native_abi, readiness_plan)?;

    Ok(GerbilSchemePackageNativeReadinessReceipt {
        package_id: manifest.package_id.clone(),
        abi_id: native_abi.abi_id.clone(),
        abi_version: native_abi.version,
        required_symbol_count: native_abi.exported_symbols.len(),
        available_symbol_count: readiness_plan.exported_symbols.len(),
        matched_symbol_count,
    })
}

fn validate_projection_contracts(
    manifest: &GerbilSchemePackageManifest,
) -> Result<(), GerbilSchemeTypeDecodeError> {
    let mut contracts = std::collections::BTreeSet::new();

    for contract in &manifest.projection_contracts {
        if manifest
            .type_manifest
            .type_spec(contract.type_id())
            .is_none()
        {
            return Err(GerbilSchemeTypeDecodeError::UnknownType {
                type_id: contract.type_id().clone(),
            });
        }

        let key = (contract.type_id().clone(), contract.schema_id().cloned());
        if !contracts.insert(key) {
            return Err(GerbilSchemeTypeDecodeError::DuplicateProjectionContract {
                type_id: contract.type_id().clone(),
                schema_id: contract.schema_id().cloned(),
            });
        }
    }

    Ok(())
}

fn validate_native_readiness_identity(
    native_abi: &GerbilSchemeNativeAbiContract,
    readiness_plan: &GerbilSchemeNativeAbiReadinessPlan,
) -> Result<(), GerbilSchemeTypeDecodeError> {
    if native_abi.abi_id != readiness_plan.abi_id {
        return Err(GerbilSchemeTypeDecodeError::NativeAbiMismatch {
            expected: native_abi.abi_id.clone(),
            actual: readiness_plan.abi_id.clone(),
        });
    }

    if native_abi.version != readiness_plan.version {
        return Err(GerbilSchemeTypeDecodeError::NativeAbiVersionMismatch {
            abi_id: native_abi.abi_id.clone(),
            expected: native_abi.version,
            actual: readiness_plan.version,
        });
    }

    Ok(())
}

fn validate_native_readiness_symbols(
    native_abi: &GerbilSchemeNativeAbiContract,
    readiness_plan: &GerbilSchemeNativeAbiReadinessPlan,
) -> Result<usize, GerbilSchemeTypeDecodeError> {
    let available_symbols = readiness_plan
        .exported_symbols
        .iter()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();

    for required_symbol in &native_abi.exported_symbols {
        if !available_symbols.contains(required_symbol) {
            return Err(GerbilSchemeTypeDecodeError::MissingNativeSymbol {
                symbol: required_symbol.clone(),
            });
        }
    }

    Ok(native_abi.exported_symbols.len())
}

fn validate_native_abi(
    native_abi: Option<&GerbilSchemeNativeAbiContract>,
) -> Result<usize, GerbilSchemeTypeDecodeError> {
    let Some(native_abi) = native_abi else {
        return Ok(0);
    };

    if native_abi.exported_symbols.is_empty() {
        return Err(GerbilSchemeTypeDecodeError::MissingNativeSymbols {
            abi_id: native_abi.abi_id.clone(),
        });
    }

    let mut symbols = std::collections::BTreeSet::new();
    for symbol in &native_abi.exported_symbols {
        if !symbols.insert(symbol.clone()) {
            return Err(GerbilSchemeTypeDecodeError::DuplicateNativeSymbol {
                symbol: symbol.clone(),
            });
        }
    }

    Ok(native_abi.exported_symbols.len())
}

fn package_validation_receipt(
    manifest: &GerbilSchemePackageManifest,
    type_receipt: &GerbilSchemeTypeManifestValidationReceipt,
    native_symbol_count: usize,
) -> GerbilSchemePackageManifestValidationReceipt {
    GerbilSchemePackageManifestValidationReceipt {
        schema_id: manifest.schema_id.clone(),
        package_id: manifest.package_id.clone(),
        type_count: type_receipt.type_count,
        field_count: type_receipt.field_count,
        projection_contract_count: manifest.projection_contracts.len(),
        native_abi_version: manifest
            .native_abi
            .as_ref()
            .map(|native_abi| native_abi.version),
        native_symbol_count,
    }
}
