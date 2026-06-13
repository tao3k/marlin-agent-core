//! Typed value envelopes emitted by `Scheme` runtimes.

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;

use super::{
    error::GerbilSchemeTypeDecodeError,
    ids::{GerbilSchemeSchemaId, GerbilSchemeTypeId},
    projection::{GerbilSchemeProjectionContract, GerbilSchemeTypedProjection},
};

/// Stable envelope for Scheme values whose concrete Rust projection may evolve downstream.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GerbilSchemeTypedValue {
    pub type_id: GerbilSchemeTypeId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_id: Option<GerbilSchemeSchemaId>,
    pub value: Value,
}

impl GerbilSchemeTypedValue {
    pub fn new(type_id: GerbilSchemeTypeId, value: Value) -> Self {
        Self {
            type_id,
            schema_id: None,
            value,
        }
    }

    pub fn with_schema_id(mut self, schema_id: GerbilSchemeSchemaId) -> Self {
        self.schema_id = Some(schema_id);
        self
    }

    pub fn type_id(&self) -> &GerbilSchemeTypeId {
        &self.type_id
    }

    pub fn schema_id(&self) -> Option<&GerbilSchemeSchemaId> {
        self.schema_id.as_ref()
    }

    pub fn value(&self) -> &Value {
        &self.value
    }

    pub fn ensure_type(
        &self,
        expected_type_id: &GerbilSchemeTypeId,
    ) -> Result<(), GerbilSchemeTypeDecodeError> {
        if self.type_id == *expected_type_id {
            return Ok(());
        }

        Err(GerbilSchemeTypeDecodeError::TypeMismatch {
            expected: expected_type_id.clone(),
            actual: self.type_id.clone(),
        })
    }

    pub fn ensure_schema(
        &self,
        expected_schema_id: &GerbilSchemeSchemaId,
    ) -> Result<(), GerbilSchemeTypeDecodeError> {
        if self.schema_id() == Some(expected_schema_id) {
            return Ok(());
        }

        Err(GerbilSchemeTypeDecodeError::SchemaMismatch {
            type_id: self.type_id.clone(),
            expected: Some(expected_schema_id.clone()),
            actual: self.schema_id.clone(),
        })
    }

    pub fn ensure_type_and_schema(
        &self,
        expected_type_id: &GerbilSchemeTypeId,
        expected_schema_id: Option<&GerbilSchemeSchemaId>,
    ) -> Result<(), GerbilSchemeTypeDecodeError> {
        self.ensure_type(expected_type_id)?;

        if let Some(expected_schema_id) = expected_schema_id {
            self.ensure_schema(expected_schema_id)?;
        }

        Ok(())
    }

    pub fn ensure_projection_contract(
        &self,
        contract: &GerbilSchemeProjectionContract,
    ) -> Result<(), GerbilSchemeTypeDecodeError> {
        self.ensure_type_and_schema(contract.type_id(), contract.schema_id())
    }

    pub fn decode_value<T>(&self) -> Result<T, GerbilSchemeTypeDecodeError>
    where
        T: DeserializeOwned,
    {
        T::deserialize(&self.value).map_err(|error| GerbilSchemeTypeDecodeError::Json {
            message: error.to_string(),
        })
    }

    pub fn decode_value_as<T>(
        &self,
        expected_type_id: &GerbilSchemeTypeId,
    ) -> Result<T, GerbilSchemeTypeDecodeError>
    where
        T: DeserializeOwned,
    {
        self.ensure_type(expected_type_id)?;
        self.decode_value()
    }

    pub fn decode_value_with_contract<T>(
        &self,
        contract: &GerbilSchemeProjectionContract,
    ) -> Result<T, GerbilSchemeTypeDecodeError>
    where
        T: DeserializeOwned,
    {
        self.ensure_projection_contract(contract)?;
        self.decode_value()
    }

    pub fn decode_projection<T>(&self) -> Result<T, GerbilSchemeTypeDecodeError>
    where
        T: GerbilSchemeTypedProjection,
    {
        let contract = T::scheme_projection_contract();
        self.decode_value_with_contract(&contract)
    }
}

/// Decode a Scheme-emitted typed value envelope.
pub fn decode_gerbil_scheme_typed_value(
    value_json: &str,
) -> Result<GerbilSchemeTypedValue, GerbilSchemeTypeDecodeError> {
    serde_json::from_str(value_json).map_err(|error| GerbilSchemeTypeDecodeError::Json {
        message: error.to_string(),
    })
}
