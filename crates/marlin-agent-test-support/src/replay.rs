//! Replay artifacts for no-LLM runtime evidence chains.

use marlin_agent_protocol::{
    AgentScenario, AgentScenarioContract, AgentScenarioStep, LoopEvidence, LoopEvidenceKind,
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

/// Typed replay artifact loaded by harness tests without touching live LLMs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NoLlmRuntimeReplayArtifact {
    contract: AgentScenarioContract,
    replay_evidence: Vec<LoopEvidence>,
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

/// Deterministic replay artifact covering graph, session, and hook receipts.
pub fn no_llm_runtime_replay_artifact_fixture() -> NoLlmRuntimeReplayArtifact {
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

    let scenario = AgentScenario::new(NO_LLM_RUNTIME_REPLAY_ARTIFACT_ID)
        .with_description("replays graph, sub-agent session, and hook receipts without live LLMs")
        .with_step(
            AgentScenarioStep::new("load-replay-artifact")
                .with_input("artifact", NO_LLM_RUNTIME_REPLAY_ARTIFACT_ID),
        )
        .expecting_evidence(LoopEvidenceKind::Visibility)
        .expecting_evidence(LoopEvidenceKind::Runtime);

    let replay_evidence = vec![
        graph_policy.visibility_evidence(),
        sub_agent_memory_session_visibility_evidence(&child_session, &isolation_receipt),
        sub_agent_memory_session_replay_evidence(&child_session, &isolation_receipt),
        hook_dispatch_replay_evidence(&hook_summary, &hook_selection, &hook_policy),
    ];

    NoLlmRuntimeReplayArtifact {
        contract: AgentScenarioContract::new(scenario),
        replay_evidence,
    }
}
