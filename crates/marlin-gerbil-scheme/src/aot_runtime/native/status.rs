//! Status values for native Gerbil Deck runtime AOT artifact planning.

use serde::{Deserialize, Serialize};

/// Readiness status for a native Deck runtime AOT artifact plan.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum GerbilDeckRuntimeNativeAotStatus {
    MissingGsc,
    MissingCompiledRuntime,
    MissingHeader,
    ReadyToBuildLinkUnit,
}

/// Execution status for a native Deck runtime AOT link-unit build.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum GerbilDeckRuntimeNativeAotBuildStatus {
    MissingGsc,
    MissingHeader,
    MissingCompiledRuntime,
    OutputDirCreateFailed,
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
