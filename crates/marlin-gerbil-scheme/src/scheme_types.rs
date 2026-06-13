//! Generic Scheme type envelopes for Rust-side projections.

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;

/// Stable identifier for a Scheme-side value type.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GerbilSchemeTypeId(String);

impl GerbilSchemeTypeId {
    pub fn new(type_id: impl Into<String>) -> Self {
        Self(type_id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Stable identifier for a JSON schema emitted by Scheme.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GerbilSchemeSchemaId(String);

impl GerbilSchemeSchemaId {
    pub fn new(schema_id: impl Into<String>) -> Self {
        Self(schema_id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Field descriptor for a Scheme-side value type.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilSchemeTypeFieldSpec {
    pub name: String,
    pub type_id: GerbilSchemeTypeId,
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
    pub fn field(&self, name: &str) -> Option<&GerbilSchemeTypeFieldSpec> {
        self.fields.iter().find(|field| field.name == name)
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

    pub fn decode_value<T>(&self) -> Result<T, GerbilSchemeTypeDecodeError>
    where
        T: DeserializeOwned,
    {
        serde_json::from_value(self.value.clone()).map_err(|error| {
            GerbilSchemeTypeDecodeError::Json {
                message: error.to_string(),
            }
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
}

/// Error raised while decoding Scheme type manifests or typed values.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GerbilSchemeTypeDecodeError {
    Json {
        message: String,
    },
    TypeMismatch {
        expected: GerbilSchemeTypeId,
        actual: GerbilSchemeTypeId,
    },
}

impl Display for GerbilSchemeTypeDecodeError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json { message } => {
                write!(formatter, "failed to decode Scheme type JSON: {message}")
            }
            Self::TypeMismatch { expected, actual } => write!(
                formatter,
                "Scheme typed value has type_id {}, expected {}",
                actual.as_str(),
                expected.as_str()
            ),
        }
    }
}

impl Error for GerbilSchemeTypeDecodeError {}

/// Decode a Scheme-emitted type manifest.
pub fn decode_gerbil_scheme_type_manifest(
    manifest_json: &str,
) -> Result<GerbilSchemeTypeManifest, GerbilSchemeTypeDecodeError> {
    serde_json::from_str(manifest_json).map_err(|error| GerbilSchemeTypeDecodeError::Json {
        message: error.to_string(),
    })
}

/// Decode a Scheme-emitted typed value envelope.
pub fn decode_gerbil_scheme_typed_value(
    value_json: &str,
) -> Result<GerbilSchemeTypedValue, GerbilSchemeTypeDecodeError> {
    serde_json::from_str(value_json).map_err(|error| GerbilSchemeTypeDecodeError::Json {
        message: error.to_string(),
    })
}
