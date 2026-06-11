//! `Gerbil Scheme` compiler boundary for typed `marlin` artifacts.

mod artifact;
mod command;
mod compiler;

pub use artifact::{GerbilArtifactKind, GerbilArtifactKindMismatch, GerbilCompiledArtifact};
pub use command::{
    GERBIL_COMMAND_PROFILE_ENV, GerbilCommandCompiler, GerbilCommandProfile, GerbilCommandSpec,
    GerbilCompileRequest, GerbilCompileResponse,
};
pub use compiler::{GerbilCompiler, GerbilSource, compile_checked};
