//! Generic Scheme type envelopes for Rust-side projections.

use std::{
    collections::BTreeSet,
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

/// Stable field name used inside a Scheme-side type descriptor.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GerbilSchemeFieldName(String);

impl GerbilSchemeFieldName {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Field descriptor for a Scheme-side value type.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilSchemeTypeFieldSpec {
    pub name: GerbilSchemeFieldName,
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

    pub fn validate(
        &self,
    ) -> Result<GerbilSchemeTypeManifestValidationReceipt, GerbilSchemeTypeDecodeError> {
        let mut type_ids = BTreeSet::new();
        let mut field_count = 0;

        for spec in &self.types {
            if !type_ids.insert(spec.type_id.clone()) {
                return Err(GerbilSchemeTypeDecodeError::DuplicateType {
                    type_id: spec.type_id.clone(),
                });
            }

            let mut field_names = BTreeSet::new();
            for field in &spec.fields {
                if !field_names.insert(field.name.clone()) {
                    return Err(GerbilSchemeTypeDecodeError::DuplicateField {
                        type_id: spec.type_id.clone(),
                        field_name: field.name.clone(),
                    });
                }
                field_count += 1;
            }
        }

        Ok(GerbilSchemeTypeManifestValidationReceipt {
            schema_id: self.schema_id.clone(),
            type_count: self.types.len(),
            field_count,
        })
    }

    pub fn validate_typed_value(
        &self,
        typed_value: &GerbilSchemeTypedValue,
    ) -> Result<GerbilSchemeTypedValueValidationReceipt, GerbilSchemeTypeDecodeError> {
        let spec = self.type_spec(typed_value.type_id()).ok_or_else(|| {
            GerbilSchemeTypeDecodeError::UnknownType {
                type_id: typed_value.type_id().clone(),
            }
        })?;

        if let Some(expected_schema_id) = spec.schema_id.as_ref()
            && typed_value.schema_id() != Some(expected_schema_id)
        {
            return Err(GerbilSchemeTypeDecodeError::SchemaMismatch {
                type_id: spec.type_id.clone(),
                expected: Some(expected_schema_id.clone()),
                actual: typed_value.schema_id().cloned(),
            });
        }

        let object = typed_value.value().as_object().ok_or_else(|| {
            GerbilSchemeTypeDecodeError::ValueShape {
                type_id: typed_value.type_id().clone(),
                expected: GerbilSchemeJsonTypeKind::Object,
                actual: GerbilSchemeJsonTypeKind::from_value(typed_value.value()),
            }
        })?;

        let mut required_fields = 0;
        for field in &spec.fields {
            let value = object.get(field.name.as_str());
            if field.required {
                required_fields += 1;
                if value.is_none() {
                    return Err(GerbilSchemeTypeDecodeError::MissingRequiredField {
                        type_id: spec.type_id.clone(),
                        field_name: field.name.clone(),
                    });
                }
            }

            if let Some(value) = value
                && !field_value_matches_type(value, &field.type_id)
            {
                return Err(GerbilSchemeTypeDecodeError::FieldTypeMismatch {
                    type_id: spec.type_id.clone(),
                    field_name: field.name.clone(),
                    expected: field.type_id.clone(),
                    actual: GerbilSchemeJsonTypeKind::from_value(value),
                });
            }
        }

        Ok(GerbilSchemeTypedValueValidationReceipt {
            type_id: typed_value.type_id().clone(),
            schema_id: typed_value.schema_id().cloned(),
            required_fields,
            value_field_count: object.len(),
        })
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
    pub required_fields: usize,
    pub value_field_count: usize,
}

/// JSON value kind observed while validating a Scheme typed value payload.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GerbilSchemeJsonTypeKind {
    Null,
    Boolean,
    Number,
    String,
    Array,
    Object,
}

impl GerbilSchemeJsonTypeKind {
    fn from_value(value: &Value) -> Self {
        match value {
            Value::Null => Self::Null,
            Value::Bool(_) => Self::Boolean,
            Value::Number(_) => Self::Number,
            Value::String(_) => Self::String,
            Value::Array(_) => Self::Array,
            Value::Object(_) => Self::Object,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::Boolean => "boolean",
            Self::Number => "number",
            Self::String => "string",
            Self::Array => "array",
            Self::Object => "object",
        }
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

    pub fn decode_value_with_manifest<T>(
        &self,
        manifest: &GerbilSchemeTypeManifest,
    ) -> Result<T, GerbilSchemeTypeDecodeError>
    where
        T: DeserializeOwned,
    {
        manifest.validate_typed_value(self)?;
        self.decode_value()
    }
}

/// Error raised while decoding Scheme type manifests or typed values.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GerbilSchemeTypeDecodeError {
    Json {
        message: String,
    },
    DuplicateType {
        type_id: GerbilSchemeTypeId,
    },
    DuplicateField {
        type_id: GerbilSchemeTypeId,
        field_name: GerbilSchemeFieldName,
    },
    UnknownType {
        type_id: GerbilSchemeTypeId,
    },
    SchemaMismatch {
        type_id: GerbilSchemeTypeId,
        expected: Option<GerbilSchemeSchemaId>,
        actual: Option<GerbilSchemeSchemaId>,
    },
    ValueShape {
        type_id: GerbilSchemeTypeId,
        expected: GerbilSchemeJsonTypeKind,
        actual: GerbilSchemeJsonTypeKind,
    },
    MissingRequiredField {
        type_id: GerbilSchemeTypeId,
        field_name: GerbilSchemeFieldName,
    },
    FieldTypeMismatch {
        type_id: GerbilSchemeTypeId,
        field_name: GerbilSchemeFieldName,
        expected: GerbilSchemeTypeId,
        actual: GerbilSchemeJsonTypeKind,
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
            Self::DuplicateType { type_id } => write!(
                formatter,
                "Scheme type manifest repeats type_id {}",
                type_id.as_str()
            ),
            Self::DuplicateField {
                type_id,
                field_name,
            } => write!(
                formatter,
                "Scheme type {} repeats field {}",
                type_id.as_str(),
                field_name.as_str()
            ),
            Self::UnknownType { type_id } => write!(
                formatter,
                "Scheme typed value has unknown type_id {}",
                type_id.as_str()
            ),
            Self::SchemaMismatch {
                type_id,
                expected,
                actual,
            } => write!(
                formatter,
                "Scheme typed value {} has schema_id {}, expected {}",
                type_id.as_str(),
                schema_id_label(actual.as_ref()),
                schema_id_label(expected.as_ref())
            ),
            Self::ValueShape {
                type_id,
                expected,
                actual,
            } => write!(
                formatter,
                "Scheme typed value {} has {} payload, expected {}",
                type_id.as_str(),
                actual.as_str(),
                expected.as_str()
            ),
            Self::MissingRequiredField {
                type_id,
                field_name,
            } => write!(
                formatter,
                "Scheme typed value {} is missing required field {}",
                type_id.as_str(),
                field_name.as_str()
            ),
            Self::FieldTypeMismatch {
                type_id,
                field_name,
                expected,
                actual,
            } => write!(
                formatter,
                "Scheme typed value {} field {} has {} payload, expected {}",
                type_id.as_str(),
                field_name.as_str(),
                actual.as_str(),
                expected.as_str()
            ),
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

fn field_value_matches_type(value: &Value, type_id: &GerbilSchemeTypeId) -> bool {
    match type_id.as_str() {
        "any" | "json" => true,
        "null" => value.is_null(),
        "boolean" => value.is_boolean(),
        "number" => value.is_number(),
        "integer" => value.as_i64().is_some() || value.as_u64().is_some(),
        "string" => value.is_string(),
        "array" => value.is_array(),
        "object" => value.is_object(),
        _ => true,
    }
}

fn schema_id_label(schema_id: Option<&GerbilSchemeSchemaId>) -> &str {
    schema_id.map_or("<none>", GerbilSchemeSchemaId::as_str)
}
