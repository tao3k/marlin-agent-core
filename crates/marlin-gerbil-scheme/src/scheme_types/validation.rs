//! Structural validation for `Scheme` type manifests and typed values.

use std::collections::BTreeSet;

use super::{
    error::GerbilSchemeTypeDecodeError,
    ids::GerbilSchemeTypeId,
    manifest::{
        GerbilSchemeTypeManifest, GerbilSchemeTypeManifestValidationReceipt, GerbilSchemeTypeSpec,
        GerbilSchemeTypedValueValidationReceipt,
    },
    typed_value::GerbilSchemeTypedValue,
};

/// Validate a `Gerbil` Scheme type manifest before building a registry.
pub fn validate_gerbil_scheme_type_manifest(
    manifest: &GerbilSchemeTypeManifest,
) -> Result<GerbilSchemeTypeManifestValidationReceipt, GerbilSchemeTypeDecodeError> {
    let type_ids = collect_unique_type_ids(manifest)?;
    let field_count = validate_unique_fields(manifest)?;
    validate_field_type_references(manifest, &type_ids)?;

    Ok(GerbilSchemeTypeManifestValidationReceipt {
        schema_id: manifest.schema_id.clone(),
        type_count: manifest.types.len(),
        field_count,
    })
}

/// Validate a typed value against a `Gerbil` Scheme manifest.
pub fn validate_gerbil_scheme_typed_value(
    manifest: &GerbilSchemeTypeManifest,
    typed_value: &GerbilSchemeTypedValue,
) -> Result<GerbilSchemeTypedValueValidationReceipt, GerbilSchemeTypeDecodeError> {
    validate_typed_value_with_lookup(|type_id| manifest.type_spec(type_id), typed_value)
}

pub(super) fn validate_typed_value_with_lookup<'a>(
    mut lookup: impl FnMut(&GerbilSchemeTypeId) -> Option<&'a GerbilSchemeTypeSpec>,
    typed_value: &GerbilSchemeTypedValue,
) -> Result<GerbilSchemeTypedValueValidationReceipt, GerbilSchemeTypeDecodeError> {
    let spec =
        lookup(typed_value.type_id()).ok_or_else(|| GerbilSchemeTypeDecodeError::UnknownType {
            type_id: typed_value.type_id().clone(),
        })?;

    validate_typed_value_schema(spec, typed_value)?;

    Ok(GerbilSchemeTypedValueValidationReceipt {
        type_id: spec.type_id.clone(),
        schema_id: typed_value.schema_id().cloned(),
        declared_field_count: spec.fields.len(),
    })
}

fn collect_unique_type_ids(
    manifest: &GerbilSchemeTypeManifest,
) -> Result<BTreeSet<GerbilSchemeTypeId>, GerbilSchemeTypeDecodeError> {
    manifest
        .types
        .iter()
        .try_fold(BTreeSet::new(), |mut type_ids, spec| {
            if type_ids.insert(spec.type_id.clone()) {
                Ok(type_ids)
            } else {
                Err(GerbilSchemeTypeDecodeError::DuplicateType {
                    type_id: spec.type_id.clone(),
                })
            }
        })
}

fn validate_unique_fields(
    manifest: &GerbilSchemeTypeManifest,
) -> Result<usize, GerbilSchemeTypeDecodeError> {
    manifest.types.iter().try_fold(0, |field_count, spec| {
        validate_unique_fields_for_type(spec).map(|count| field_count + count)
    })
}

fn validate_field_type_references(
    manifest: &GerbilSchemeTypeManifest,
    type_ids: &BTreeSet<GerbilSchemeTypeId>,
) -> Result<(), GerbilSchemeTypeDecodeError> {
    manifest
        .types
        .iter()
        .try_for_each(|spec| validate_field_type_references_for_type(spec, type_ids))
}

fn validate_unique_fields_for_type(
    spec: &GerbilSchemeTypeSpec,
) -> Result<usize, GerbilSchemeTypeDecodeError> {
    spec.fields
        .iter()
        .try_fold(BTreeSet::new(), |mut field_names, field| {
            if field_names.insert(field.name.clone()) {
                Ok(field_names)
            } else {
                Err(GerbilSchemeTypeDecodeError::DuplicateField {
                    type_id: spec.type_id.clone(),
                    field_name: field.name.clone(),
                })
            }
        })
        .map(|_| spec.fields.len())
}

fn validate_field_type_references_for_type(
    spec: &GerbilSchemeTypeSpec,
    type_ids: &BTreeSet<GerbilSchemeTypeId>,
) -> Result<(), GerbilSchemeTypeDecodeError> {
    spec.fields.iter().try_for_each(|field| {
        if is_builtin_scheme_type(&field.type_id) || type_ids.contains(&field.type_id) {
            Ok(())
        } else {
            Err(GerbilSchemeTypeDecodeError::UnknownFieldType {
                type_id: spec.type_id.clone(),
                field_name: field.name.clone(),
                field_type_id: field.type_id.clone(),
            })
        }
    })
}

fn validate_typed_value_schema(
    spec: &GerbilSchemeTypeSpec,
    typed_value: &GerbilSchemeTypedValue,
) -> Result<(), GerbilSchemeTypeDecodeError> {
    if let Some(expected_schema_id) = spec.schema_id.as_ref()
        && typed_value.schema_id() != Some(expected_schema_id)
    {
        return Err(GerbilSchemeTypeDecodeError::SchemaMismatch {
            type_id: spec.type_id.clone(),
            expected: Some(expected_schema_id.clone()),
            actual: typed_value.schema_id().cloned(),
        });
    }

    Ok(())
}

fn is_builtin_scheme_type(type_id: &GerbilSchemeTypeId) -> bool {
    matches!(
        type_id.as_str(),
        "any" | "json" | "null" | "boolean" | "number" | "integer" | "string" | "array" | "object"
    )
}
