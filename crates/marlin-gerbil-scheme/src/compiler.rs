//! Compiler trait for translating `Gerbil Scheme` into typed `IR`.

use crate::{GerbilArtifactKind, GerbilCompiledArtifact};
use serde::{Deserialize, Serialize};

/// Source unit submitted to the `Gerbil` compiler boundary.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilSource {
    pub module: String,
    pub text: String,
}

impl GerbilSource {
    pub fn new(module: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            module: module.into(),
            text: text.into(),
        }
    }
}

/// Compiler boundary that returns typed artifacts for Rust validation.
pub trait GerbilCompiler: Send + Sync {
    fn compile(
        &self,
        source: GerbilSource,
        expected: GerbilArtifactKind,
    ) -> Result<GerbilCompiledArtifact, String>;
}

/// Compile a source unit and enforce the requested artifact class.
pub fn compile_checked<C>(
    compiler: &C,
    source: GerbilSource,
    expected: GerbilArtifactKind,
) -> Result<GerbilCompiledArtifact, String>
where
    C: GerbilCompiler + ?Sized,
{
    compiler
        .compile(source, expected)?
        .ensure_kind(expected)
        .map_err(|error| error.to_string())
}
