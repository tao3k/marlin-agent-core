//! Decode and validation errors for `Scheme` type manifests.

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

use super::{
    ids::{GerbilSchemeFieldName, GerbilSchemeSchemaId, GerbilSchemeTypeId},
    json_kind::GerbilSchemeJsonTypeKind,
};

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
    UnknownFieldType {
        type_id: GerbilSchemeTypeId,
        field_name: GerbilSchemeFieldName,
        field_type_id: GerbilSchemeTypeId,
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
            Self::UnknownFieldType {
                type_id,
                field_name,
                field_type_id,
            } => write!(
                formatter,
                "Scheme type {} field {} references unknown type_id {}",
                type_id.as_str(),
                field_name.as_str(),
                field_type_id.as_str()
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

fn schema_id_label(schema_id: Option<&GerbilSchemeSchemaId>) -> &str {
    schema_id.map_or("<none>", GerbilSchemeSchemaId::as_str)
}
