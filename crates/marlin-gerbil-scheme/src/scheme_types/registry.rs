//! Prevalidated `Scheme` type registry for hot-path projections.

use std::collections::BTreeMap;

use serde::de::DeserializeOwned;
use serde_json::Value;

use super::{
    error::GerbilSchemeTypeDecodeError,
    ids::GerbilSchemeTypeId,
    manifest::{
        GerbilSchemeTypeManifest, GerbilSchemeTypeManifestValidationReceipt, GerbilSchemeTypeSpec,
        GerbilSchemeTypedValueValidationReceipt,
    },
    typed_value::GerbilSchemeTypedValue,
    validation,
};

/// Prevalidated Scheme type registry for hot-path value validation and projection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilSchemeTypeRegistry {
    manifest: GerbilSchemeTypeManifest,
    type_index: BTreeMap<GerbilSchemeTypeId, usize>,
    validation_receipt: GerbilSchemeTypeManifestValidationReceipt,
}

impl GerbilSchemeTypeRegistry {
    /// Build a registry from a structurally validated `Gerbil` Scheme manifest.
    pub fn new(manifest: GerbilSchemeTypeManifest) -> Result<Self, GerbilSchemeTypeDecodeError> {
        let validation_receipt = validation::validate_gerbil_scheme_type_manifest(&manifest)?;
        let type_index = manifest
            .types
            .iter()
            .enumerate()
            .map(|(index, spec)| (spec.type_id.clone(), index))
            .collect::<BTreeMap<_, _>>();

        Ok(Self {
            manifest,
            type_index,
            validation_receipt,
        })
    }

    /// Return the manifest backing this registry.
    pub fn manifest(&self) -> &GerbilSchemeTypeManifest {
        &self.manifest
    }

    /// Return the validation receipt captured when the registry was built.
    pub fn validation_receipt(&self) -> &GerbilSchemeTypeManifestValidationReceipt {
        &self.validation_receipt
    }

    /// Resolve a Scheme type descriptor by stable type id.
    pub fn type_spec(&self, type_id: &GerbilSchemeTypeId) -> Option<&GerbilSchemeTypeSpec> {
        self.type_index
            .get(type_id)
            .map(|index| &self.manifest.types[*index])
    }

    /// Validate a typed Scheme value through this registry index.
    pub fn validate_typed_value(
        &self,
        typed_value: &GerbilSchemeTypedValue,
    ) -> Result<GerbilSchemeTypedValueValidationReceipt, GerbilSchemeTypeDecodeError> {
        validation::validate_typed_value_with_lookup(|type_id| self.type_spec(type_id), typed_value)
    }

    /// Validate a raw `JSON` payload as a named Scheme type through this registry.
    pub fn validate_value_as_type(
        &self,
        type_id: &GerbilSchemeTypeId,
        value: &Value,
    ) -> Result<GerbilSchemeTypedValueValidationReceipt, GerbilSchemeTypeDecodeError> {
        validation::validate_value_as_type_with_lookup(
            |resolved_type_id| self.type_spec(resolved_type_id),
            type_id,
            value,
        )
    }

    /// Validate and decode a typed Scheme value into a Rust projection.
    pub fn decode_typed_value<T>(
        &self,
        typed_value: &GerbilSchemeTypedValue,
    ) -> Result<T, GerbilSchemeTypeDecodeError>
    where
        T: DeserializeOwned,
    {
        self.validate_typed_value(typed_value)?;
        typed_value.decode_value()
    }
}
