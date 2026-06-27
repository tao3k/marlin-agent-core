//! Intent-case artifact bundle DTOs shared by loop/runtime harnesses.

use serde::{Deserialize, Serialize};

/// Stable schema id for serialized intent-case artifact manifests.
pub const INTENT_CASE_ARTIFACT_MANIFEST_SCHEMA_ID: &str = "marlin.intent-case.artifact-manifest.v1";
/// Stable schema id for serialized intent-case run receipts.
pub const INTENT_CASE_RUN_RECEIPT_SCHEMA_ID: &str = "marlin.intent-case.run-receipt.v1";

macro_rules! define_intent_case_string_id {
    ($type_name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $type_name(String);

        impl $type_name {
            #[must_use]
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl From<&str> for $type_name {
            fn from(value: &str) -> Self {
                Self::new(value)
            }
        }

        impl From<String> for $type_name {
            fn from(value: String) -> Self {
                Self::new(value)
            }
        }

        impl std::fmt::Display for $type_name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str(self.as_str())
            }
        }
    };
}

define_intent_case_string_id!(IntentCaseId, "Stable id for one engineering intent case.");
define_intent_case_string_id!(IntentCaseRunId, "Stable id for one intent-case run.");
define_intent_case_string_id!(
    IntentCaseArtifactId,
    "Stable id for one artifact inside an intent-case run bundle."
);
define_intent_case_string_id!(
    IntentCasePolicyDigest,
    "Stable digest for the policy pack that produced an intent-case run."
);
define_intent_case_string_id!(
    IntentCaseLoopProgramId,
    "Stable id for the LoopProgram that drove an intent-case run."
);
define_intent_case_string_id!(
    IntentCaseTraceEntryId,
    "Stable id for one trace index entry inside an intent-case run."
);
define_intent_case_string_id!(
    IntentCaseTransitionId,
    "Stable id for one transition recorded in an intent-case trace index."
);
define_intent_case_string_id!(
    IntentCaseRuntimeOwner,
    "Stable runtime owner recorded for one intent-case trace entry."
);

/// Artifact lane expected in a complete intent-case run bundle.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IntentCaseArtifactKind {
    Intent,
    PolicyPack,
    LoopProgram,
    VerticalTrace,
    ExecutionTrace,
    ModelEvents,
    ToolCalls,
    SandboxReceipts,
    MemoryReceipts,
    DiffPatch,
    TestBefore,
    TestAfter,
    VerifierReceipt,
    PolicyExplanation,
    ReplayScript,
}

/// Presence and location metadata for one artifact in an intent-case bundle.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct IntentCaseArtifactRef {
    pub artifact_id: IntentCaseArtifactId,
    pub kind: IntentCaseArtifactKind,
    pub present: bool,
    pub path: Option<String>,
    pub content_digest: Option<String>,
}

impl IntentCaseArtifactRef {
    #[must_use]
    pub fn present(
        artifact_id: impl Into<IntentCaseArtifactId>,
        kind: IntentCaseArtifactKind,
        path: impl Into<String>,
    ) -> Self {
        Self {
            artifact_id: artifact_id.into(),
            kind,
            present: true,
            path: Some(path.into()),
            content_digest: None,
        }
    }

    #[must_use]
    pub fn missing(
        artifact_id: impl Into<IntentCaseArtifactId>,
        kind: IntentCaseArtifactKind,
    ) -> Self {
        Self {
            artifact_id: artifact_id.into(),
            kind,
            present: false,
            path: None,
            content_digest: None,
        }
    }

    #[must_use]
    pub fn with_content_digest(mut self, content_digest: impl Into<String>) -> Self {
        self.content_digest = Some(content_digest.into());
        self
    }
}

/// Named request for constructing one trace index entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IntentCaseTraceEntryRequest {
    pub trace_id: IntentCaseTraceEntryId,
    pub step_index: u64,
    pub transition_id: IntentCaseTransitionId,
    pub action: String,
    pub event: String,
}

/// Correlation keys for one runtime trace step.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct IntentCaseTraceEntry {
    pub trace_id: IntentCaseTraceEntryId,
    pub step_index: u64,
    pub transition_id: IntentCaseTransitionId,
    pub action: String,
    pub event: String,
    pub runtime_owner: Option<IntentCaseRuntimeOwner>,
    pub artifact_refs: Vec<IntentCaseArtifactId>,
}

impl IntentCaseTraceEntry {
    #[must_use]
    pub fn from_request(request: IntentCaseTraceEntryRequest) -> Self {
        Self {
            trace_id: request.trace_id,
            step_index: request.step_index,
            transition_id: request.transition_id,
            action: request.action,
            event: request.event,
            runtime_owner: None,
            artifact_refs: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_runtime_owner(mut self, runtime_owner: impl Into<IntentCaseRuntimeOwner>) -> Self {
        self.runtime_owner = Some(runtime_owner.into());
        self
    }

    #[must_use]
    pub fn with_artifact_ref(mut self, artifact_id: impl Into<IntentCaseArtifactId>) -> Self {
        self.artifact_refs.push(artifact_id.into());
        self
    }
}

/// Ordered trace index for one intent-case run.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct IntentCaseTraceIndex {
    pub entries: Vec<IntentCaseTraceEntry>,
}

impl IntentCaseTraceIndex {
    #[must_use]
    pub fn new(entries: impl Into<Vec<IntentCaseTraceEntry>>) -> Self {
        Self {
            entries: entries.into(),
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Named request for constructing one intent-case manifest.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IntentCaseArtifactManifestRequest {
    pub case_id: IntentCaseId,
    pub run_id: IntentCaseRunId,
    pub policy_epoch: u64,
    pub policy_digest: IntentCasePolicyDigest,
    pub loop_program_id: IntentCaseLoopProgramId,
}

/// Manifest for all artifacts and trace keys emitted by one intent-case run.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct IntentCaseArtifactManifest {
    #[serde(default = "default_intent_case_artifact_manifest_schema_id")]
    pub schema_id: String,
    pub case_id: IntentCaseId,
    pub run_id: IntentCaseRunId,
    pub policy_epoch: u64,
    pub policy_digest: IntentCasePolicyDigest,
    pub loop_program_id: IntentCaseLoopProgramId,
    pub artifacts: Vec<IntentCaseArtifactRef>,
    pub trace_index: IntentCaseTraceIndex,
}

impl IntentCaseArtifactManifest {
    #[must_use]
    pub fn from_request(request: IntentCaseArtifactManifestRequest) -> Self {
        Self {
            schema_id: INTENT_CASE_ARTIFACT_MANIFEST_SCHEMA_ID.to_owned(),
            case_id: request.case_id,
            run_id: request.run_id,
            policy_epoch: request.policy_epoch,
            policy_digest: request.policy_digest,
            loop_program_id: request.loop_program_id,
            artifacts: Vec::new(),
            trace_index: IntentCaseTraceIndex::default(),
        }
    }

    #[must_use]
    pub fn with_artifact(mut self, artifact: IntentCaseArtifactRef) -> Self {
        self.artifacts.push(artifact);
        self
    }

    #[must_use]
    pub fn with_trace_index(mut self, trace_index: IntentCaseTraceIndex) -> Self {
        self.trace_index = trace_index;
        self
    }

    #[must_use]
    pub fn has_artifact_kind(&self, kind: IntentCaseArtifactKind) -> bool {
        self.artifacts
            .iter()
            .any(|artifact| artifact.kind == kind && artifact.present)
    }

    #[must_use]
    pub fn has_core_artifact_bundle(&self) -> bool {
        [
            IntentCaseArtifactKind::PolicyPack,
            IntentCaseArtifactKind::LoopProgram,
            IntentCaseArtifactKind::VerticalTrace,
            IntentCaseArtifactKind::ExecutionTrace,
            IntentCaseArtifactKind::ReplayScript,
        ]
        .into_iter()
        .all(|kind| self.has_artifact_kind(kind))
    }

    #[must_use]
    pub fn is_supported_schema(&self) -> bool {
        self.schema_id == INTENT_CASE_ARTIFACT_MANIFEST_SCHEMA_ID
    }
}

/// Terminal status for one intent-case run receipt.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IntentCaseRunStatus {
    Passed,
    Failed,
    Incomplete,
}

/// Run receipt tying an intent-case artifact manifest to validation outcome.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct IntentCaseRunReceipt {
    #[serde(default = "default_intent_case_run_receipt_schema_id")]
    pub schema_id: String,
    pub manifest: IntentCaseArtifactManifest,
    pub status: IntentCaseRunStatus,
    pub diagnostics: Vec<String>,
}

impl IntentCaseRunReceipt {
    #[must_use]
    pub fn passed(manifest: IntentCaseArtifactManifest) -> Self {
        Self {
            schema_id: INTENT_CASE_RUN_RECEIPT_SCHEMA_ID.to_owned(),
            manifest,
            status: IntentCaseRunStatus::Passed,
            diagnostics: Vec::new(),
        }
    }

    #[must_use]
    pub fn incomplete(
        manifest: IntentCaseArtifactManifest,
        diagnostics: impl Into<Vec<String>>,
    ) -> Self {
        Self {
            schema_id: INTENT_CASE_RUN_RECEIPT_SCHEMA_ID.to_owned(),
            manifest,
            status: IntentCaseRunStatus::Incomplete,
            diagnostics: diagnostics.into(),
        }
    }

    #[must_use]
    pub fn failed(
        manifest: IntentCaseArtifactManifest,
        diagnostics: impl Into<Vec<String>>,
    ) -> Self {
        Self {
            schema_id: INTENT_CASE_RUN_RECEIPT_SCHEMA_ID.to_owned(),
            manifest,
            status: IntentCaseRunStatus::Failed,
            diagnostics: diagnostics.into(),
        }
    }

    #[must_use]
    pub fn is_supported_schema(&self) -> bool {
        self.schema_id == INTENT_CASE_RUN_RECEIPT_SCHEMA_ID
    }
}

fn default_intent_case_artifact_manifest_schema_id() -> String {
    INTENT_CASE_ARTIFACT_MANIFEST_SCHEMA_ID.to_owned()
}

fn default_intent_case_run_receipt_schema_id() -> String {
    INTENT_CASE_RUN_RECEIPT_SCHEMA_ID.to_owned()
}
