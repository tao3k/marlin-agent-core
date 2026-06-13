//! Structural validation for `Scheme` type manifests and typed values.

use std::collections::BTreeSet;

use serde_json::Value;

use super::{
    error::GerbilSchemeTypeDecodeError,
    ids::{GerbilSchemeSchemaId, GerbilSchemeTypeId},
    json_kind::GerbilSchemeJsonTypeKind,
    manifest::{
        GerbilSchemeTypeFieldSpec, GerbilSchemeTypeManifest,
        GerbilSchemeTypeManifestValidationReceipt, GerbilSchemeTypeSpec,
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

/// Validate a raw `JSON` payload as a named `Gerbil` Scheme type.
pub fn validate_gerbil_scheme_value_as_type(
    manifest: &GerbilSchemeTypeManifest,
    type_id: &GerbilSchemeTypeId,
    value: &Value,
) -> Result<GerbilSchemeTypedValueValidationReceipt, GerbilSchemeTypeDecodeError> {
    validate_value_as_type_with_lookup(
        |resolved_type_id| manifest.type_spec(resolved_type_id),
        type_id,
        value,
    )
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
    validate_typed_value_against_spec(spec, typed_value, &mut lookup)
}

pub(super) fn validate_value_as_type_with_lookup<'a>(
    mut lookup: impl FnMut(&GerbilSchemeTypeId) -> Option<&'a GerbilSchemeTypeSpec>,
    type_id: &GerbilSchemeTypeId,
    value: &Value,
) -> Result<GerbilSchemeTypedValueValidationReceipt, GerbilSchemeTypeDecodeError> {
    let spec = lookup(type_id).ok_or_else(|| GerbilSchemeTypeDecodeError::UnknownType {
        type_id: type_id.clone(),
    })?;
    let schema_id = schema_id_from_value(value);

    validate_value_against_spec(spec, schema_id.as_ref(), value, &mut lookup)
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

fn validate_typed_value_against_spec<'a>(
    spec: &GerbilSchemeTypeSpec,
    typed_value: &GerbilSchemeTypedValue,
    lookup: &mut impl FnMut(&GerbilSchemeTypeId) -> Option<&'a GerbilSchemeTypeSpec>,
) -> Result<GerbilSchemeTypedValueValidationReceipt, GerbilSchemeTypeDecodeError> {
    validate_value_against_spec(spec, typed_value.schema_id(), typed_value.value(), lookup)
}

fn validate_value_against_spec<'a>(
    spec: &GerbilSchemeTypeSpec,
    schema_id: Option<&GerbilSchemeSchemaId>,
    value: &Value,
    lookup: &mut impl FnMut(&GerbilSchemeTypeId) -> Option<&'a GerbilSchemeTypeSpec>,
) -> Result<GerbilSchemeTypedValueValidationReceipt, GerbilSchemeTypeDecodeError> {
    if let Some(expected_schema_id) = spec.schema_id.as_ref()
        && schema_id != Some(expected_schema_id)
    {
        return Err(GerbilSchemeTypeDecodeError::SchemaMismatch {
            type_id: spec.type_id.clone(),
            expected: Some(expected_schema_id.clone()),
            actual: schema_id.cloned(),
        });
    }

    let object = value
        .as_object()
        .ok_or_else(|| GerbilSchemeTypeDecodeError::ValueShape {
            type_id: spec.type_id.clone(),
            expected: GerbilSchemeJsonTypeKind::Object,
            actual: GerbilSchemeJsonTypeKind::from_value(value),
        })?;

    let required_fields = spec.fields.iter().try_fold(0, |required_fields, field| {
        validate_typed_value_field(spec, field, object, lookup)
            .map(|field_required| required_fields + usize::from(field_required))
    })?;

    Ok(GerbilSchemeTypedValueValidationReceipt {
        type_id: spec.type_id.clone(),
        schema_id: schema_id.cloned(),
        required_fields,
        value_field_count: object.len(),
    })
}

fn validate_typed_value_field<'a>(
    spec: &GerbilSchemeTypeSpec,
    field: &GerbilSchemeTypeFieldSpec,
    object: &serde_json::Map<String, Value>,
    lookup: &mut impl FnMut(&GerbilSchemeTypeId) -> Option<&'a GerbilSchemeTypeSpec>,
) -> Result<bool, GerbilSchemeTypeDecodeError> {
    let value = object.get(field.name.as_str());
    if field.required && value.is_none() {
        return Err(GerbilSchemeTypeDecodeError::MissingRequiredField {
            type_id: spec.type_id.clone(),
            field_name: field.name.clone(),
        });
    }

    if let Some(value) = value {
        validate_field_value_matches_type(spec, field, value, lookup)?;
    }

    Ok(field.required)
}

fn validate_field_value_matches_type<'a>(
    spec: &GerbilSchemeTypeSpec,
    field: &GerbilSchemeTypeFieldSpec,
    value: &Value,
    lookup: &mut impl FnMut(&GerbilSchemeTypeId) -> Option<&'a GerbilSchemeTypeSpec>,
) -> Result<(), GerbilSchemeTypeDecodeError> {
    if is_builtin_scheme_type(&field.type_id) {
        return validate_builtin_field_value_matches_type(spec, field, value);
    }

    let nested_spec =
        lookup(&field.type_id).ok_or_else(|| GerbilSchemeTypeDecodeError::UnknownFieldType {
            type_id: spec.type_id.clone(),
            field_name: field.name.clone(),
            field_type_id: field.type_id.clone(),
        })?;

    if !value.is_object() {
        return Err(GerbilSchemeTypeDecodeError::FieldTypeMismatch {
            type_id: spec.type_id.clone(),
            field_name: field.name.clone(),
            expected: field.type_id.clone(),
            actual: GerbilSchemeJsonTypeKind::from_value(value),
        });
    }

    let schema_id = schema_id_from_value(value);
    validate_value_against_spec(nested_spec, schema_id.as_ref(), value, lookup).map(|_| ())
}

fn validate_builtin_field_value_matches_type(
    spec: &GerbilSchemeTypeSpec,
    field: &GerbilSchemeTypeFieldSpec,
    value: &Value,
) -> Result<(), GerbilSchemeTypeDecodeError> {
    if builtin_field_value_matches_type(value, &field.type_id) {
        Ok(())
    } else {
        Err(GerbilSchemeTypeDecodeError::FieldTypeMismatch {
            type_id: spec.type_id.clone(),
            field_name: field.name.clone(),
            expected: field.type_id.clone(),
            actual: GerbilSchemeJsonTypeKind::from_value(value),
        })
    }
}

fn builtin_field_value_matches_type(value: &Value, type_id: &GerbilSchemeTypeId) -> bool {
    match type_id.as_str() {
        "any" | "json" => true,
        "null" => value.is_null(),
        "boolean" => value.is_boolean(),
        "number" => value.is_number(),
        "integer" => value.as_i64().is_some() || value.as_u64().is_some(),
        "string" => value.is_string(),
        "array" => value.is_array(),
        "object" => value.is_object(),
        _ => false,
    }
}

fn is_builtin_scheme_type(type_id: &GerbilSchemeTypeId) -> bool {
    matches!(
        type_id.as_str(),
        "any" | "json" | "null" | "boolean" | "number" | "integer" | "string" | "array" | "object"
    )
}

fn schema_id_from_value(value: &Value) -> Option<GerbilSchemeSchemaId> {
    value
        .as_object()
        .and_then(|object| object.get("schema_id"))
        .and_then(Value::as_str)
        .map(GerbilSchemeSchemaId::new)
}
