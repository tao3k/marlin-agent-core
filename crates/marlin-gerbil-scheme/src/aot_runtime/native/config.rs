//! Configuration for native Gerbil runtime AOT artifact planning.

use std::path::PathBuf;

use crate::{
    GERBIL_AGENT_POLICY_ROUTING_NATIVE_ABI_ID, GERBIL_AGENT_POLICY_ROUTING_NATIVE_ABI_VERSION,
    GERBIL_AGENT_POLICY_ROUTING_NATIVE_HEADER_PATH, GERBIL_DECK_RUNTIME_NATIVE_ABI_ID,
    GERBIL_DECK_RUNTIME_NATIVE_ABI_VERSION, GERBIL_DECK_RUNTIME_NATIVE_HEADER_PATH,
};

use super::paths::{
    GERBIL_AGENT_POLICY_ROUTING_NATIVE_ARTIFACT_STEM,
    GERBIL_AGENT_POLICY_ROUTING_NATIVE_BINDING_ARTIFACT_STEM,
    GERBIL_AGENT_POLICY_ROUTING_NATIVE_INITIALIZE_SYMBOL,
    GERBIL_AGENT_POLICY_ROUTING_NATIVE_SELECT_SYMBOL, GERBIL_DECK_RUNTIME_NATIVE_ARTIFACT_STEM,
    GERBIL_DECK_RUNTIME_NATIVE_BINDING_ARTIFACT_STEM, GERBIL_DECK_RUNTIME_NATIVE_INITIALIZE_SYMBOL,
    GERBIL_DECK_RUNTIME_NATIVE_SELECT_SYMBOL,
};

/// C compiler program passed through Gambit `gsc -cc`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilNativeCCompiler(String);

impl GerbilNativeCCompiler {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Native library name required when linking Gambit-generated objects.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilNativeLinkLibrary(String);

impl GerbilNativeLinkLibrary {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Symbol table auditor used to verify exported native ABI symbols.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilNativeSymbolAuditor(PathBuf);

impl GerbilNativeSymbolAuditor {
    pub fn new(value: impl Into<PathBuf>) -> Self {
        Self(value.into())
    }

    pub fn as_path(&self) -> &std::path::Path {
        &self.0
    }
}

/// Native Gerbil AOT unit compiled into a Rust-linkable C ABI link unit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GerbilDeckRuntimeNativeAotProfile {
    DeckRuntime,
    AgentPolicyRouting,
}

impl GerbilDeckRuntimeNativeAotProfile {
    pub(super) const fn abi_id(self) -> &'static str {
        match self {
            Self::DeckRuntime => GERBIL_DECK_RUNTIME_NATIVE_ABI_ID,
            Self::AgentPolicyRouting => GERBIL_AGENT_POLICY_ROUTING_NATIVE_ABI_ID,
        }
    }

    pub(super) const fn abi_version(self) -> u32 {
        match self {
            Self::DeckRuntime => GERBIL_DECK_RUNTIME_NATIVE_ABI_VERSION,
            Self::AgentPolicyRouting => GERBIL_AGENT_POLICY_ROUTING_NATIVE_ABI_VERSION,
        }
    }

    pub(super) const fn artifact_stem(self) -> &'static str {
        match self {
            Self::DeckRuntime => GERBIL_DECK_RUNTIME_NATIVE_ARTIFACT_STEM,
            Self::AgentPolicyRouting => GERBIL_AGENT_POLICY_ROUTING_NATIVE_ARTIFACT_STEM,
        }
    }

    pub(super) const fn dependency_artifact_stems(self) -> &'static [&'static str] {
        match self {
            Self::DeckRuntime => &[GERBIL_DECK_RUNTIME_NATIVE_BINDING_ARTIFACT_STEM],
            Self::AgentPolicyRouting => &[GERBIL_AGENT_POLICY_ROUTING_NATIVE_BINDING_ARTIFACT_STEM],
        }
    }

    pub(super) const fn header_path(self) -> &'static str {
        match self {
            Self::DeckRuntime => GERBIL_DECK_RUNTIME_NATIVE_HEADER_PATH,
            Self::AgentPolicyRouting => GERBIL_AGENT_POLICY_ROUTING_NATIVE_HEADER_PATH,
        }
    }

    pub(super) const fn initialize_symbol(self) -> &'static str {
        match self {
            Self::DeckRuntime => GERBIL_DECK_RUNTIME_NATIVE_INITIALIZE_SYMBOL,
            Self::AgentPolicyRouting => GERBIL_AGENT_POLICY_ROUTING_NATIVE_INITIALIZE_SYMBOL,
        }
    }

    pub(super) const fn select_symbol(self) -> &'static str {
        match self {
            Self::DeckRuntime => GERBIL_DECK_RUNTIME_NATIVE_SELECT_SYMBOL,
            Self::AgentPolicyRouting => GERBIL_AGENT_POLICY_ROUTING_NATIVE_SELECT_SYMBOL,
        }
    }

    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::DeckRuntime => "Deck runtime",
            Self::AgentPolicyRouting => "AgentGraph policy routing",
        }
    }
}

/// Configuration for a two-stage native Gerbil runtime AOT artifact build.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilDeckRuntimeNativeAotConfig {
    pub(super) profile: GerbilDeckRuntimeNativeAotProfile,
    pub(super) root: PathBuf,
    pub(super) output_dir: PathBuf,
    pub(super) compiled_runtime_scm: PathBuf,
    pub(super) compiled_runtime_dependency_scms: Vec<PathBuf>,
    pub(super) gsc: PathBuf,
    pub(super) header: PathBuf,
    pub(super) c_compiler: Option<GerbilNativeCCompiler>,
    pub(super) symbol_auditor: GerbilNativeSymbolAuditor,
    pub(super) gambit_link_library: GerbilNativeLinkLibrary,
    pub(super) gambit_link_search_dir: Option<PathBuf>,
}
