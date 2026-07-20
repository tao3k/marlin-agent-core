//! Structural validation for `Scheme` type manifests and typed values.

use std::collections::BTreeSet;

use super::{
    error::GerbilSchemeTypeDecodeError,
    ids::GerbilSchemeTypeId,
    manifest::{
        GerbilSchemeTypeFieldSpec, GerbilSchemeTypeManifest,
        GerbilSchemeTypeManifestValidationReceipt, GerbilSchemeTypeSpec,
        GerbilSchemeTypedValueValidationReceipt,
    },
    typed_value::{GerbilSchemeTypedValue, GerbilSchemeValue},
};

const MAX_DYNAMIC_VALIDATION_DEPTH: usize = 32;

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

use crate::GerbilSchemeBackedBindingCatalogReceipt;

/// Return the reusable Gerbil Scheme Rust binding surface Marlin can currently
/// delegate to without treating fixture-only value shapes as upstream support.
pub fn gerbil_scheme_backed_binding_catalog() -> GerbilSchemeBackedBindingCatalogReceipt {
    GerbilSchemeBackedBindingCatalogReceipt {
        upstream_crate: "gerbil-scheme".to_owned(),
        backed_shape_selectors: GERBIL_SCHEME_RUST_BACKED_SHAPE_SELECTORS
            .iter()
            .map(|selector| (*selector).to_owned())
            .collect(),
        fixture_only_value_families: MARLIN_FIXTURE_ONLY_VALUE_FAMILIES
            .iter()
            .map(|family| (*family).to_owned())
            .collect(),
    }
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
        validate_field_type_reference(spec, field, type_ids)?;

        if let Some(element_type_id) = &field.element_type_id {
            if field.type_id.as_str() != "array" {
                return Err(
                    GerbilSchemeTypeDecodeError::ArrayElementTypeOnNonArrayField {
                        type_id: spec.type_id.clone(),
                        field_name: field.name.clone(),
                        field_type_id: field.type_id.clone(),
                    },
                );
            }
            validate_field_type_reference_for_type_id(spec, field, element_type_id, type_ids)?;
        }

        Ok(())
    })
}

fn validate_field_type_reference(
    spec: &GerbilSchemeTypeSpec,
    field: &GerbilSchemeTypeFieldSpec,
    type_ids: &BTreeSet<GerbilSchemeTypeId>,
) -> Result<(), GerbilSchemeTypeDecodeError> {
    validate_field_type_reference_for_type_id(spec, field, &field.type_id, type_ids)
}

fn validate_field_type_reference_for_type_id(
    spec: &GerbilSchemeTypeSpec,
    field: &GerbilSchemeTypeFieldSpec,
    type_id: &GerbilSchemeTypeId,
    type_ids: &BTreeSet<GerbilSchemeTypeId>,
) -> Result<(), GerbilSchemeTypeDecodeError> {
    if is_builtin_scheme_type(type_id) || type_ids.contains(type_id) {
        Ok(())
    } else {
        Err(GerbilSchemeTypeDecodeError::UnknownFieldType {
            type_id: spec.type_id.clone(),
            field_name: field.name.clone(),
            field_type_id: type_id.clone(),
        })
    }
}

const GERBIL_SCHEME_RUST_BACKED_SHAPE_SELECTORS: &[&str] = &[
    "gerbil_scheme_rust_i64_shape",
    "gerbil_scheme_rust_bool_shape",
    "gerbil_scheme_rust_comparison_shape",
    "gerbil_scheme_rust_utf8_shape",
    "gerbil_scheme_rust_value_handle_shape",
    "gerbil_scheme_rust_value_is_pair",
    "gerbil_scheme_rust_value_is_list",
    "gerbil_scheme_rust_value_is_null",
    "gerbil_scheme_rust_pair_car",
    "gerbil_scheme_rust_pair_cdr",
    "gerbil_scheme_rust_pair_parts",
    "gerbil_scheme_rust_i64_callback_shape",
    "gerbil_scheme_rust_native_value_shape",
    "gerbil_scheme_rust_native_error_shape",
    "gerbil_scheme_rust_native_result_shape",
];

const MARLIN_FIXTURE_ONLY_VALUE_FAMILIES: &[&str] = &["null-sentinel", "f64", "vector", "record"];

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

pub(super) fn validate_typed_value_payload<'a>(
    mut lookup: impl FnMut(&GerbilSchemeTypeId) -> Option<&'a GerbilSchemeTypeSpec>,
    spec: &'a GerbilSchemeTypeSpec,
    typed_value: &GerbilSchemeTypedValue,
) -> Result<(), GerbilSchemeTypeDecodeError> {
    validate_typed_value_payload_value(
        &mut lookup,
        spec,
        typed_value.value(),
        &mut Vec::new(),
        &mut Vec::new(),
    )
}

fn validate_typed_value_payload_value<'a>(
    lookup: &mut impl FnMut(&GerbilSchemeTypeId) -> Option<&'a GerbilSchemeTypeSpec>,
    spec: &GerbilSchemeTypeSpec,
    value: &GerbilSchemeValue,
    field_path: &mut Vec<String>,
    type_stack: &mut Vec<GerbilSchemeTypeId>,
) -> Result<(), GerbilSchemeTypeDecodeError> {
    if type_stack.iter().any(|type_id| type_id == &spec.type_id) {
        return Err(GerbilSchemeTypeDecodeError::RecursiveTypeReference {
            type_id: spec.type_id.clone(),
            field_path: format_field_path(field_path),
        });
    }
    if type_stack.len() >= MAX_DYNAMIC_VALIDATION_DEPTH {
        return Err(
            GerbilSchemeTypeDecodeError::DynamicValidationDepthExceeded {
                type_id: spec.type_id.clone(),
                max_depth: MAX_DYNAMIC_VALIDATION_DEPTH,
                field_path: format_field_path(field_path),
            },
        );
    }

    type_stack.push(spec.type_id.clone());
    let result = validate_typed_value_payload_fields(lookup, spec, value, field_path, type_stack);
    type_stack.pop();
    result
}

fn validate_typed_value_payload_fields<'a>(
    lookup: &mut impl FnMut(&GerbilSchemeTypeId) -> Option<&'a GerbilSchemeTypeSpec>,
    spec: &GerbilSchemeTypeSpec,
    value: &GerbilSchemeValue,
    field_path: &mut Vec<String>,
    type_stack: &mut Vec<GerbilSchemeTypeId>,
) -> Result<(), GerbilSchemeTypeDecodeError> {
    if spec.fields.is_empty() {
        return Ok(());
    }

    let object =
        value
            .as_record()
            .ok_or_else(|| GerbilSchemeTypeDecodeError::ValueTypeMismatch {
                type_id: spec.type_id.clone(),
                expected: GerbilSchemeTypeId::new("object"),
                field_path: format_field_path(field_path),
            })?;

    for field in &spec.fields {
        let Some(value) = object.get(field.name.as_str()) else {
            if field.required {
                return Err(GerbilSchemeTypeDecodeError::MissingRequiredField {
                    type_id: spec.type_id.clone(),
                    field_name: field.name.clone(),
                    field_path: format_field_path_with(field_path, field.name.as_str()),
                });
            }
            continue;
        };

        if builtin_field_value_matches(field.type_id.as_str(), value) == Some(false) {
            return Err(GerbilSchemeTypeDecodeError::FieldTypeMismatch {
                type_id: spec.type_id.clone(),
                field_name: field.name.clone(),
                expected: field.type_id.clone(),
                field_path: format_field_path_with(field_path, field.name.as_str()),
            });
        }

        if field.type_id.as_str() == "array" {
            validate_array_elements(lookup, spec, field, value, field_path, type_stack)?;
        } else if !is_builtin_scheme_type(&field.type_id) {
            let Some(nested_spec) = lookup(&field.type_id) else {
                continue;
            };
            if !value.is_record() {
                return Err(GerbilSchemeTypeDecodeError::FieldTypeMismatch {
                    type_id: spec.type_id.clone(),
                    field_name: field.name.clone(),
                    expected: field.type_id.clone(),
                    field_path: format_field_path_with(field_path, field.name.as_str()),
                });
            }
            field_path.push(field.name.as_str().to_owned());
            let result = validate_typed_value_payload_value(
                lookup,
                nested_spec,
                value,
                field_path,
                type_stack,
            );
            field_path.pop();
            result?;
        }
    }

    Ok(())
}

fn validate_array_elements<'a>(
    lookup: &mut impl FnMut(&GerbilSchemeTypeId) -> Option<&'a GerbilSchemeTypeSpec>,
    spec: &GerbilSchemeTypeSpec,
    field: &GerbilSchemeTypeFieldSpec,
    value: &GerbilSchemeValue,
    field_path: &mut Vec<String>,
    type_stack: &mut Vec<GerbilSchemeTypeId>,
) -> Result<(), GerbilSchemeTypeDecodeError> {
    let Some(element_type_id) = &field.element_type_id else {
        return Ok(());
    };
    let Some(elements) = value.as_vector() else {
        return Ok(());
    };

    for (index, element) in elements.iter().enumerate() {
        field_path.push(format!("{}[{index}]", field.name.as_str()));
        if builtin_field_value_matches(element_type_id.as_str(), element) == Some(false) {
            return Err(GerbilSchemeTypeDecodeError::FieldTypeMismatch {
                type_id: spec.type_id.clone(),
                field_name: field.name.clone(),
                expected: element_type_id.clone(),
                field_path: format_field_path(field_path),
            });
        }

        if !is_builtin_scheme_type(element_type_id) {
            let Some(nested_spec) = lookup(element_type_id) else {
                field_path.pop();
                continue;
            };
            if !element.is_record() {
                return Err(GerbilSchemeTypeDecodeError::FieldTypeMismatch {
                    type_id: spec.type_id.clone(),
                    field_name: field.name.clone(),
                    expected: element_type_id.clone(),
                    field_path: format_field_path(field_path),
                });
            }
            let result = validate_typed_value_payload_value(
                lookup,
                nested_spec,
                element,
                field_path,
                type_stack,
            );
            field_path.pop();
            result?;
        } else {
            field_path.pop();
        }
    }

    Ok(())
}

fn is_builtin_scheme_type(type_id: &GerbilSchemeTypeId) -> bool {
    matches!(
        type_id.as_str(),
        "any" | "null" | "boolean" | "number" | "integer" | "string" | "array" | "object"
    )
}

fn builtin_field_value_matches(type_id: &str, value: &GerbilSchemeValue) -> Option<bool> {
    match type_id {
        "any" => Some(true),
        "null" => Some(value.is_null()),
        "boolean" => Some(value.is_boolean()),
        "number" => Some(value.is_number()),
        "integer" => Some(value.is_integer()),
        "string" => Some(value.is_text()),
        "array" => Some(value.is_vector()),
        "object" => Some(value.is_record()),
        _ => None,
    }
}

fn format_field_path(path: &[String]) -> String {
    if path.is_empty() {
        "<root>".to_owned()
    } else {
        path.join(".")
    }
}

fn format_field_path_with(path: &[String], field_name: &str) -> String {
    if path.is_empty() {
        field_name.to_owned()
    } else {
        format!("{}.{field_name}", path.join("."))
    }
}
