// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Downstream Scheme package manifests consumed by Rust.

use marlin_agent_protocol::{GraphAbiSymbol, GraphCAbiRequirement};
use serde::{Deserialize, Serialize};

use super::{
    error::GerbilSchemeTypeDecodeError,
    ids::{GerbilSchemeAbiSymbol, GerbilSchemeCAbiId, GerbilSchemePackageId, GerbilSchemeSchemaId},
    manifest::{GerbilSchemeTypeManifest, GerbilSchemeTypeManifestValidationReceipt},
    projection::GerbilSchemeProjectionContract,
    validation::validate_gerbil_scheme_type_manifest,
};

/// Native ABI contract declared by a downstream Scheme package.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilSchemeCAbiContract {
    pub abi_id: GerbilSchemeCAbiId,
    pub version: u32,
    #[serde(default)]
    pub exported_symbols: Vec<GerbilSchemeAbiSymbol>,
}

impl GerbilSchemeCAbiContract {
    pub fn new(abi_id: GerbilSchemeCAbiId, version: u32) -> Self {
        Self {
            abi_id,
            version,
            exported_symbols: Vec::new(),
        }
    }

    pub fn with_exported_symbols(
        mut self,
        exported_symbols: impl IntoIterator<Item = GerbilSchemeAbiSymbol>,
    ) -> Self {
        self.exported_symbols = exported_symbols.into_iter().collect();
        self
    }

    pub fn graph_c_abi_requirement(&self) -> GraphCAbiRequirement {
        GraphCAbiRequirement::new(self.abi_id.as_str(), self.version).with_required_symbols(
            self.exported_symbols
                .iter()
                .map(|symbol| GraphAbiSymbol::new(symbol.as_str())),
        )
    }
}

/// Native ABI exports made available by a concrete build or link plan.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilSchemeCAbiReadinessPlan {
    pub abi_id: GerbilSchemeCAbiId,
    pub version: u32,
    #[serde(default)]
    pub exported_symbols: Vec<GerbilSchemeAbiSymbol>,
}

impl GerbilSchemeCAbiReadinessPlan {
    pub fn new(abi_id: GerbilSchemeCAbiId, version: u32) -> Self {
        Self {
            abi_id,
            version,
            exported_symbols: Vec::new(),
        }
    }

    pub fn with_exported_symbols(
        mut self,
        exported_symbols: impl IntoIterator<Item = GerbilSchemeAbiSymbol>,
    ) -> Self {
        self.exported_symbols = exported_symbols.into_iter().collect();
        self
    }
}

/// Manifest emitted by a pure Scheme downstream package.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GerbilSchemePackageManifest {
    pub schema_id: GerbilSchemeSchemaId,
    pub package_id: GerbilSchemePackageId,
    pub type_manifest: GerbilSchemeTypeManifest,
    #[serde(default)]
    pub projection_contracts: Vec<GerbilSchemeProjectionContract>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub c_abi: Option<GerbilSchemeCAbiContract>,
}

impl GerbilSchemePackageManifest {
    pub fn new(package_id: GerbilSchemePackageId, type_manifest: GerbilSchemeTypeManifest) -> Self {
        Self {
            schema_id: GerbilSchemeSchemaId::new("marlin.scheme-package.manifest.v1"),
            package_id,
            type_manifest,
            projection_contracts: Vec::new(),
            c_abi: None,
        }
    }

    pub fn with_projection_contracts(
        mut self,
        projection_contracts: impl IntoIterator<Item = GerbilSchemeProjectionContract>,
    ) -> Self {
        self.projection_contracts = projection_contracts.into_iter().collect();
        self
    }

    pub fn with_c_abi(mut self, c_abi: GerbilSchemeCAbiContract) -> Self {
        self.c_abi = Some(c_abi);
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

    pub c_abi_version: Option<u32>,

    pub abi_symbol_count: usize,
}

/// Native readiness receipt used before linking a downstream Scheme package.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilSchemePackageAbiReadinessReceipt {
    pub package_id: GerbilSchemePackageId,
    pub abi_id: GerbilSchemeCAbiId,
    pub abi_version: u32,
    pub required_symbol_count: usize,
    pub available_symbol_count: usize,
    pub matched_symbol_count: usize,
}

/// Decode a serialized downstream package manifest fixture.
pub fn decode_gerbil_scheme_package_manifest_fixture(
    fixture: &str,
) -> Result<GerbilSchemePackageManifest, GerbilSchemeTypeDecodeError> {
    serde_json::from_str(fixture).map_err(|error| GerbilSchemeTypeDecodeError::SerializedFixture {
        message: error.to_string(),
    })
}

/// Validate a downstream Scheme package manifest without inspecting payload shapes.
pub fn validate_gerbil_scheme_package_manifest(
    manifest: &GerbilSchemePackageManifest,
) -> Result<GerbilSchemePackageManifestValidationReceipt, GerbilSchemeTypeDecodeError> {
    let type_receipt = validate_gerbil_scheme_type_manifest(&manifest.type_manifest)?;
    validate_projection_contracts(manifest)?;
    let abi_symbol_count = validate_c_abi(manifest.c_abi.as_ref())?;

    Ok(package_validation_receipt(
        manifest,
        &type_receipt,
        abi_symbol_count,
    ))
}

/// Validate that a downstream Scheme package can bind to the available native ABI exports.
pub fn validate_gerbil_scheme_package_abi_readiness(
    manifest: &GerbilSchemePackageManifest,
    readiness_plan: &GerbilSchemeCAbiReadinessPlan,
) -> Result<GerbilSchemePackageAbiReadinessReceipt, GerbilSchemeTypeDecodeError> {
    validate_gerbil_scheme_package_manifest(manifest)?;

    let c_abi =
        manifest
            .c_abi
            .as_ref()
            .ok_or_else(|| GerbilSchemeTypeDecodeError::MissingCAbi {
                package_id: manifest.package_id.clone(),
            })?;

    validate_abi_readiness_identity(c_abi, readiness_plan)?;
    let matched_symbol_count = validate_abi_readiness_symbols(c_abi, readiness_plan)?;

    Ok(GerbilSchemePackageAbiReadinessReceipt {
        package_id: manifest.package_id.clone(),
        abi_id: c_abi.abi_id.clone(),
        abi_version: c_abi.version,
        required_symbol_count: c_abi.exported_symbols.len(),
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

fn validate_abi_readiness_identity(
    c_abi: &GerbilSchemeCAbiContract,
    readiness_plan: &GerbilSchemeCAbiReadinessPlan,
) -> Result<(), GerbilSchemeTypeDecodeError> {
    if c_abi.abi_id != readiness_plan.abi_id {
        return Err(GerbilSchemeTypeDecodeError::CAbiMismatch {
            expected: c_abi.abi_id.clone(),
            actual: readiness_plan.abi_id.clone(),
        });
    }

    if c_abi.version != readiness_plan.version {
        return Err(GerbilSchemeTypeDecodeError::CAbiVersionMismatch {
            abi_id: c_abi.abi_id.clone(),
            expected: c_abi.version,
            actual: readiness_plan.version,
        });
    }

    Ok(())
}

fn validate_abi_readiness_symbols(
    c_abi: &GerbilSchemeCAbiContract,
    readiness_plan: &GerbilSchemeCAbiReadinessPlan,
) -> Result<usize, GerbilSchemeTypeDecodeError> {
    let available_symbols = readiness_plan
        .exported_symbols
        .iter()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();

    for required_symbol in &c_abi.exported_symbols {
        if !available_symbols.contains(required_symbol) {
            return Err(GerbilSchemeTypeDecodeError::MissingAbiSymbol {
                symbol: required_symbol.clone(),
            });
        }
    }

    Ok(c_abi.exported_symbols.len())
}

fn validate_c_abi(
    c_abi: Option<&GerbilSchemeCAbiContract>,
) -> Result<usize, GerbilSchemeTypeDecodeError> {
    let Some(c_abi) = c_abi else {
        return Ok(0);
    };

    if c_abi.exported_symbols.is_empty() {
        return Err(GerbilSchemeTypeDecodeError::MissingAbiSymbols {
            abi_id: c_abi.abi_id.clone(),
        });
    }

    let mut symbols = std::collections::BTreeSet::new();
    for symbol in &c_abi.exported_symbols {
        if !symbols.insert(symbol.clone()) {
            return Err(GerbilSchemeTypeDecodeError::DuplicateAbiSymbol {
                symbol: symbol.clone(),
            });
        }
    }

    Ok(c_abi.exported_symbols.len())
}

fn package_validation_receipt(
    manifest: &GerbilSchemePackageManifest,
    type_receipt: &GerbilSchemeTypeManifestValidationReceipt,
    abi_symbol_count: usize,
) -> GerbilSchemePackageManifestValidationReceipt {
    GerbilSchemePackageManifestValidationReceipt {
        schema_id: manifest.schema_id.clone(),
        package_id: manifest.package_id.clone(),
        type_count: type_receipt.type_count,
        field_count: type_receipt.field_count,
        projection_contract_count: manifest.projection_contracts.len(),
        c_abi_version: manifest.c_abi.as_ref().map(|c_abi| c_abi.version),
        abi_symbol_count,
    }
}
