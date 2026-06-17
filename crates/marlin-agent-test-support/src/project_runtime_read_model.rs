//! Project-runtime read-model replay artifacts for no-LLM harness scenarios.

use std::{error::Error, fmt};

use marlin_agent_harness_types::{
    AGENT_HARNESS_SCENARIO_CONTRACT_SCHEMA_ID, AgentHarnessEvidence, AgentHarnessEvidenceKind,
    AgentHarnessScenario, AgentHarnessScenarioContract,
};

/// Stable fixture id for the project-runtime read-model replay artifact.
pub const PROJECT_RUNTIME_READ_MODEL_REPLAY_ARTIFACT_ID: &str = "project-runtime-read-model-replay";

/// Serialized scenario contract used by the deterministic read-model fixture.
pub const PROJECT_RUNTIME_READ_MODEL_REPLAY_CONTRACT_JSON: &str = r#"{
  "schema_id": "marlin.agent.harness_scenario.v1",
  "scenario": {
    "agent_scenario": {
      "id": "project-runtime-read-model-replay",
      "description": "replays project-runtime read-model receipts without live LLMs or sandbox execution",
      "steps": [
        {
          "name": "load-project-runtime-read-model",
          "input": {
            "artifact": "project-runtime-read-model-replay"
          }
        }
      ]
    },
    "expected_evidence": ["Visibility", "Runtime", "Tool", "Content"]
  }
}"#;

/// Typed replay artifact covering project-runtime graph read-model receipts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectRuntimeReadModelReplayArtifact {
    contract: AgentHarnessScenarioContract,
    replay_evidence: Vec<AgentHarnessEvidence>,
}

/// Error returned while loading a serialized project-runtime read-model contract.
#[derive(Debug)]
pub enum ProjectRuntimeReadModelReplayArtifactLoadError {
    InvalidContract(serde_json::Error),
    UnsupportedSchema { schema_id: String },
}

impl fmt::Display for ProjectRuntimeReadModelReplayArtifactLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidContract(error) => {
                write!(
                    f,
                    "invalid project-runtime read-model scenario contract: {error}"
                )
            }
            Self::UnsupportedSchema { schema_id } => write!(
                f,
                "unsupported project-runtime read-model scenario schema: {schema_id} (expected {AGENT_HARNESS_SCENARIO_CONTRACT_SCHEMA_ID})"
            ),
        }
    }
}

impl Error for ProjectRuntimeReadModelReplayArtifactLoadError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidContract(error) => Some(error),
            Self::UnsupportedSchema { .. } => None,
        }
    }
}

impl From<serde_json::Error> for ProjectRuntimeReadModelReplayArtifactLoadError {
    fn from(error: serde_json::Error) -> Self {
        Self::InvalidContract(error)
    }
}

impl ProjectRuntimeReadModelReplayArtifact {
    /// Serialized scenario contract represented by this artifact.
    pub fn contract(&self) -> &AgentHarnessScenarioContract {
        &self.contract
    }

    /// Scenario used by harness validation.
    pub fn scenario(&self) -> &AgentHarnessScenario {
        &self.contract.scenario
    }

    /// Evidence replayed by harness tests.
    pub fn replay_evidence(&self) -> &[AgentHarnessEvidence] {
        &self.replay_evidence
    }

    /// Consume the artifact into its scenario contract and replay evidence.
    pub fn into_parts(self) -> (AgentHarnessScenarioContract, Vec<AgentHarnessEvidence>) {
        (self.contract, self.replay_evidence)
    }
}

/// Load a deterministic read-model replay artifact from a serialized contract.
pub fn load_project_runtime_read_model_replay_artifact(
    serialized_contract: &str,
) -> Result<ProjectRuntimeReadModelReplayArtifact, ProjectRuntimeReadModelReplayArtifactLoadError> {
    let contract: AgentHarnessScenarioContract = serde_json::from_str(serialized_contract)?;
    if !contract.is_supported_schema() {
        return Err(
            ProjectRuntimeReadModelReplayArtifactLoadError::UnsupportedSchema {
                schema_id: contract.schema_id,
            },
        );
    }

    Ok(ProjectRuntimeReadModelReplayArtifact {
        contract,
        replay_evidence: deterministic_project_runtime_read_model_evidence(),
    })
}

/// Deterministic fixture covering project-runtime graph read families.
pub fn project_runtime_read_model_replay_artifact_fixture() -> ProjectRuntimeReadModelReplayArtifact
{
    load_project_runtime_read_model_replay_artifact(PROJECT_RUNTIME_READ_MODEL_REPLAY_CONTRACT_JSON)
        .expect("project-runtime read-model replay contract fixture should load")
}

fn deterministic_project_runtime_read_model_evidence() -> Vec<AgentHarnessEvidence> {
    vec![
        AgentHarnessEvidence::present(
            AgentHarnessEvidenceKind::Visibility,
            "project-runtime:memory-visibility",
        )
        .with_detail(
            "family=Memory same_project=true cross_worktree=true sibling_transcript_hidden=true external_project_denied=true source_anchor_projection=true",
        ),
        AgentHarnessEvidence::present(
            AgentHarnessEvidenceKind::Visibility,
            "project-runtime:session-transcript-isolation",
        )
        .with_detail(
            "family=Session raw_sibling_transcript=false context_pack_bounded=true content_anchor=content:turn-7 source_session_id=session-a",
        ),
        AgentHarnessEvidence::present(
            AgentHarnessEvidenceKind::Content,
            "project-runtime:child-content-fork",
        )
        .with_detail(
            "family=Content content_id=content:summary-a parent_content_id=content:turn-7 source_agent_id=agent:reviewer source_anchor_projection=true",
        ),
        AgentHarnessEvidence::present(
            AgentHarnessEvidenceKind::Content,
            "project-runtime:turn-context-steering",
        )
        .with_detail(
            "turn_id=turn:review-8 context_pack_id=context-pack:review selected_item_ids=[content:summary-a,memory:runtime-1] omitted_reasons=[VisibilityDenied,LowerRanked] memory_citation_ids=[citation:memory-runtime-1] source_anchor_ids=[anchor:summary-a,anchor:memory-runtime-1] graph_query_receipt_ids=[receipt:memory-query-1] whole_memory_shard=false raw_transcript=false",
        ),
        AgentHarnessEvidence::present(
            AgentHarnessEvidenceKind::Content,
            "project-runtime:memory-trigger-receipt",
        )
        .with_detail(
            "content_id=content:summary-a memory_id=memory:runtime-1 status=Committed candidate_score=8700 turn_id=turn:review-8 context_pack_id=context-pack:review steering_receipt_id=receipt:steer-review memory_citation_id=citation:memory-runtime-1 source_anchor_ids=[anchor:memory-runtime-1] graph_query_receipt_ids=[receipt:memory-query-1] whole_memory_shard=false raw_transcript=false",
        ),
        AgentHarnessEvidence::present(
            AgentHarnessEvidenceKind::Content,
            "project-runtime:loaded-node-memory-context-pack",
        )
        .with_detail(
            "context_pack_id=context-pack:loaded-node-memory memory_id=memory:missing-source source_anchor_id=memory-node:missing-source source_span_absent=true store_discovery=false explicit_shard_plumbing=false whole_memory_shard=false",
        ),
        AgentHarnessEvidence::present(
            AgentHarnessEvidenceKind::Content,
            "project-runtime:turn-context-item-view",
        )
        .with_detail(
            "receipt_id=receipt:item-view context_pack_id=context-pack:loaded-node-memory item_kinds=[ProjectMemory,ToolCapability] memory_citation_ids=[citation:memory-runtime-1] source_anchor_ids=[memory-node:missing-source,tool-node:rustfmt-card] required_receipt_ids=[receipt:format-check,receipt:workspace-clean] backend_requirement_ids=[backend:process-sandbox,backend:macos-compatible] raw_transcript=false whole_memory_shard=false sandbox_backend_selection=false",
        ),
        AgentHarnessEvidence::present(
            AgentHarnessEvidenceKind::Tool,
            "project-runtime:tool-capability-store",
        )
        .with_detail(
            "family=Tool root_kind=ToolCapability cli_root=--org-tool-root capability_id=tool:rustfmt required_receipts=receipt:format-check",
        ),
        AgentHarnessEvidence::present(
            AgentHarnessEvidenceKind::Tool,
            "project-runtime:tool-capability-card-requirements",
        )
        .with_detail(
            "capability_id=tool:rustfmt required_receipt_ids=[receipt:format-check,receipt:workspace-clean] required_capability_ids=[tool:workspace-status] isolation_requirement_ids=[isolation:write-worktree] backend_requirement_ids=[backend:process-sandbox,backend:macos-compatible] sandbox_backend_selection=false",
        ),
        AgentHarnessEvidence::present(
            AgentHarnessEvidenceKind::Runtime,
            "project-runtime:debug-graph-query-readout",
        )
        .with_detail(
            "families=[Memory,Tool,Session,Content] summary_fields=[source_project_ids,source_session_ids,source_agent_ids,source_anchor_ids,memory_ids,content_ids,tool_capability_ids] live_llm=false sandbox_execution=false",
        ),
        AgentHarnessEvidence::present(
            AgentHarnessEvidenceKind::Runtime,
            "project-runtime:negative-cli-gates",
        )
        .with_detail(
            "missing_store_roots=true fixture_store_conflicts=true unsupported_org_fixture=true stale_root_disambiguation=covered missing_source_span_regression=covered loaded_node_recall=true",
        ),
    ]
}
