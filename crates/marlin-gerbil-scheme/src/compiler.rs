//! Compiler trait for delegating `Gerbil Scheme` projection to Gerbil-owned code.

use crate::{GerbilArtifactKind, GerbilCompiledArtifact};
use serde::{Deserialize, Serialize};

/// Opaque source unit submitted to the `Gerbil` compiler boundary.
///
/// Rust may transport this text to Gerbil-owned code, but it must not parse it
/// into artifacts. Artifact projection crosses as Scheme types -> native ABI ->
/// Rust types.
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
///
/// Implementations must delegate to Gerbil/native ABI projection instead of
/// acting as Rust-side Scheme readers.
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
