//! Receipt types for native Gerbil Deck runtime AOT artifact planning.

use super::{
    config::{GerbilNativeCCompiler, GerbilNativeLinkLibrary, GerbilNativeSymbolAuditor},
    status::{GerbilDeckRuntimeNativeAotBuildStatus, GerbilDeckRuntimeNativeAotStatus},
};
use std::path::PathBuf;

/// Exported C symbol name owned by the native Deck runtime ABI.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilDeckRuntimeNativeSymbol(String);

impl GerbilDeckRuntimeNativeSymbol {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Program and argv for one native AOT compiler phase.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilDeckRuntimeNativeAotCommandPlan {
    pub program: PathBuf,
    pub args: Vec<String>,
}

/// Captured command result for a native AOT compiler phase.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilDeckRuntimeNativeAotCommandReceipt {
    pub status_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

/// Typed plan for producing a Rust-linkable native Deck runtime link unit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilDeckRuntimeNativeAotPlan {
    pub status: GerbilDeckRuntimeNativeAotStatus,
    pub root: PathBuf,
    pub output_dir: PathBuf,
    pub scheme_source: PathBuf,
    pub header: PathBuf,
    pub generated_loader_scm: PathBuf,
    pub generated_runtime_scm: PathBuf,
    pub generated_ssi: PathBuf,
    pub generated_ssxi: PathBuf,
    pub static_scm: PathBuf,
    pub object: PathBuf,
    pub link_c_source: PathBuf,
    pub link_object: PathBuf,
    pub exported_symbols: Vec<GerbilDeckRuntimeNativeSymbol>,
    pub c_compiler: Option<GerbilNativeCCompiler>,
    pub symbol_auditor: GerbilNativeSymbolAuditor,
    pub gambit_link_library: GerbilNativeLinkLibrary,
    pub gambit_link_search_dir: Option<PathBuf>,
    pub gxc_generate_scheme: GerbilDeckRuntimeNativeAotCommandPlan,
    pub gsc_compile_object: GerbilDeckRuntimeNativeAotCommandPlan,
    pub gsc_generate_link_source: GerbilDeckRuntimeNativeAotCommandPlan,
    pub gsc_compile_link_object: GerbilDeckRuntimeNativeAotCommandPlan,
    pub audit_symbols: GerbilDeckRuntimeNativeAotCommandPlan,
    pub detail: Option<String>,
}

/// Structured result for executing a native Deck runtime AOT link-unit build.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilDeckRuntimeNativeAotBuildReceipt {
    pub status: GerbilDeckRuntimeNativeAotBuildStatus,
    pub plan: GerbilDeckRuntimeNativeAotPlan,
    pub detail: Option<String>,
    pub gxc_generate_scheme: Option<GerbilDeckRuntimeNativeAotCommandReceipt>,
    pub gsc_compile_object: Option<GerbilDeckRuntimeNativeAotCommandReceipt>,
    pub gsc_generate_link_source: Option<GerbilDeckRuntimeNativeAotCommandReceipt>,
    pub gsc_compile_link_object: Option<GerbilDeckRuntimeNativeAotCommandReceipt>,
    pub symbol_audit: Option<GerbilDeckRuntimeNativeAotCommandReceipt>,
    pub missing_symbols: Vec<GerbilDeckRuntimeNativeSymbol>,
}
