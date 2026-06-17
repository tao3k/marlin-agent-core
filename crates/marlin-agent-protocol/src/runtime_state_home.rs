//! Runtime-owned state-home layout contracts.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Root directory used by runtime-owned agent state.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeHome {
    pub path: PathBuf,
    pub source: RuntimeHomeSource,
    pub profile: Option<String>,
}

impl RuntimeHome {
    pub fn default(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            source: RuntimeHomeSource::Default,
            profile: None,
        }
    }

    pub fn default_for_user_home(user_home: impl Into<PathBuf>) -> Self {
        Self::default(user_home.into().join(MARLIN_DEFAULT_HOME_DIR_NAME))
    }

    pub fn custom(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            source: RuntimeHomeSource::Custom,
            profile: None,
        }
    }

    pub fn with_profile(mut self, profile: impl Into<String>) -> Self {
        self.profile = Some(profile.into());
        self
    }

    pub fn state_layout(&self) -> RuntimeStateLayout {
        RuntimeStateLayout::standard(self.clone())
    }
}

/// Source of a runtime home path.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RuntimeHomeSource {
    Default,
    Custom,
    InheritedSubAgent { parent_home: PathBuf },
}

/// Environment variable used to override the runtime-owned state home.
pub const MARLIN_HOME_ENV_VAR: &str = "MARLIN_HOME";

/// Default directory name under a user's home directory for runtime-owned state.
pub const MARLIN_DEFAULT_HOME_DIR_NAME: &str = ".marlin";

/// Stable directory kinds inside a runtime state home.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RuntimeStateDirectoryKind {
    Config,
    Cache,
    GraphCache,
    Sessions,
    Workspaces,
    Projects,
    Memory,
    Receipts,
    Logs,
    Tmp,
}

impl RuntimeStateDirectoryKind {
    pub fn relative_path(self) -> &'static str {
        match self {
            Self::Config => "config",
            Self::Cache => "cache",
            Self::GraphCache => "cache/graph",
            Self::Sessions => "sessions",
            Self::Workspaces => "workspaces",
            Self::Projects => "projects",
            Self::Memory => "memory",
            Self::Receipts => "receipts",
            Self::Logs => "logs",
            Self::Tmp => "tmp",
        }
    }
}

/// One resolved runtime state directory.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeStateDirectory {
    pub kind: RuntimeStateDirectoryKind,
    pub path: PathBuf,
}

impl RuntimeStateDirectory {
    pub fn new(home: &RuntimeHome, kind: RuntimeStateDirectoryKind) -> Self {
        Self {
            kind,
            path: home.path.join(kind.relative_path()),
        }
    }
}

/// Unified file layout rooted under the runtime home.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeStateLayout {
    pub home: RuntimeHome,
    #[serde(default)]
    pub directories: Vec<RuntimeStateDirectory>,
}

impl RuntimeStateLayout {
    pub fn standard(home: RuntimeHome) -> Self {
        let directories = standard_state_directory_kinds()
            .into_iter()
            .map(|kind| RuntimeStateDirectory::new(&home, kind))
            .collect();
        Self { home, directories }
    }

    pub fn path_for(&self, kind: RuntimeStateDirectoryKind) -> Option<&PathBuf> {
        self.directories
            .iter()
            .find(|directory| directory.kind == kind)
            .map(|directory| &directory.path)
    }

    pub fn session_path(
        &self,
        key: impl Into<RuntimeStateObjectKey>,
    ) -> Option<RuntimeStateObjectPath> {
        self.object_path(RuntimeStateObjectKind::Session, key)
    }

    pub fn memory_shard_path(
        &self,
        key: impl Into<RuntimeStateObjectKey>,
    ) -> Option<RuntimeStateObjectPath> {
        self.object_path(RuntimeStateObjectKind::MemoryShard, key)
    }

    pub fn receipt_path(
        &self,
        key: impl Into<RuntimeStateObjectKey>,
    ) -> Option<RuntimeStateObjectPath> {
        self.object_path(RuntimeStateObjectKind::Receipt, key)
    }

    pub fn graph_cache_path(
        &self,
        key: impl Into<RuntimeStateObjectKey>,
    ) -> Option<RuntimeStateObjectPath> {
        self.object_path(RuntimeStateObjectKind::GraphCache, key)
    }

    pub fn object_path(
        &self,
        kind: RuntimeStateObjectKind,
        key: impl Into<RuntimeStateObjectKey>,
    ) -> Option<RuntimeStateObjectPath> {
        let key = key.into();
        let file_stem = key.file_stem();
        let base_directory = self.path_for(kind.directory_kind())?;

        Some(RuntimeStateObjectPath {
            kind,
            file_stem: file_stem.clone(),
            path: base_directory.join(file_stem.json_file_name()),
        })
    }
}

/// Runtime state object classes stored below a unified state home.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RuntimeStateObjectKind {
    Session,
    MemoryShard,
    Receipt,
    GraphCache,
}

impl RuntimeStateObjectKind {
    pub fn directory_kind(self) -> RuntimeStateDirectoryKind {
        match self {
            Self::Session => RuntimeStateDirectoryKind::Sessions,
            Self::MemoryShard => RuntimeStateDirectoryKind::Memory,
            Self::Receipt => RuntimeStateDirectoryKind::Receipts,
            Self::GraphCache => RuntimeStateDirectoryKind::GraphCache,
        }
    }
}

/// Caller-provided logical key for one runtime state object.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeStateObjectKey(String);

impl RuntimeStateObjectKey {
    pub fn new(key: impl Into<String>) -> Self {
        Self(key.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    fn file_stem(&self) -> RuntimeStateObjectFileStem {
        RuntimeStateObjectFileStem::from_key(self.as_str())
    }
}

impl From<&str> for RuntimeStateObjectKey {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for RuntimeStateObjectKey {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

/// Filesystem-safe file stem for one runtime state object.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeStateObjectFileStem(String);

impl RuntimeStateObjectFileStem {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub fn json_file_name(&self) -> String {
        format!("{}.json", self.as_str())
    }

    fn from_key(key: &str) -> Self {
        let mut stem = String::new();
        let mut last_was_separator = false;

        for character in key.chars() {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                stem.push(character);
                last_was_separator = false;
            } else if !last_was_separator {
                stem.push('-');
                last_was_separator = true;
            }
        }

        let stem = stem.trim_matches(|character| matches!(character, '.' | '-' | '_'));
        if stem.is_empty() {
            Self("unnamed".to_owned())
        } else {
            Self(stem.to_owned())
        }
    }
}

/// Resolved file path for one runtime state object without embedding its body.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeStateObjectPath {
    pub kind: RuntimeStateObjectKind,
    pub file_stem: RuntimeStateObjectFileStem,
    pub path: PathBuf,
}

/// Status for runtime state-home storage initialization.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RuntimeStateStorageStatus {
    Planned,
    Initialized,
    Failed,
}

/// Receipt describing state-home layout initialization without embedding state bodies.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeStateStorageReceipt {
    pub home: RuntimeHome,
    pub status: RuntimeStateStorageStatus,
    #[serde(default)]
    pub directories: Vec<RuntimeStateDirectory>,
    pub reason: Option<String>,
}

impl RuntimeStateStorageReceipt {
    pub fn planned(layout: &RuntimeStateLayout) -> Self {
        Self {
            home: layout.home.clone(),
            status: RuntimeStateStorageStatus::Planned,
            directories: layout.directories.clone(),
            reason: None,
        }
    }

    pub fn initialized(layout: &RuntimeStateLayout) -> Self {
        Self {
            home: layout.home.clone(),
            status: RuntimeStateStorageStatus::Initialized,
            directories: layout.directories.clone(),
            reason: None,
        }
    }

    pub fn failed(home: RuntimeHome, reason: impl Into<String>) -> Self {
        Self {
            home,
            status: RuntimeStateStorageStatus::Failed,
            directories: Vec::new(),
            reason: Some(reason.into()),
        }
    }
}

fn standard_state_directory_kinds() -> Vec<RuntimeStateDirectoryKind> {
    vec![
        RuntimeStateDirectoryKind::Config,
        RuntimeStateDirectoryKind::Cache,
        RuntimeStateDirectoryKind::GraphCache,
        RuntimeStateDirectoryKind::Sessions,
        RuntimeStateDirectoryKind::Workspaces,
        RuntimeStateDirectoryKind::Projects,
        RuntimeStateDirectoryKind::Memory,
        RuntimeStateDirectoryKind::Receipts,
        RuntimeStateDirectoryKind::Logs,
        RuntimeStateDirectoryKind::Tmp,
    ]
}
