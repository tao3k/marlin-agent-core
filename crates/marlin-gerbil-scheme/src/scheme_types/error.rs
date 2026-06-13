//! Decode and validation errors for `Scheme` type manifests.

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

use super::ids::{
    GerbilSchemeFieldName, GerbilSchemeNativeAbiId, GerbilSchemeNativeSymbol,
    GerbilSchemePackageId, GerbilSchemeSchemaId, GerbilSchemeTypeId,
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
    ArrayElementTypeOnNonArrayField {
        type_id: GerbilSchemeTypeId,
        field_name: GerbilSchemeFieldName,
        field_type_id: GerbilSchemeTypeId,
    },
    UnknownType {
        type_id: GerbilSchemeTypeId,
    },
    ValueTypeMismatch {
        type_id: GerbilSchemeTypeId,
        expected: GerbilSchemeTypeId,
        field_path: String,
    },
    MissingRequiredField {
        type_id: GerbilSchemeTypeId,
        field_name: GerbilSchemeFieldName,
        field_path: String,
    },
    FieldTypeMismatch {
        type_id: GerbilSchemeTypeId,
        field_name: GerbilSchemeFieldName,
        expected: GerbilSchemeTypeId,
        field_path: String,
    },
    RecursiveTypeReference {
        type_id: GerbilSchemeTypeId,
        field_path: String,
    },
    DynamicValidationDepthExceeded {
        type_id: GerbilSchemeTypeId,
        max_depth: usize,
        field_path: String,
    },
    SchemaMismatch {
        type_id: GerbilSchemeTypeId,
        expected: Option<GerbilSchemeSchemaId>,
        actual: Option<GerbilSchemeSchemaId>,
    },
    DuplicateProjectionContract {
        type_id: GerbilSchemeTypeId,
        schema_id: Option<GerbilSchemeSchemaId>,
    },
    MissingNativeSymbols {
        abi_id: GerbilSchemeNativeAbiId,
    },
    DuplicateNativeSymbol {
        symbol: GerbilSchemeNativeSymbol,
    },
    MissingNativeAbi {
        package_id: GerbilSchemePackageId,
    },
    NativeAbiMismatch {
        expected: GerbilSchemeNativeAbiId,
        actual: GerbilSchemeNativeAbiId,
    },
    NativeAbiVersionMismatch {
        abi_id: GerbilSchemeNativeAbiId,
        expected: u32,
        actual: u32,
    },
    MissingNativeSymbol {
        symbol: GerbilSchemeNativeSymbol,
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
            Self::ArrayElementTypeOnNonArrayField {
                type_id,
                field_name,
                field_type_id,
            } => write!(
                formatter,
                "Scheme type {} field {} declares element_type_id for non-array type {}",
                type_id.as_str(),
                field_name.as_str(),
                field_type_id.as_str()
            ),
            Self::UnknownType { type_id } => write!(
                formatter,
                "Scheme typed value has unknown type_id {}",
                type_id.as_str()
            ),
            Self::ValueTypeMismatch {
                type_id,
                expected,
                field_path,
            } => write!(
                formatter,
                "Scheme typed value {} path {} must be {}",
                type_id.as_str(),
                field_path,
                expected.as_str()
            ),
            Self::MissingRequiredField {
                type_id,
                field_name,
                field_path,
            } => write!(
                formatter,
                "Scheme typed value {} is missing required field {} at path {}",
                type_id.as_str(),
                field_name.as_str(),
                field_path
            ),
            Self::FieldTypeMismatch {
                type_id,
                field_name,
                expected,
                field_path,
            } => write!(
                formatter,
                "Scheme typed value {} field {} at path {} must be {}",
                type_id.as_str(),
                field_name.as_str(),
                field_path,
                expected.as_str()
            ),
            Self::RecursiveTypeReference {
                type_id,
                field_path,
            } => write!(
                formatter,
                "Scheme typed value recursion reached type_id {} at path {}",
                type_id.as_str(),
                field_path
            ),
            Self::DynamicValidationDepthExceeded {
                type_id,
                max_depth,
                field_path,
            } => write!(
                formatter,
                "Scheme typed value dynamic validation exceeded depth {} at type_id {} path {}",
                max_depth,
                type_id.as_str(),
                field_path
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
            Self::DuplicateProjectionContract { type_id, schema_id } => write!(
                formatter,
                "Scheme package repeats projection contract for type_id {} schema_id {}",
                type_id.as_str(),
                schema_id_label(schema_id.as_ref())
            ),
            Self::MissingNativeSymbols { abi_id } => write!(
                formatter,
                "Scheme native ABI {} declares no exported symbols",
                abi_id.as_str()
            ),
            Self::DuplicateNativeSymbol { symbol } => write!(
                formatter,
                "Scheme native ABI repeats exported symbol {}",
                symbol.as_str()
            ),
            Self::MissingNativeAbi { package_id } => write!(
                formatter,
                "Scheme package {} declares no native ABI",
                package_id.as_str()
            ),
            Self::NativeAbiMismatch { expected, actual } => write!(
                formatter,
                "Scheme native ABI id {}, expected {}",
                actual.as_str(),
                expected.as_str()
            ),
            Self::NativeAbiVersionMismatch {
                abi_id,
                expected,
                actual,
            } => write!(
                formatter,
                "Scheme native ABI {} has version {}, expected {}",
                abi_id.as_str(),
                actual,
                expected
            ),
            Self::MissingNativeSymbol { symbol } => write!(
                formatter,
                "Scheme native ABI is missing exported symbol {}",
                symbol.as_str()
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
