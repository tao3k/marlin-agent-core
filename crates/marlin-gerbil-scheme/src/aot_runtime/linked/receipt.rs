//! Receipt types for native Gerbil runtime AOT artifact planning.

use super::{
    config::{
        GerbilCCompiler, GerbilDeckRuntimeAotProfile, GerbilLinkLibrary, GerbilSymbolAuditor,
    },
    status::{GerbilDeckRuntimeAotBuildStatus, GerbilDeckRuntimeAotStatus},
};
use crate::{
    GerbilSchemeAbiSymbol, GerbilSchemeCAbiContract, GerbilSchemeCAbiId,
    GerbilSchemeCAbiReadinessPlan,
};
use marlin_agent_protocol::{GraphCAbiReadinessReceipt, GraphCAbiRequirement};
use std::path::PathBuf;

/// Exported C symbol name owned by the native Deck runtime ABI.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilDeckRouteAbiSymbol(String);

impl GerbilDeckRouteAbiSymbol {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&GerbilDeckRouteAbiSymbol> for GerbilSchemeAbiSymbol {
    fn from(symbol: &GerbilDeckRouteAbiSymbol) -> Self {
        Self::new(symbol.as_str())
    }
}

/// Program and argv for one native AOT compiler phase.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilDeckRuntimeAotCommandPlan {
    pub program: PathBuf,
    pub args: Vec<String>,
}

/// Captured command result for a native AOT compiler phase.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilDeckRuntimeAotCommandReceipt {
    pub status_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

/// Mechanism used to verify required symbols on generated native objects.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GerbilDeckRouteAbiSymbolAuditMethod {
    ObjectFiles,
    SymbolTableCommand,
}

/// Typed plan for producing a Rust-linkable native Gerbil runtime link unit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilDeckRuntimeAotPlan {
    pub profile: GerbilDeckRuntimeAotProfile,
    pub status: GerbilDeckRuntimeAotStatus,
    pub root: PathBuf,
    pub output_dir: PathBuf,
    pub compiled_runtime_scm: PathBuf,
    pub compiled_runtime_dependency_scms: Vec<PathBuf>,
    pub header: PathBuf,
    pub object: PathBuf,
    pub dependency_objects: Vec<PathBuf>,
    pub module_objects: Vec<PathBuf>,
    pub link_c_source: PathBuf,
    pub link_object: PathBuf,
    pub exported_symbols: Vec<GerbilDeckRouteAbiSymbol>,
    pub c_compiler: Option<GerbilCCompiler>,
    pub symbol_auditor: GerbilSymbolAuditor,
    pub gambit_link_library: GerbilLinkLibrary,
    pub gambit_link_search_dir: Option<PathBuf>,
    pub gsc_compile_object: GerbilDeckRuntimeAotCommandPlan,
    pub gsc_compile_dependency_objects: Vec<GerbilDeckRuntimeAotCommandPlan>,
    pub gsc_generate_link_source: GerbilDeckRuntimeAotCommandPlan,
    pub gsc_compile_link_object: GerbilDeckRuntimeAotCommandPlan,
    pub audit_symbols: GerbilDeckRuntimeAotCommandPlan,
    pub detail: Option<String>,
}

impl GerbilDeckRuntimeAotPlan {
    /// Convert this native build plan into a Scheme package native ABI contract.
    pub fn scheme_c_abi_contract(&self) -> GerbilSchemeCAbiContract {
        GerbilSchemeCAbiContract::new(
            GerbilSchemeCAbiId::new(self.profile.abi_id()),
            self.profile.abi_version(),
        )
        .with_exported_symbols(self.scheme_abi_symbols())
    }

    /// Convert this native build plan into a graph-loop native ABI requirement.
    pub fn graph_c_abi_requirement(&self) -> GraphCAbiRequirement {
        self.scheme_c_abi_contract().graph_c_abi_requirement()
    }

    /// Convert this native build plan into a Scheme package readiness plan.
    pub fn scheme_c_abi_readiness_plan(&self) -> GerbilSchemeCAbiReadinessPlan {
        GerbilSchemeCAbiReadinessPlan::new(
            GerbilSchemeCAbiId::new(self.profile.abi_id()),
            self.profile.abi_version(),
        )
        .with_exported_symbols(self.scheme_abi_symbols())
    }

    fn scheme_abi_symbols(&self) -> impl Iterator<Item = GerbilSchemeAbiSymbol> + '_ {
        self.exported_symbols
            .iter()
            .map(GerbilSchemeAbiSymbol::from)
    }
}

/// Structured result for executing a native Deck runtime AOT link-unit build.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilDeckRuntimeAotBuildReceipt {
    pub status: GerbilDeckRuntimeAotBuildStatus,
    pub plan: GerbilDeckRuntimeAotPlan,
    pub detail: Option<String>,
    pub gsc_compile_object: Option<GerbilDeckRuntimeAotCommandReceipt>,
    pub gsc_compile_dependency_objects: Vec<GerbilDeckRuntimeAotCommandReceipt>,
    pub gsc_generate_link_source: Option<GerbilDeckRuntimeAotCommandReceipt>,
    pub gsc_compile_link_object: Option<GerbilDeckRuntimeAotCommandReceipt>,
    pub symbol_audit_method: Option<GerbilDeckRouteAbiSymbolAuditMethod>,
    pub symbol_audit: Option<GerbilDeckRuntimeAotCommandReceipt>,
    pub missing_symbols: Vec<GerbilDeckRouteAbiSymbol>,
}

impl GerbilDeckRuntimeAotBuildReceipt {
    /// Project the executed native AOT build into the graph-loop ABI readiness gate.
    pub fn graph_c_abi_readiness_receipt(&self) -> GraphCAbiReadinessReceipt {
        let requirement = self.plan.graph_c_abi_requirement();
        let available_symbols = match self.status {
            GerbilDeckRuntimeAotBuildStatus::LinkUnitReady => self
                .plan
                .exported_symbols
                .iter()
                .map(|symbol| symbol.as_str().to_owned())
                .collect::<Vec<_>>(),
            GerbilDeckRuntimeAotBuildStatus::RequiredSymbolsMissing => self
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

        GraphCAbiReadinessReceipt::evaluate(&requirement, available_symbols)
    }
}
