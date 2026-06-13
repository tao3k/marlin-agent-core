//! Manifest descriptors emitted by `Scheme` and validated in Rust.

use serde::{Deserialize, Serialize};

use super::{
    error::GerbilSchemeTypeDecodeError,
    ids::{GerbilSchemeFieldName, GerbilSchemeSchemaId, GerbilSchemeTypeId},
};

/// Field descriptor for a Scheme-side value type.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilSchemeTypeFieldSpec {
    pub name: GerbilSchemeFieldName,
    pub type_id: GerbilSchemeTypeId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub element_type_id: Option<GerbilSchemeTypeId>,
    #[serde(default)]
    pub required: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Type descriptor emitted by Scheme and consumed by Rust without codegen.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilSchemeTypeSpec {
    pub type_id: GerbilSchemeTypeId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_id: Option<GerbilSchemeSchemaId>,
    #[serde(default)]
    pub fields: Vec<GerbilSchemeTypeFieldSpec>,
}

impl GerbilSchemeTypeSpec {
    pub fn field(&self, name: &GerbilSchemeFieldName) -> Option<&GerbilSchemeTypeFieldSpec> {
        self.fields.iter().find(|field| &field.name == name)
    }
}

/// Manifest of Scheme-side value types available from a runtime package.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilSchemeTypeManifest {
    pub schema_id: GerbilSchemeSchemaId,
    #[serde(default)]
    pub types: Vec<GerbilSchemeTypeSpec>,
}

impl GerbilSchemeTypeManifest {
    pub fn type_spec(&self, type_id: &GerbilSchemeTypeId) -> Option<&GerbilSchemeTypeSpec> {
        self.types.iter().find(|spec| &spec.type_id == type_id)
    }
}

/// Manifest validation receipt used by tests and quality gates.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilSchemeTypeManifestValidationReceipt {
    pub schema_id: GerbilSchemeSchemaId,
    pub type_count: usize,
    pub field_count: usize,
}

/// Typed value validation receipt used by tests and quality gates.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilSchemeTypedValueValidationReceipt {
    pub type_id: GerbilSchemeTypeId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_id: Option<GerbilSchemeSchemaId>,
    pub declared_field_count: usize,
}

/// Decode a serialized Scheme type manifest fixture.
pub fn decode_gerbil_scheme_type_manifest_fixture(
    fixture: &str,
) -> Result<GerbilSchemeTypeManifest, GerbilSchemeTypeDecodeError> {
    serde_json::from_str(fixture).map_err(|error| GerbilSchemeTypeDecodeError::SerializedFixture {
        message: error.to_string(),
    })
}
