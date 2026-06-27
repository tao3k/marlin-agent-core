//! Error contract for intent-case artifact materialization.

use std::{error::Error, fmt, path::PathBuf};

use marlin_agent_harness_types::{
    IntentCaseArtifactId, IntentCaseArtifactKind, IntentCaseTraceEntryId,
};

/// Error returned when an intent-case artifact bundle cannot be materialized.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IntentCaseArtifactBundleMaterializationError {
    EmptyManifest,
    MissingArtifactPath {
        artifact_id: IntentCaseArtifactId,
    },
    UnsafeArtifactPath {
        artifact_id: IntentCaseArtifactId,
        path: String,
    },
    ExecutionTraceMismatch {
        trace_entries: usize,
        execution_steps: usize,
    },
    MissingTraceRuntimeOwner {
        trace_id: IntentCaseTraceEntryId,
    },
    UnknownTraceArtifactRef {
        trace_id: IntentCaseTraceEntryId,
        artifact_id: IntentCaseArtifactId,
    },
    EmptyTraceCorrelationIndex,
    IncompleteArtifactBundle {
        missing_artifacts: Vec<IntentCaseArtifactKind>,
    },
    Io {
        path: PathBuf,
        message: String,
    },
}

impl fmt::Display for IntentCaseArtifactBundleMaterializationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyManifest => formatter.write_str("intent-case manifest has no artifacts"),
            Self::MissingArtifactPath { artifact_id } => {
                write!(
                    formatter,
                    "artifact {artifact_id} is present but has no path"
                )
            }
            Self::UnsafeArtifactPath { artifact_id, path } => {
                write!(formatter, "artifact {artifact_id} has unsafe path {path:?}")
            }
            Self::ExecutionTraceMismatch {
                trace_entries,
                execution_steps,
            } => write!(
                formatter,
                "trace index has {trace_entries} entries but execution receipt has {execution_steps} steps"
            ),
            Self::MissingTraceRuntimeOwner { trace_id } => {
                write!(formatter, "trace entry {trace_id} has no runtime owner")
            }
            Self::UnknownTraceArtifactRef {
                trace_id,
                artifact_id,
            } => write!(
                formatter,
                "trace entry {trace_id} references unknown artifact {artifact_id}"
            ),
            Self::EmptyTraceCorrelationIndex => {
                formatter.write_str("intent-case trace correlation index is empty")
            }
            Self::IncompleteArtifactBundle { missing_artifacts } => {
                write!(
                    formatter,
                    "intent-case artifact bundle is incomplete; missing artifact kinds: {missing_artifacts:?}"
                )
            }
            Self::Io { path, message } => write!(formatter, "write {}: {message}", path.display()),
        }
    }
}

impl Error for IntentCaseArtifactBundleMaterializationError {}
