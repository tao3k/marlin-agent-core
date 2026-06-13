//! Replay artifacts for no-LLM runtime evidence chains.

use std::{error::Error, fmt};

use marlin_agent_protocol::{
    AGENT_SCENARIO_CONTRACT_SCHEMA_ID, AgentScenario, AgentScenarioContract, LoopEvidence,
};
use marlin_agent_sessions::SessionKind;

use crate::{
    accepted_graph_policy_proposal_fixture, custom_hook_policy_receipt_fixture,
    custom_sub_agent_start_hook_summary_fixture, hook_dispatch_replay_evidence,
    sub_agent_hook_dispatch_selection_fixture, sub_agent_memory_denied_fixture,
    sub_agent_memory_session_replay_evidence, sub_agent_memory_session_visibility_evidence,
};

/// Stable fixture id for the no-LLM runtime replay artifact.
pub const NO_LLM_RUNTIME_REPLAY_ARTIFACT_ID: &str = "no-llm-runtime-replay";

/// Serialized scenario contract used by the deterministic no-LLM replay fixture.
pub const NO_LLM_RUNTIME_REPLAY_CONTRACT_JSON: &str = r#"{
  "schema_id": "marlin.agent.scenario.v1",
  "scenario": {
    "id": "no-llm-runtime-replay",
    "description": "replays graph, sub-agent session, and hook receipts without live LLMs",
    "steps": [
      {
        "name": "load-replay-artifact",
        "input": {
          "artifact": "no-llm-runtime-replay"
        }
      }
    ],
    "expected_evidence": ["Visibility", "Runtime"]
  }
}"#;

/// Typed replay artifact loaded by harness tests without touching live LLMs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NoLlmRuntimeReplayArtifact {
    contract: AgentScenarioContract,
    replay_evidence: Vec<LoopEvidence>,
}

/// Error returned while loading a serialized no-LLM replay artifact contract.
#[derive(Debug)]
pub enum NoLlmRuntimeReplayArtifactLoadError {
    InvalidContract(serde_json::Error),
    UnsupportedSchema { schema_id: String },
}

impl fmt::Display for NoLlmRuntimeReplayArtifactLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidContract(error) => {
                write!(f, "invalid no-LLM replay scenario contract: {error}")
            }
            Self::UnsupportedSchema { schema_id } => write!(
                f,
                "unsupported no-LLM replay scenario schema: {schema_id} (expected {AGENT_SCENARIO_CONTRACT_SCHEMA_ID})"
            ),
        }
    }
}

impl Error for NoLlmRuntimeReplayArtifactLoadError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidContract(error) => Some(error),
            Self::UnsupportedSchema { .. } => None,
        }
    }
}

impl From<serde_json::Error> for NoLlmRuntimeReplayArtifactLoadError {
    fn from(error: serde_json::Error) -> Self {
        Self::InvalidContract(error)
    }
}

impl NoLlmRuntimeReplayArtifact {
    /// Serialized scenario contract represented by this artifact.
    pub fn contract(&self) -> &AgentScenarioContract {
        &self.contract
    }

    /// Scenario used by harness validation.
    pub fn scenario(&self) -> &AgentScenario {
        &self.contract.scenario
    }

    /// Evidence replayed before dynamic harness execution.
    pub fn replay_evidence(&self) -> &[LoopEvidence] {
        &self.replay_evidence
    }

    /// Consume the artifact into its scenario contract and replay evidence.
    pub fn into_parts(self) -> (AgentScenarioContract, Vec<LoopEvidence>) {
        (self.contract, self.replay_evidence)
    }
}

/// Load a deterministic no-LLM replay artifact from a serialized scenario contract.
pub fn load_no_llm_runtime_replay_artifact(
    serialized_contract: &str,
) -> Result<NoLlmRuntimeReplayArtifact, NoLlmRuntimeReplayArtifactLoadError> {
    let contract: AgentScenarioContract = serde_json::from_str(serialized_contract)?;
    if !contract.is_supported_schema() {
        return Err(NoLlmRuntimeReplayArtifactLoadError::UnsupportedSchema {
            schema_id: contract.schema_id,
        });
    }

    Ok(NoLlmRuntimeReplayArtifact {
        contract,
        replay_evidence: deterministic_no_llm_runtime_replay_evidence(),
    })
}

/// Deterministic replay artifact fixture covering graph, session, and hook receipts.
pub fn no_llm_runtime_replay_artifact_fixture() -> NoLlmRuntimeReplayArtifact {
    load_no_llm_runtime_replay_artifact(NO_LLM_RUNTIME_REPLAY_CONTRACT_JSON)
        .expect("no-LLM runtime replay contract fixture should load")
}

fn deterministic_no_llm_runtime_replay_evidence() -> Vec<LoopEvidence> {
    let memory_fixture = sub_agent_memory_denied_fixture();
    let (child_session, isolation_receipt) = memory_fixture.parent_session().child_session(
        SessionKind::SubAgent,
        memory_fixture.config().child_session_id(),
        memory_fixture.requested_visibility(),
    );
    let hook_summary = custom_sub_agent_start_hook_summary_fixture();
    let hook_selection = sub_agent_hook_dispatch_selection_fixture();
    let hook_policy = custom_hook_policy_receipt_fixture();
    let graph_policy = accepted_graph_policy_proposal_fixture();

    vec![
        graph_policy.visibility_evidence(),
        sub_agent_memory_session_visibility_evidence(&child_session, &isolation_receipt),
        sub_agent_memory_session_replay_evidence(&child_session, &isolation_receipt),
        hook_dispatch_replay_evidence(&hook_summary, &hook_selection, &hook_policy),
    ]
}
