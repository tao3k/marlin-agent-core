//! Generic `Scheme` type envelopes for Rust-side projections.

mod error;
mod ids;
mod manifest;
mod package;
mod projection;
mod registry;
mod typed_value;
mod validation;

pub use error::GerbilSchemeTypeDecodeError;
pub use ids::{
    GerbilSchemeFieldName, GerbilSchemeNativeAbiId, GerbilSchemeNativeSymbol,
    GerbilSchemePackageId, GerbilSchemeSchemaId, GerbilSchemeTypeId,
};
pub use manifest::{
    GerbilSchemeTypeFieldSpec, GerbilSchemeTypeManifest, GerbilSchemeTypeManifestValidationReceipt,
    GerbilSchemeTypeSpec, GerbilSchemeTypedValueValidationReceipt,
    decode_gerbil_scheme_type_manifest,
};
pub use package::{
    GerbilSchemeNativeAbiContract, GerbilSchemeNativeAbiReadinessPlan, GerbilSchemePackageManifest,
    GerbilSchemePackageManifestValidationReceipt, GerbilSchemePackageNativeReadinessReceipt,
    decode_gerbil_scheme_package_manifest, validate_gerbil_scheme_package_manifest,
    validate_gerbil_scheme_package_native_readiness,
};
pub use projection::{GerbilSchemeProjectionContract, GerbilSchemeTypedProjection};
pub use registry::GerbilSchemeTypeRegistry;
pub use typed_value::{GerbilSchemeTypedValue, decode_gerbil_scheme_typed_value};
pub use validation::{validate_gerbil_scheme_type_manifest, validate_gerbil_scheme_typed_value};
