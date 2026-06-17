//! AgentGraph readiness replay artifacts for no-execution harness scenarios.

use std::{error::Error, fmt};

use marlin_agent_harness_types::{
    AGENT_HARNESS_SCENARIO_CONTRACT_SCHEMA_ID, AgentHarnessEvidence, AgentHarnessEvidenceKind,
    AgentHarnessScenario, AgentHarnessScenarioContract,
};

/// Stable fixture id for the AgentGraph readiness replay artifact.
pub const AGENT_GRAPH_READINESS_REPLAY_ARTIFACT_ID: &str = "agent-graph-readiness-replay";

/// Serialized scenario contract used by the deterministic AgentGraph readiness fixture.
pub const AGENT_GRAPH_READINESS_REPLAY_CONTRACT_JSON: &str = r#"{
  "schema_id": "marlin.agent.harness_scenario.v1",
  "scenario": {
    "agent_scenario": {
      "id": "agent-graph-readiness-replay",
      "description": "replays AgentGraph planning, runtime projection, and readiness receipts without graph-loop execution",
      "steps": [
        {
          "name": "project-agent-graph-readiness",
          "input": {
            "artifact": "agent-graph-readiness-replay"
          }
        }
      ]
    },
    "expected_evidence": ["Runtime", "Visibility"]
  }
}"#;

/// Typed replay artifact covering AgentGraph planning and readiness receipts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentGraphReadinessReplayArtifact {
    contract: AgentHarnessScenarioContract,
    replay_evidence: Vec<AgentHarnessEvidence>,
}

/// Error returned while loading a serialized AgentGraph readiness contract.
#[derive(Debug)]
pub enum AgentGraphReadinessReplayArtifactLoadError {
    InvalidContract(serde_json::Error),
    UnsupportedSchema { schema_id: String },
}

impl fmt::Display for AgentGraphReadinessReplayArtifactLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidContract(error) => {
                write!(f, "invalid AgentGraph readiness scenario contract: {error}")
            }
            Self::UnsupportedSchema { schema_id } => write!(
                f,
                "unsupported AgentGraph readiness scenario schema: {schema_id} (expected {AGENT_HARNESS_SCENARIO_CONTRACT_SCHEMA_ID})"
            ),
        }
    }
}

impl Error for AgentGraphReadinessReplayArtifactLoadError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidContract(error) => Some(error),
            Self::UnsupportedSchema { .. } => None,
        }
    }
}

impl From<serde_json::Error> for AgentGraphReadinessReplayArtifactLoadError {
    fn from(error: serde_json::Error) -> Self {
        Self::InvalidContract(error)
    }
}

impl AgentGraphReadinessReplayArtifact {
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

/// Load a deterministic AgentGraph readiness replay artifact from a serialized contract.
pub fn load_agent_graph_readiness_replay_artifact(
    serialized_contract: &str,
) -> Result<AgentGraphReadinessReplayArtifact, AgentGraphReadinessReplayArtifactLoadError> {
    let contract: AgentHarnessScenarioContract = serde_json::from_str(serialized_contract)?;
    if !contract.is_supported_schema() {
        return Err(
            AgentGraphReadinessReplayArtifactLoadError::UnsupportedSchema {
                schema_id: contract.schema_id,
            },
        );
    }

    Ok(AgentGraphReadinessReplayArtifact {
        contract,
        replay_evidence: deterministic_agent_graph_readiness_evidence(),
    })
}

/// Deterministic fixture covering AgentGraph readiness without execution.
pub fn agent_graph_readiness_replay_artifact_fixture() -> AgentGraphReadinessReplayArtifact {
    load_agent_graph_readiness_replay_artifact(AGENT_GRAPH_READINESS_REPLAY_CONTRACT_JSON)
        .expect("AgentGraph readiness replay contract fixture should load")
}

fn deterministic_agent_graph_readiness_evidence() -> Vec<AgentHarnessEvidence> {
    vec![
        AgentHarnessEvidence::present(
            AgentHarnessEvidenceKind::Runtime,
            "agent-graph:planning-projection-readiness",
        )
        .with_detail(
            "graph_id=agent-graph.p1 planning_status=Planned projection_status=Projected readiness_status=Ready root_loop_entry=loop.planner graph_loop_execution=false controller_execution=false tool_execution=false live_llm=false",
        ),
        AgentHarnessEvidence::present(
            AgentHarnessEvidenceKind::Visibility,
            "agent-graph:metadata-scope-refs",
        )
        .with_detail(
            "org_memory_scope_ref=memory.scope.project gerbil_policy_scope_ref=policy.scope.implementation memory_query=false policy_program=false session_binding=false worktree_binding=false",
        ),
    ]
}
