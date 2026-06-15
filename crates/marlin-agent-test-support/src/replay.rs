//! Replay artifacts for no-LLM runtime evidence chains.

use std::{error::Error, fmt};

use marlin_agent_harness_types::{
    AGENT_HARNESS_SCENARIO_CONTRACT_SCHEMA_ID, AgentHarnessEvidence, AgentHarnessEvidenceKind,
    AgentHarnessScenario, AgentHarnessScenarioContract,
};
use marlin_agent_protocol::{
    GraphNodeExecutionReceipt, GraphNodeExecutionStatus, ModelEndpoint, ModelGatewayRequest,
    ModelGatewayTransport, user_gateway_message,
};
use marlin_agent_sessions::SessionKind;

use crate::{
    NoLiveLlmModelGateway, complex_gerbil_graph_policy_replay_fixture,
    custom_hook_policy_receipt_fixture, custom_sub_agent_start_hook_summary_fixture,
    hook_dispatch_replay_evidence, no_live_llm_gateway_denial_evidence,
    sub_agent_hook_dispatch_selection_fixture, sub_agent_memory_denied_fixture,
    sub_agent_memory_session_replay_evidence, sub_agent_memory_session_visibility_evidence,
};

/// Stable fixture id for the no-LLM runtime replay artifact.
pub const NO_LLM_RUNTIME_REPLAY_ARTIFACT_ID: &str = "no-llm-runtime-replay";

/// Serialized scenario contract used by the deterministic no-LLM replay fixture.
pub const NO_LLM_RUNTIME_REPLAY_CONTRACT_JSON: &str = r#"{
  "schema_id": "marlin.agent.harness_scenario.v1",
  "scenario": {
    "agent_scenario": {
      "id": "no-llm-runtime-replay",
      "description": "replays graph, sub-agent session, and hook receipts without live LLMs",
      "steps": [
        {
          "name": "load-replay-artifact",
          "input": {
            "artifact": "no-llm-runtime-replay"
          }
        }
      ]
    },
    "expected_evidence": ["Visibility", "Runtime"]
  }
}"#;

/// Typed replay artifact loaded by harness tests without touching live LLMs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NoLlmRuntimeReplayArtifact {
    contract: AgentHarnessScenarioContract,
    replay_evidence: Vec<AgentHarnessEvidence>,
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
                "unsupported no-LLM replay scenario schema: {schema_id} (expected {AGENT_HARNESS_SCENARIO_CONTRACT_SCHEMA_ID})"
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
    pub fn contract(&self) -> &AgentHarnessScenarioContract {
        &self.contract
    }

    /// Scenario used by harness validation.
    pub fn scenario(&self) -> &AgentHarnessScenario {
        &self.contract.scenario
    }

    /// Evidence replayed before dynamic harness execution.
    pub fn replay_evidence(&self) -> &[AgentHarnessEvidence] {
        &self.replay_evidence
    }

    /// Consume the artifact into its scenario contract and replay evidence.
    pub fn into_parts(self) -> (AgentHarnessScenarioContract, Vec<AgentHarnessEvidence>) {
        (self.contract, self.replay_evidence)
    }
}

/// Load a deterministic no-LLM replay artifact from a serialized scenario contract.
pub fn load_no_llm_runtime_replay_artifact(
    serialized_contract: &str,
) -> Result<NoLlmRuntimeReplayArtifact, NoLlmRuntimeReplayArtifactLoadError> {
    let contract: AgentHarnessScenarioContract = serde_json::from_str(serialized_contract)?;
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

fn deterministic_no_llm_runtime_replay_evidence() -> Vec<AgentHarnessEvidence> {
    let memory_fixture = sub_agent_memory_denied_fixture();
    let (child_session, isolation_receipt) = memory_fixture.parent_session().child_session(
        SessionKind::SubAgent,
        memory_fixture.config().child_session_id(),
        memory_fixture.requested_visibility(),
    );
    let hook_summary = custom_sub_agent_start_hook_summary_fixture();
    let hook_selection = sub_agent_hook_dispatch_selection_fixture();
    let hook_policy = custom_hook_policy_receipt_fixture();
    let graph_policy = complex_gerbil_graph_policy_replay_fixture();
    let no_live_llm_gateway = NoLiveLlmModelGateway::new();
    let _denial = no_live_llm_gateway.deny_completion_attempt(
        ModelGatewayRequest::new(
            ModelEndpoint::new("anthropic", "claude-opus-4-8"),
            vec![user_gateway_message("no live LLM replay probe")],
        )
        .with_transport(ModelGatewayTransport::Auto),
    );
    let no_live_llm_gateway_evidence = no_live_llm_gateway_denial_evidence(
        NO_LLM_RUNTIME_REPLAY_ARTIFACT_ID,
        &no_live_llm_gateway.denied_requests(),
    );

    let mut replay_evidence = vec![
        graph_policy.visibility_evidence(),
        sub_agent_memory_session_visibility_evidence(&child_session, &isolation_receipt),
        sub_agent_memory_session_replay_evidence(&child_session, &isolation_receipt),
        hook_dispatch_replay_evidence(&hook_summary, &hook_selection, &hook_policy),
        no_live_llm_gateway_evidence,
    ];
    replay_evidence.extend(graph_policy_node_receipt_replay_evidence(&graph_policy));
    replay_evidence
}

fn graph_policy_node_receipt_replay_evidence(
    graph_policy: &crate::DeterministicGraphPolicyProposalFixture,
) -> Vec<AgentHarnessEvidence> {
    let request = graph_policy
        .compilation()
        .request
        .as_ref()
        .expect("replay graph policy should produce an execution request");

    request
        .graph
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| {
            let receipt =
                GraphNodeExecutionReceipt::completed(node.id.as_str(), node.executor.as_str());
            AgentHarnessEvidence::present(
                AgentHarnessEvidenceKind::Runtime,
                format!(
                    "graph-loop-node:{}:{}",
                    request.run_id,
                    receipt.node_id.as_str()
                ),
            )
            .with_detail(format!(
                "node_index={} node_id={} executor={} status={} diagnostic_count={} replay=true",
                index,
                receipt.node_id.as_str(),
                receipt.executor.as_str(),
                graph_node_execution_status_label(&receipt.status),
                receipt.diagnostics.len()
            ))
        })
        .collect()
}

fn graph_node_execution_status_label(status: &GraphNodeExecutionStatus) -> &'static str {
    match status {
        GraphNodeExecutionStatus::Completed => "Completed",
        GraphNodeExecutionStatus::Failed => "Failed",
    }
}
