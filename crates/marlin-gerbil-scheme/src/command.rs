//! Rust-owned `Gerbil` command request and profile types.

use crate::{
    GerbilArtifactKind, GerbilSource,
    runtime::{GERBIL_ADAPTER_MODULE, GERBIL_LOADPATH_ENV, gerbil_runtime_loadpath},
};
use marlin_gerbil_ir::GerbilWorkspaceContractFacts;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, env, ffi::OsString, path::PathBuf};

/// Environment variable carrying a JSON encoded `GerbilCommandProfile`.
pub const GERBIL_COMMAND_PROFILE_ENV: &str = "MARLIN_GERBIL_COMMAND_PROFILE";

/// Typed Rust request for compiling a `Gerbil` source projection.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilCompileRequest {
    pub source: GerbilSource,
    pub expected: GerbilArtifactKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contract_facts: Option<GerbilWorkspaceContractFacts>,
}

impl GerbilCompileRequest {
    /// Build a compile request with protocol defaults for the expected artifact.
    pub fn new(source: GerbilSource, expected: GerbilArtifactKind) -> Self {
        Self {
            source,
            expected,
            contract_facts: default_contract_facts_for(expected),
        }
    }

    /// Build a compile request with explicit parser-owned Org contract facts.
    pub fn with_contract_facts(
        source: GerbilSource,
        expected: GerbilArtifactKind,
        contract_facts: GerbilWorkspaceContractFacts,
    ) -> Self {
        Self {
            source,
            expected,
            contract_facts: Some(contract_facts),
        }
    }
}

/// Serializable command profile for configuring a `Gerbil` compiler executable.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilCommandProfile {
    pub program: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub current_dir: Option<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
}

impl GerbilCommandProfile {
    pub fn new(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            current_dir: None,
            env: BTreeMap::new(),
        }
    }

    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    pub fn current_dir(mut self, current_dir: impl Into<String>) -> Self {
        self.current_dir = Some(current_dir.into());
        self
    }

    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    /// Builds a profile for the crate-shipped `:marlin/adapter` module entrypoint.
    pub fn marlin_runtime_module(
        program: impl Into<String>,
        loadpath_root: impl Into<PathBuf>,
    ) -> Self {
        let loadpath_root = loadpath_root.into();
        let loadpath = gerbil_runtime_loadpath(&loadpath_root);
        Self::new(program)
            .env(GERBIL_LOADPATH_ENV, loadpath.to_string_lossy().into_owned())
            .arg(GERBIL_ADAPTER_MODULE)
    }

    pub fn from_json(value: &str) -> Result<Self, String> {
        serde_json::from_str(value)
            .map_err(|error| format!("failed to decode gerbil command profile: {error}"))
    }

    pub fn from_env() -> Result<Option<Self>, String> {
        match env::var(GERBIL_COMMAND_PROFILE_ENV) {
            Ok(value) => Self::from_json(&value).map(Some),
            Err(env::VarError::NotPresent) => Ok(None),
            Err(error) => Err(format!(
                "failed to read {GERBIL_COMMAND_PROFILE_ENV} environment variable: {error}"
            )),
        }
    }
}

impl From<GerbilCommandProfile> for GerbilCommandSpec {
    fn from(profile: GerbilCommandProfile) -> Self {
        let mut spec = GerbilCommandSpec::new(profile.program);
        spec.args = profile.args.into_iter().map(OsString::from).collect();
        spec.current_dir = profile.current_dir.map(PathBuf::from);
        spec.env = profile
            .env
            .into_iter()
            .map(|(key, value)| (OsString::from(key), OsString::from(value)))
            .collect();
        spec
    }
}

/// Command used to invoke an external `Gerbil` compiler adapter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilCommandSpec {
    pub program: PathBuf,
    pub args: Vec<OsString>,
    pub current_dir: Option<PathBuf>,
    pub env: BTreeMap<OsString, OsString>,
}

impl GerbilCommandSpec {
    pub fn new(program: impl Into<PathBuf>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            current_dir: None,
            env: BTreeMap::new(),
        }
    }

    pub fn arg(mut self, arg: impl Into<OsString>) -> Self {
        self.args.push(arg.into());
        self
    }

    pub fn current_dir(mut self, current_dir: impl Into<PathBuf>) -> Self {
        self.current_dir = Some(current_dir.into());
        self
    }

    pub fn env(mut self, key: impl Into<OsString>, value: impl Into<OsString>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    /// Builds a command spec for the crate-shipped `:marlin/adapter` module entrypoint.
    pub fn marlin_runtime_module(
        program: impl Into<PathBuf>,
        loadpath_root: impl Into<PathBuf>,
    ) -> Self {
        let loadpath_root = loadpath_root.into();
        let loadpath = gerbil_runtime_loadpath(&loadpath_root);
        Self::new(program)
            .env(GERBIL_LOADPATH_ENV, loadpath.into_os_string())
            .arg(GERBIL_ADAPTER_MODULE)
    }
}

fn default_contract_facts_for(
    expected: GerbilArtifactKind,
) -> Option<GerbilWorkspaceContractFacts> {
    matches!(expected, GerbilArtifactKind::WorkspacePatchIntent)
        .then(GerbilWorkspaceContractFacts::default)
}
