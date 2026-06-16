//! Replay artifacts for no-LLM runtime evidence chains.

use std::{error::Error, fmt};

use marlin_agent_harness_types::{
    AGENT_HARNESS_SCENARIO_CONTRACT_SCHEMA_ID, AgentHarnessEvidence, AgentHarnessEvidenceKind,
    AgentHarnessScenario, AgentHarnessScenarioContract,
};
use marlin_agent_protocol::{
    FailureClassificationReceipt, GraphLoopContinuationAction, GraphLoopContinuationReceipt,
    GraphLoopFailureKind, GraphNodeExecutionReceipt, GraphNodeExecutionStatus, ModelEndpoint,
    ModelGatewayRequest, ModelGatewayTransport, user_gateway_message,
};
use marlin_agent_sessions::SessionKind;

use crate::{
    NO_LIVE_LLM_GATE_DENIAL_MESSAGE, NoLiveLlmModelGateway, ScriptedGatewayRequestReceipt,
    complex_gerbil_graph_policy_replay_fixture, custom_hook_policy_receipt_fixture,
    custom_sub_agent_start_hook_summary_fixture, hook_dispatch_replay_evidence,
    no_live_llm_gateway_denial_evidence, sub_agent_hook_dispatch_selection_fixture,
    sub_agent_memory_denied_fixture, sub_agent_memory_session_replay_evidence,
    sub_agent_memory_session_visibility_evidence,
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
#[derive(Clone, Debug, PartialEq)]
pub struct NoLlmRuntimeReplayArtifact {
    contract: AgentHarnessScenarioContract,
    replay_evidence: Vec<AgentHarnessEvidence>,
    receipts: NoLlmRuntimeReplayReceipts,
}

/// Runtime receipts replayed by the deterministic no-LLM artifact.
#[derive(Clone, Debug, PartialEq)]
pub struct NoLlmRuntimeReplayReceipts {
    continuation_receipt: GraphLoopContinuationReceipt,
    failure_classification_receipt: FailureClassificationReceipt,
    node_receipts: Vec<GraphNodeExecutionReceipt>,
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

    /// Typed runtime receipts preserved by this replay artifact.
    pub fn receipts(&self) -> &NoLlmRuntimeReplayReceipts {
        &self.receipts
    }

    /// Consume the artifact into its scenario contract and replay evidence.
    pub fn into_parts(self) -> (AgentHarnessScenarioContract, Vec<AgentHarnessEvidence>) {
        (self.contract, self.replay_evidence)
    }
}

impl NoLlmRuntimeReplayReceipts {
    /// Continuation decision receipt replayed for the accepted graph policy.
    pub fn continuation_receipt(&self) -> &GraphLoopContinuationReceipt {
        &self.continuation_receipt
    }

    /// Failure classification receipt replayed for the no-live-LLM gateway denial.
    pub fn failure_classification_receipt(&self) -> &FailureClassificationReceipt {
        &self.failure_classification_receipt
    }

    /// Per-node execution receipts replayed for the compiled graph.
    pub fn node_receipts(&self) -> &[GraphNodeExecutionReceipt] {
        &self.node_receipts
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

    let deterministic_replay = deterministic_no_llm_runtime_replay();

    Ok(NoLlmRuntimeReplayArtifact {
        contract,
        replay_evidence: deterministic_replay.replay_evidence,
        receipts: deterministic_replay.receipts,
    })
}

/// Deterministic replay artifact fixture covering graph, session, and hook receipts.
pub fn no_llm_runtime_replay_artifact_fixture() -> NoLlmRuntimeReplayArtifact {
    load_no_llm_runtime_replay_artifact(NO_LLM_RUNTIME_REPLAY_CONTRACT_JSON)
        .expect("no-LLM runtime replay contract fixture should load")
}

struct DeterministicNoLlmRuntimeReplay {
    replay_evidence: Vec<AgentHarnessEvidence>,
    receipts: NoLlmRuntimeReplayReceipts,
}

fn deterministic_no_llm_runtime_replay() -> DeterministicNoLlmRuntimeReplay {
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
    let denied_requests = no_live_llm_gateway.denied_requests();
    let no_live_llm_gateway_evidence =
        no_live_llm_gateway_denial_evidence(NO_LLM_RUNTIME_REPLAY_ARTIFACT_ID, &denied_requests);

    let receipts = graph_policy_loop_replay_receipts(&graph_policy, &denied_requests);
    let mut replay_evidence = vec![
        graph_policy.visibility_evidence(),
        sub_agent_memory_session_visibility_evidence(&child_session, &isolation_receipt),
        sub_agent_memory_session_replay_evidence(&child_session, &isolation_receipt),
        hook_dispatch_replay_evidence(&hook_summary, &hook_selection, &hook_policy),
        no_live_llm_gateway_evidence,
    ];
    replay_evidence.extend(graph_policy_loop_receipt_replay_evidence(&receipts));
    replay_evidence.extend(graph_policy_node_receipt_replay_evidence(&receipts));
    DeterministicNoLlmRuntimeReplay {
        replay_evidence,
        receipts,
    }
}

fn graph_policy_loop_replay_receipts(
    graph_policy: &crate::DeterministicGraphPolicyProposalFixture,
    denied_requests: &[ScriptedGatewayRequestReceipt],
) -> NoLlmRuntimeReplayReceipts {
    let request = graph_policy
        .compilation()
        .request
        .as_ref()
        .expect("replay graph policy should produce an execution request");
    let iteration_id = 0_u64;
    let continuation_reason = "native Gerbil graph policy replay accepted runtime graph";
    let continuation_receipt = GraphLoopContinuationReceipt::new(
        request.run_id.clone(),
        iteration_id,
        GraphLoopContinuationAction::Rewrite {
            graph: request.graph.clone(),
            reason: continuation_reason.to_owned(),
        },
    )
    .with_diagnostic("replay=true")
    .with_diagnostic("live_llm=false");

    let denied_models = denied_models_summary(denied_requests);
    let failure_classification_receipt = FailureClassificationReceipt::new(
        format!("{}:no-live-llm-denial", request.run_id),
        request.run_id.clone(),
        iteration_id,
        GraphLoopFailureKind::PolicyFailure,
    )
    .with_retryable(false)
    .with_requires_human(false)
    .with_diagnostic(format!("denied_requests={}", denied_requests.len()))
    .with_diagnostic(format!("denied_models=[{denied_models}]"))
    .with_diagnostic(format!(
        "denial_message=\"{NO_LIVE_LLM_GATE_DENIAL_MESSAGE}\""
    ));

    NoLlmRuntimeReplayReceipts {
        continuation_receipt,
        failure_classification_receipt,
        node_receipts: graph_policy_node_receipts(graph_policy),
    }
}

fn graph_policy_loop_receipt_replay_evidence(
    receipts: &NoLlmRuntimeReplayReceipts,
) -> Vec<AgentHarnessEvidence> {
    let (continuation_action, continuation_graph_id, continuation_node_count) =
        match &receipts.continuation_receipt.action {
            GraphLoopContinuationAction::Rewrite { graph, .. } => {
                ("Rewrite", graph.graph_id.as_str(), graph.nodes.len())
            }
            GraphLoopContinuationAction::Accept => ("Accept", "", 0),
            GraphLoopContinuationAction::Deny { .. } => ("Deny", "", 0),
            GraphLoopContinuationAction::Defer { .. } => ("Defer", "", 0),
        };
    let continuation_evidence = AgentHarnessEvidence::present(
        AgentHarnessEvidenceKind::Runtime,
        format!(
            "graph-loop-continuation:{}:{}",
            receipts.continuation_receipt.run_id.as_str(),
            receipts.continuation_receipt.iteration_id.get()
        ),
    )
    .with_detail(format!(
        "continuation_receipt=true run_id={} iteration_id={} action={} graph_id={} node_count={} diagnostic_count={} replay=true live_llm=false",
        receipts.continuation_receipt.run_id.as_str(),
        receipts.continuation_receipt.iteration_id.get(),
        continuation_action,
        continuation_graph_id,
        continuation_node_count,
        receipts.continuation_receipt.diagnostics.len(),
    ));

    let failure_classification_evidence = AgentHarnessEvidence::present(
        AgentHarnessEvidenceKind::Runtime,
        format!(
            "graph-loop-failure-classification:{}:{}",
            receipts.failure_classification_receipt.run_id.as_str(),
            receipts.failure_classification_receipt.iteration_id.get()
        ),
    )
    .with_detail(format!(
        "failure_classification_receipt=true classification_id={} run_id={} iteration_id={} failure_kind={} retryable={} requires_human={} source_node_count={} diagnostic_count={} replay=true live_llm=false",
        receipts.failure_classification_receipt.classification_id.as_str(),
        receipts.failure_classification_receipt.run_id.as_str(),
        receipts.failure_classification_receipt.iteration_id.get(),
        graph_loop_failure_kind_label(&receipts.failure_classification_receipt.failure_kind),
        receipts.failure_classification_receipt.retryable,
        receipts.failure_classification_receipt.requires_human,
        receipts.failure_classification_receipt.source_nodes.len(),
        receipts.failure_classification_receipt.diagnostics.len(),
    ));

    vec![continuation_evidence, failure_classification_evidence]
}

fn graph_policy_node_receipts(
    graph_policy: &crate::DeterministicGraphPolicyProposalFixture,
) -> Vec<GraphNodeExecutionReceipt> {
    let request = graph_policy
        .compilation()
        .request
        .as_ref()
        .expect("replay graph policy should produce an execution request");

    request
        .graph
        .nodes
        .iter()
        .map(|node| GraphNodeExecutionReceipt::completed(node.id.as_str(), node.executor.as_str()))
        .collect()
}

fn graph_policy_node_receipt_replay_evidence(
    receipts: &NoLlmRuntimeReplayReceipts,
) -> Vec<AgentHarnessEvidence> {
    receipts
        .node_receipts
        .iter()
        .enumerate()
        .map(|(index, receipt)| {
            AgentHarnessEvidence::present(
                AgentHarnessEvidenceKind::Runtime,
                format!(
                    "graph-loop-node:{}:{}",
                    receipts.continuation_receipt.run_id.as_str(),
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

fn denied_models_summary(denied_requests: &[ScriptedGatewayRequestReceipt]) -> String {
    denied_requests
        .iter()
        .map(|request| request.litellm_model_id.as_str())
        .collect::<Vec<_>>()
        .join(",")
}

fn graph_loop_failure_kind_label(kind: &GraphLoopFailureKind) -> &'static str {
    match kind {
        GraphLoopFailureKind::TransientFailure => "TransientFailure",
        GraphLoopFailureKind::ToolUsageFailure => "ToolUsageFailure",
        GraphLoopFailureKind::VerificationFailure => "VerificationFailure",
        GraphLoopFailureKind::ContextFailure => "ContextFailure",
        GraphLoopFailureKind::PolicyFailure => "PolicyFailure",
        GraphLoopFailureKind::StrategyFailure => "StrategyFailure",
        GraphLoopFailureKind::Unknown => "Unknown",
    }
}

fn graph_node_execution_status_label(status: &GraphNodeExecutionStatus) -> &'static str {
    match status {
        GraphNodeExecutionStatus::Completed => "Completed",
        GraphNodeExecutionStatus::Failed => "Failed",
    }
}
