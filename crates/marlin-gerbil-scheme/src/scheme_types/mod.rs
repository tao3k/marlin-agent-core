//! Generic `Scheme` type envelopes for Rust-side projections.

mod error;
mod ids;
mod manifest;
mod native_projection;
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
    GerbilSchemeBackedBindingCatalogReceipt, GerbilSchemeTypeFieldSpec, GerbilSchemeTypeManifest,
    GerbilSchemeTypeManifestValidationReceipt, GerbilSchemeTypeSpec,
    GerbilSchemeTypedValueValidationReceipt, decode_gerbil_scheme_type_manifest_fixture,
};
pub use native_projection::{
    GerbilSchemeNativeProjectionReceipt, GerbilSchemeNativeProjectionRequest,
    GerbilSchemeNativeProjectionStatus, decode_gerbil_scheme_native_projection,
    validate_gerbil_scheme_native_projection,
};
pub use package::{
    GerbilSchemeNativeAbiContract, GerbilSchemeNativeAbiReadinessPlan, GerbilSchemePackageManifest,
    GerbilSchemePackageManifestValidationReceipt, GerbilSchemePackageNativeReadinessReceipt,
    decode_gerbil_scheme_package_manifest_fixture, validate_gerbil_scheme_package_manifest,
    validate_gerbil_scheme_package_native_readiness,
};
pub use projection::{GerbilSchemeProjectionContract, GerbilSchemeTypedProjection};
pub use registry::GerbilSchemeTypeRegistry;
pub use typed_value::{
    GerbilSchemeTypedValue, GerbilSchemeValue, decode_gerbil_scheme_typed_value_fixture,
};
pub use validation::{
    gerbil_scheme_backed_binding_catalog, validate_gerbil_scheme_type_manifest,
    validate_gerbil_scheme_typed_value,
};
