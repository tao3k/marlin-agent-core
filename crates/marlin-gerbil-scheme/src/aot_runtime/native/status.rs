//! Status values for native Gerbil Deck runtime AOT artifact planning.

use serde::{Deserialize, Serialize};

/// Readiness status for a native Deck runtime AOT artifact plan.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum GerbilDeckRuntimeNativeAotStatus {
    MissingGxc,
    MissingGsc,
    MissingSchemeSource,
    MissingHeader,
    ReadyToBuildLinkUnit,
}

/// Execution status for a native Deck runtime AOT link-unit build.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum GerbilDeckRuntimeNativeAotBuildStatus {
    MissingGxc,
    MissingGsc,
    MissingHeader,
    AssetWriteFailed,
    OutputDirCreateFailed,
    GxcGenerateSchemeFailed,
    GeneratedSchemeMissing,
    GscCompileObjectFailed,
    ObjectMissing,
    GscGenerateLinkSourceFailed,
    LinkSourceMissing,
    GscCompileLinkObjectFailed,
    LinkObjectMissing,
    SymbolAuditFailed,
    RequiredSymbolsMissing,
    LinkUnitReady,
}

/// Readiness status for consuming a native Deck runtime link unit from Rust.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum GerbilDeckRuntimeNativeStaticLinkStatus {
    LinkUnitNotReady,
    Ready,
}
