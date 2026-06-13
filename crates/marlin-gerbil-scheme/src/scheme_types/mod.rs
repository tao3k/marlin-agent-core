//! Generic `Scheme` type envelopes for Rust-side projections.

mod error;
mod ids;
mod json_kind;
mod manifest;
mod registry;
mod typed_value;
mod validation;

pub use error::GerbilSchemeTypeDecodeError;
pub use ids::{GerbilSchemeFieldName, GerbilSchemeSchemaId, GerbilSchemeTypeId};
pub use json_kind::GerbilSchemeJsonTypeKind;
pub use manifest::{
    GerbilSchemeTypeFieldSpec, GerbilSchemeTypeManifest, GerbilSchemeTypeManifestValidationReceipt,
    GerbilSchemeTypeSpec, GerbilSchemeTypedValueValidationReceipt,
    decode_gerbil_scheme_type_manifest,
};
pub use registry::GerbilSchemeTypeRegistry;
pub use typed_value::{GerbilSchemeTypedValue, decode_gerbil_scheme_typed_value};
pub use validation::{
    validate_gerbil_scheme_type_manifest, validate_gerbil_scheme_typed_value,
    validate_gerbil_scheme_value_as_type,
};
