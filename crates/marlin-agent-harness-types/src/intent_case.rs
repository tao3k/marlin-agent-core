//! Intent-case artifact bundle DTOs shared by loop/runtime harnesses.

use serde::{Deserialize, Serialize};

/// Stable schema id for serialized intent-case artifact manifests.
pub const INTENT_CASE_ARTIFACT_MANIFEST_SCHEMA_ID: &str = "marlin.intent-case.artifact-manifest.v1";
/// Stable schema id for intent-case artifact completeness receipts.
pub const INTENT_CASE_ARTIFACT_COMPLETENESS_RECEIPT_SCHEMA_ID: &str =
    "marlin.intent-case.artifact-completeness-receipt.v1";
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
    IntentCaseModelInvocationId,
    "Stable id for one model invocation inside an intent-case run."
);
define_intent_case_string_id!(
    IntentCaseToolCallId,
    "Stable id for one tool call inside an intent-case run."
);
define_intent_case_string_id!(
    IntentCaseResourceKey,
    "Stable resource key selected by one runtime/tool action inside an intent-case run."
);
define_intent_case_string_id!(
    IntentCaseSandboxProfile,
    "Stable sandbox profile selected by one runtime/tool action inside an intent-case run."
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
define_intent_case_string_id!(
    IntentCaseTraceAction,
    "Stable action label recorded for one intent-case trace entry."
);
define_intent_case_string_id!(
    IntentCaseTraceEvent,
    "Stable event label recorded for one intent-case trace entry."
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
    pub model_invocation_id: Option<IntentCaseModelInvocationId>,
    pub tool_call_id: Option<IntentCaseToolCallId>,
    pub resource_key: Option<IntentCaseResourceKey>,
    pub sandbox_profile: Option<IntentCaseSandboxProfile>,
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
            model_invocation_id: None,
            tool_call_id: None,
            resource_key: None,
            sandbox_profile: None,
            artifact_refs: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_runtime_owner(mut self, runtime_owner: impl Into<IntentCaseRuntimeOwner>) -> Self {
        self.runtime_owner = Some(runtime_owner.into());
        self
    }

    #[must_use]
    pub fn with_model_invocation_id(
        mut self,
        model_invocation_id: impl Into<IntentCaseModelInvocationId>,
    ) -> Self {
        self.model_invocation_id = Some(model_invocation_id.into());
        self
    }

    #[must_use]
    pub fn with_tool_call_id(mut self, tool_call_id: impl Into<IntentCaseToolCallId>) -> Self {
        self.tool_call_id = Some(tool_call_id.into());
        self
    }

    #[must_use]
    pub fn with_resource_key(mut self, resource_key: impl Into<IntentCaseResourceKey>) -> Self {
        self.resource_key = Some(resource_key.into());
        self
    }

    #[must_use]
    pub fn with_sandbox_profile(
        mut self,
        sandbox_profile: impl Into<IntentCaseSandboxProfile>,
    ) -> Self {
        self.sandbox_profile = Some(sandbox_profile.into());
        self
    }

    #[must_use]
    pub fn with_artifact_ref(mut self, artifact_id: impl Into<IntentCaseArtifactId>) -> Self {
        self.artifact_refs.push(artifact_id.into());
        self
    }
}

/// Fully qualified correlation key for one trace-step to artifact edge.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct IntentCaseCorrelationKey {
    pub case_id: IntentCaseId,
    pub run_id: IntentCaseRunId,
    pub policy_epoch: u64,
    pub policy_digest: IntentCasePolicyDigest,
    pub loop_program_id: IntentCaseLoopProgramId,
    pub trace_id: IntentCaseTraceEntryId,
    pub step_index: u64,
    pub transition_id: IntentCaseTransitionId,
    pub action: IntentCaseTraceAction,
    pub event: IntentCaseTraceEvent,
    pub runtime_owner: IntentCaseRuntimeOwner,
    pub model_invocation_id: Option<IntentCaseModelInvocationId>,
    pub tool_call_id: Option<IntentCaseToolCallId>,
    pub resource_key: Option<IntentCaseResourceKey>,
    pub sandbox_profile: Option<IntentCaseSandboxProfile>,
    pub artifact_id: IntentCaseArtifactId,
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
    pub fn present_artifact_kinds(&self) -> Vec<IntentCaseArtifactKind> {
        let mut kinds = self
            .artifacts
            .iter()
            .filter(|artifact| artifact.present)
            .map(|artifact| artifact.kind)
            .collect::<Vec<_>>();
        kinds.sort();
        kinds.dedup();
        kinds
    }

    #[must_use]
    pub fn has_artifact_id(&self, artifact_id: &IntentCaseArtifactId) -> bool {
        self.artifacts
            .iter()
            .any(|artifact| &artifact.artifact_id == artifact_id)
    }

    #[must_use]
    pub fn has_present_artifact_id(&self, artifact_id: &IntentCaseArtifactId) -> bool {
        self.artifacts
            .iter()
            .any(|artifact| &artifact.artifact_id == artifact_id && artifact.present)
    }

    #[must_use]
    pub fn trace_artifact_ref_missing_ids(&self) -> Vec<IntentCaseArtifactId> {
        let mut missing = self
            .trace_index
            .entries
            .iter()
            .flat_map(|entry| entry.artifact_refs.iter())
            .filter(|artifact_id| !self.has_present_artifact_id(artifact_id))
            .cloned()
            .collect::<Vec<_>>();
        missing.sort();
        missing.dedup();
        missing
    }

    #[must_use]
    pub fn trace_entries_without_runtime_owner(&self) -> Vec<IntentCaseTraceEntryId> {
        self.trace_index
            .entries
            .iter()
            .filter(|entry| entry.runtime_owner.is_none())
            .map(|entry| entry.trace_id.clone())
            .collect()
    }

    #[must_use]
    pub fn trace_entries_without_action_identity(&self) -> Vec<IntentCaseTraceEntryId> {
        self.trace_index
            .entries
            .iter()
            .filter(|entry| {
                (entry.action == "invoke_model" && entry.model_invocation_id.is_none())
                    || (entry.action == "dispatch_tools" && entry.tool_call_id.is_none())
                    || (entry.action == "dispatch_tools" && entry.resource_key.is_none())
                    || (entry.action == "dispatch_tools" && entry.sandbox_profile.is_none())
            })
            .map(|entry| entry.trace_id.clone())
            .collect()
    }

    #[must_use]
    pub fn correlation_keys(&self) -> Vec<IntentCaseCorrelationKey> {
        self.trace_index
            .entries
            .iter()
            .filter_map(|entry| {
                entry
                    .runtime_owner
                    .clone()
                    .map(|runtime_owner| (entry, runtime_owner))
            })
            .flat_map(|(entry, runtime_owner)| {
                entry.artifact_refs.iter().cloned().map(move |artifact_id| {
                    IntentCaseCorrelationKey {
                        case_id: self.case_id.clone(),
                        run_id: self.run_id.clone(),
                        policy_epoch: self.policy_epoch,
                        policy_digest: self.policy_digest.clone(),
                        loop_program_id: self.loop_program_id.clone(),
                        trace_id: entry.trace_id.clone(),
                        step_index: entry.step_index,
                        transition_id: entry.transition_id.clone(),
                        action: IntentCaseTraceAction::new(entry.action.clone()),
                        event: IntentCaseTraceEvent::new(entry.event.clone()),
                        runtime_owner: runtime_owner.clone(),
                        model_invocation_id: entry.model_invocation_id.clone(),
                        tool_call_id: entry.tool_call_id.clone(),
                        resource_key: entry.resource_key.clone(),
                        sandbox_profile: entry.sandbox_profile.clone(),
                        artifact_id,
                    }
                })
            })
            .collect()
    }

    #[must_use]
    pub fn has_complete_trace_correlation(&self) -> bool {
        !self.trace_index.is_empty()
            && !self.correlation_keys().is_empty()
            && self.trace_artifact_ref_missing_ids().is_empty()
            && self.trace_entries_without_runtime_owner().is_empty()
            && self.trace_entries_without_action_identity().is_empty()
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

/// Completeness status for one materialized intent-case artifact bundle.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IntentCaseArtifactCompletenessStatus {
    Complete,
    Incomplete,
}

impl IntentCaseArtifactCompletenessStatus {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Incomplete => "incomplete",
        }
    }
}

/// Receipt proving expected intent-case artifact lanes were materialized.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct IntentCaseArtifactCompletenessReceipt {
    #[serde(default = "default_intent_case_artifact_completeness_receipt_schema_id")]
    pub schema_id: String,
    pub case_id: IntentCaseId,
    pub run_id: IntentCaseRunId,
    pub policy_digest: IntentCasePolicyDigest,
    pub loop_program_id: IntentCaseLoopProgramId,
    pub expected_artifacts: Vec<IntentCaseArtifactKind>,
    pub materialized_artifacts: Vec<IntentCaseArtifactKind>,
    pub missing_artifacts: Vec<IntentCaseArtifactKind>,
    pub trace_entry_count: usize,
    pub correlation_key_count: usize,
    pub status: IntentCaseArtifactCompletenessStatus,
}

impl IntentCaseArtifactCompletenessReceipt {
    #[must_use]
    pub fn from_manifest_and_materialized_artifacts(
        manifest: &IntentCaseArtifactManifest,
        materialized_artifacts: impl IntoIterator<Item = IntentCaseArtifactKind>,
    ) -> Self {
        let expected_artifacts = manifest.present_artifact_kinds();
        let mut materialized_artifacts = materialized_artifacts.into_iter().collect::<Vec<_>>();
        materialized_artifacts.sort();
        materialized_artifacts.dedup();
        let missing_artifacts = expected_artifacts
            .iter()
            .copied()
            .filter(|kind| !materialized_artifacts.contains(kind))
            .collect::<Vec<_>>();
        let status = if missing_artifacts.is_empty() && manifest.has_complete_trace_correlation() {
            IntentCaseArtifactCompletenessStatus::Complete
        } else {
            IntentCaseArtifactCompletenessStatus::Incomplete
        };

        Self {
            schema_id: INTENT_CASE_ARTIFACT_COMPLETENESS_RECEIPT_SCHEMA_ID.to_owned(),
            case_id: manifest.case_id.clone(),
            run_id: manifest.run_id.clone(),
            policy_digest: manifest.policy_digest.clone(),
            loop_program_id: manifest.loop_program_id.clone(),
            expected_artifacts,
            materialized_artifacts,
            missing_artifacts,
            trace_entry_count: manifest.trace_index.entries.len(),
            correlation_key_count: manifest.correlation_keys().len(),
            status,
        }
    }

    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.status == IntentCaseArtifactCompletenessStatus::Complete
    }

    #[must_use]
    pub fn is_supported_schema(&self) -> bool {
        self.schema_id == INTENT_CASE_ARTIFACT_COMPLETENESS_RECEIPT_SCHEMA_ID
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

fn default_intent_case_artifact_completeness_receipt_schema_id() -> String {
    INTENT_CASE_ARTIFACT_COMPLETENESS_RECEIPT_SCHEMA_ID.to_owned()
}

fn default_intent_case_run_receipt_schema_id() -> String {
    INTENT_CASE_RUN_RECEIPT_SCHEMA_ID.to_owned()
}
