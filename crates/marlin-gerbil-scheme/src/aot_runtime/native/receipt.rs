//! Receipt types for native Gerbil runtime AOT artifact planning.

use super::{
    config::{
        GerbilDeckRuntimeNativeAotProfile, GerbilNativeCCompiler, GerbilNativeLinkLibrary,
        GerbilNativeSymbolAuditor,
    },
    status::{GerbilDeckRuntimeNativeAotBuildStatus, GerbilDeckRuntimeNativeAotStatus},
};
use crate::{
    GerbilSchemeNativeAbiContract, GerbilSchemeNativeAbiId, GerbilSchemeNativeAbiReadinessPlan,
    GerbilSchemeNativeSymbol,
};
use marlin_agent_protocol::{GraphNativeAbiReadinessReceipt, GraphNativeAbiRequirement};
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

impl From<&GerbilDeckRuntimeNativeSymbol> for GerbilSchemeNativeSymbol {
    fn from(symbol: &GerbilDeckRuntimeNativeSymbol) -> Self {
        Self::new(symbol.as_str())
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

/// Mechanism used to verify required symbols on generated native objects.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GerbilDeckRuntimeNativeSymbolAuditMethod {
    ObjectFiles,
    SymbolTableCommand,
}

/// Typed plan for producing a Rust-linkable native Gerbil runtime link unit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilDeckRuntimeNativeAotPlan {
    pub profile: GerbilDeckRuntimeNativeAotProfile,
    pub status: GerbilDeckRuntimeNativeAotStatus,
    pub root: PathBuf,
    pub output_dir: PathBuf,
    pub compiled_runtime_scm: PathBuf,
    pub header: PathBuf,
    pub object: PathBuf,
    pub link_c_source: PathBuf,
    pub link_object: PathBuf,
    pub exported_symbols: Vec<GerbilDeckRuntimeNativeSymbol>,
    pub c_compiler: Option<GerbilNativeCCompiler>,
    pub symbol_auditor: GerbilNativeSymbolAuditor,
    pub gambit_link_library: GerbilNativeLinkLibrary,
    pub gambit_link_search_dir: Option<PathBuf>,
    pub gsc_compile_object: GerbilDeckRuntimeNativeAotCommandPlan,
    pub gsc_generate_link_source: GerbilDeckRuntimeNativeAotCommandPlan,
    pub gsc_compile_link_object: GerbilDeckRuntimeNativeAotCommandPlan,
    pub audit_symbols: GerbilDeckRuntimeNativeAotCommandPlan,
    pub detail: Option<String>,
}

impl GerbilDeckRuntimeNativeAotPlan {
    /// Convert this native build plan into a Scheme package native ABI contract.
    pub fn scheme_native_abi_contract(&self) -> GerbilSchemeNativeAbiContract {
        GerbilSchemeNativeAbiContract::new(
            GerbilSchemeNativeAbiId::new(self.profile.abi_id()),
            self.profile.abi_version(),
        )
        .with_exported_symbols(self.scheme_native_symbols())
    }

    /// Convert this native build plan into a graph-loop native ABI requirement.
    pub fn graph_native_abi_requirement(&self) -> GraphNativeAbiRequirement {
        self.scheme_native_abi_contract()
            .graph_native_abi_requirement()
    }

    /// Convert this native build plan into a Scheme package readiness plan.
    pub fn scheme_native_abi_readiness_plan(&self) -> GerbilSchemeNativeAbiReadinessPlan {
        GerbilSchemeNativeAbiReadinessPlan::new(
            GerbilSchemeNativeAbiId::new(self.profile.abi_id()),
            self.profile.abi_version(),
        )
        .with_exported_symbols(self.scheme_native_symbols())
    }

    fn scheme_native_symbols(&self) -> impl Iterator<Item = GerbilSchemeNativeSymbol> + '_ {
        self.exported_symbols
            .iter()
            .map(GerbilSchemeNativeSymbol::from)
    }
}

/// Structured result for executing a native Deck runtime AOT link-unit build.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilDeckRuntimeNativeAotBuildReceipt {
    pub status: GerbilDeckRuntimeNativeAotBuildStatus,
    pub plan: GerbilDeckRuntimeNativeAotPlan,
    pub detail: Option<String>,
    pub gsc_compile_object: Option<GerbilDeckRuntimeNativeAotCommandReceipt>,
    pub gsc_generate_link_source: Option<GerbilDeckRuntimeNativeAotCommandReceipt>,
    pub gsc_compile_link_object: Option<GerbilDeckRuntimeNativeAotCommandReceipt>,
    pub symbol_audit_method: Option<GerbilDeckRuntimeNativeSymbolAuditMethod>,
    pub symbol_audit: Option<GerbilDeckRuntimeNativeAotCommandReceipt>,
    pub missing_symbols: Vec<GerbilDeckRuntimeNativeSymbol>,
}

impl GerbilDeckRuntimeNativeAotBuildReceipt {
    /// Project the executed native AOT build into the graph-loop ABI readiness gate.
    pub fn graph_native_abi_readiness_receipt(&self) -> GraphNativeAbiReadinessReceipt {
        let requirement = self.plan.graph_native_abi_requirement();
        let available_symbols = match self.status {
            GerbilDeckRuntimeNativeAotBuildStatus::LinkUnitReady => self
                .plan
                .exported_symbols
                .iter()
                .map(|symbol| symbol.as_str().to_owned())
                .collect::<Vec<_>>(),
            GerbilDeckRuntimeNativeAotBuildStatus::RequiredSymbolsMissing => self
                .plan
                .exported_symbols
                .iter()
                .filter(|symbol| {
                    !self
                        .missing_symbols
                        .iter()
                        .any(|missing| missing == *symbol)
                })
                .map(|symbol| symbol.as_str().to_owned())
                .collect::<Vec<_>>(),
            _ => Vec::new(),
        };

        GraphNativeAbiReadinessReceipt::evaluate(&requirement, available_symbols)
    }
}
