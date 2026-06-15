//! Configuration for native Gerbil Deck runtime AOT artifact planning.

use std::path::PathBuf;

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

/// Configuration for the two-stage native Deck runtime AOT artifact build.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilDeckRuntimeNativeAotConfig {
    pub(super) root: PathBuf,
    pub(super) output_dir: PathBuf,
    pub(super) compiled_runtime_scm: PathBuf,
    pub(super) gsc: PathBuf,
    pub(super) header: PathBuf,
    pub(super) c_compiler: Option<GerbilNativeCCompiler>,
    pub(super) symbol_auditor: GerbilNativeSymbolAuditor,
    pub(super) gambit_link_library: GerbilNativeLinkLibrary,
    pub(super) gambit_link_search_dir: Option<PathBuf>,
}
