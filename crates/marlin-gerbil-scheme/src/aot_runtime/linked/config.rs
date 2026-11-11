//! Configuration for native Gerbil runtime AOT artifact planning.

use std::path::PathBuf;

use crate::{
    GERBIL_AGENT_POLICY_ROUTING_ABI_HEADER_PATH, GERBIL_AGENT_POLICY_ROUTING_ABI_ID,
    GERBIL_AGENT_POLICY_ROUTING_ABI_VERSION,
};

use super::paths::{
    GERBIL_AGENT_POLICY_ROUTING_ABI_ARTIFACT_STEM,
    GERBIL_AGENT_POLICY_ROUTING_ABI_INITIALIZE_SYMBOL,
    GERBIL_AGENT_POLICY_ROUTING_ABI_SELECT_SYMBOL,
};

/// C compiler program passed through Gambit `gsc -cc`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilCCompiler(String);

impl GerbilCCompiler {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Native library name required when linking Gambit-generated objects.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilLinkLibrary(String);

impl GerbilLinkLibrary {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Symbol table auditor used to verify exported native ABI symbols.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilSymbolAuditor(PathBuf);

impl GerbilSymbolAuditor {
    pub fn new(value: impl Into<PathBuf>) -> Self {
        Self(value.into())
    }

    pub fn as_path(&self) -> &std::path::Path {
        &self.0
    }
}

/// Native Gerbil AOT unit compiled into a Rust-linkable C ABI link unit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GerbilAotProfile {
    AgentPolicyRouting,
}

impl GerbilAotProfile {
    pub(super) const fn abi_id(self) -> &'static str {
        match self {
            Self::AgentPolicyRouting => GERBIL_AGENT_POLICY_ROUTING_ABI_ID,
        }
    }

    pub(super) const fn abi_version(self) -> u32 {
        match self {
            Self::AgentPolicyRouting => GERBIL_AGENT_POLICY_ROUTING_ABI_VERSION,
        }
    }

    pub(super) const fn artifact_stem(self) -> &'static str {
        match self {
            Self::AgentPolicyRouting => GERBIL_AGENT_POLICY_ROUTING_ABI_ARTIFACT_STEM,
        }
    }

    pub(super) const fn header_path(self) -> &'static str {
        match self {
            Self::AgentPolicyRouting => GERBIL_AGENT_POLICY_ROUTING_ABI_HEADER_PATH,
        }
    }

    pub(super) const fn initialize_symbol(self) -> &'static str {
        match self {
            Self::AgentPolicyRouting => GERBIL_AGENT_POLICY_ROUTING_ABI_INITIALIZE_SYMBOL,
        }
    }

    pub(super) const fn select_symbol(self) -> &'static str {
        match self {
            Self::AgentPolicyRouting => GERBIL_AGENT_POLICY_ROUTING_ABI_SELECT_SYMBOL,
        }
    }

    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::AgentPolicyRouting => "AgentGraph policy routing",
        }
    }
}

/// Configuration for a two-stage native Gerbil runtime AOT artifact build.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilAotConfig {
    pub(super) profile: GerbilAotProfile,
    pub(super) root: PathBuf,
    pub(super) output_dir: PathBuf,
    pub(super) compiled_runtime_scm: PathBuf,
    pub(super) compiled_runtime_dependency_scms: Vec<PathBuf>,
    pub(super) gsc: PathBuf,
    pub(super) header: PathBuf,
    pub(super) c_compiler: Option<GerbilCCompiler>,
    pub(super) symbol_auditor: GerbilSymbolAuditor,
    pub(super) gambit_link_library: GerbilLinkLibrary,
    pub(super) gambit_link_search_dir: Option<PathBuf>,
}
