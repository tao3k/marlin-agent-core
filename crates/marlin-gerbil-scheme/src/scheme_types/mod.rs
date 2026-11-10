//! Generic `Scheme` type envelopes for Rust-side projections.

mod abi_projection;
mod error;
mod ids;
mod manifest;
mod package;
mod projection;
mod registry;
mod typed_value;
mod validation;

pub use abi_projection::{
    GerbilSchemeAbiProjectionReceipt, GerbilSchemeAbiProjectionRequest,
    GerbilSchemeAbiProjectionStatus, decode_gerbil_scheme_abi_projection,
    validate_gerbil_scheme_abi_projection,
};
pub use error::GerbilSchemeTypeDecodeError;
pub use ids::{
    GerbilSchemeAbiSymbol, GerbilSchemeCAbiId, GerbilSchemeFieldName, GerbilSchemePackageId,
    GerbilSchemeSchemaId, GerbilSchemeTypeId,
};
pub use manifest::{
    GerbilSchemeBackedBindingCatalogReceipt, GerbilSchemeTypeFieldSpec, GerbilSchemeTypeManifest,
    GerbilSchemeTypeManifestValidationReceipt, GerbilSchemeTypeSpec,
    GerbilSchemeTypedValueValidationReceipt, decode_gerbil_scheme_type_manifest_fixture,
};
pub use package::{
    GerbilSchemeCAbiContract, GerbilSchemeCAbiReadinessPlan,
    GerbilSchemePackageAbiReadinessReceipt, GerbilSchemePackageManifest,
    GerbilSchemePackageManifestValidationReceipt, decode_gerbil_scheme_package_manifest_fixture,
    validate_gerbil_scheme_package_abi_readiness, validate_gerbil_scheme_package_manifest,
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
