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
    projection::{GerbilSchemeProjectionContract, GerbilSchemeTypedProjection},
    typed_value::GerbilSchemeTypedValue,
    validation,
};

/// Prevalidated Scheme type registry for hot-path envelope validation and projection.
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

    /// Validate a dynamic Scheme value, including manifest-declared required fields.
    pub fn validate_dynamic_typed_value(
        &self,
        typed_value: &GerbilSchemeTypedValue,
    ) -> Result<GerbilSchemeTypedValueValidationReceipt, GerbilSchemeTypeDecodeError> {
        let receipt = self.validate_typed_value(typed_value)?;
        let spec = self
            .type_spec(typed_value.type_id())
            .expect("typed value type was validated before payload validation");
        validation::validate_typed_value_payload(
            |type_id| self.type_spec(type_id),
            spec,
            typed_value,
        )?;
        Ok(receipt)
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

    /// Validate and decode a dynamic Scheme value without requiring a Rust projection type.
    pub fn decode_dynamic_typed_value(
        &self,
        typed_value: &GerbilSchemeTypedValue,
    ) -> Result<Value, GerbilSchemeTypeDecodeError> {
        self.validate_dynamic_typed_value(typed_value)?;
        Ok(typed_value.value().clone())
    }

    /// Validate and decode a typed Scheme value into a Rust projection with a runtime contract.
    pub fn decode_typed_value_with_contract<T>(
        &self,
        typed_value: &GerbilSchemeTypedValue,
        contract: &GerbilSchemeProjectionContract,
    ) -> Result<T, GerbilSchemeTypeDecodeError>
    where
        T: DeserializeOwned,
    {
        self.validate_typed_value(typed_value)?;
        typed_value.decode_value_with_contract(contract)
    }

    /// Validate and decode a typed Scheme value into a Rust projection with a static type contract.
    pub fn decode_projection<T>(
        &self,
        typed_value: &GerbilSchemeTypedValue,
    ) -> Result<T, GerbilSchemeTypeDecodeError>
    where
        T: GerbilSchemeTypedProjection,
    {
        let contract = T::scheme_projection_contract();
        self.decode_typed_value_with_contract(typed_value, &contract)
    }
}
