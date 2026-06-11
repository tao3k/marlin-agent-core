//! Controlled harness runtime for scenario execution and evidence capture.

use marlin_agent_kernel::GraphLoopKernel;
use marlin_agent_protocol::{
    AgentScenario, GraphLoopExecutionRequest, GraphLoopExecutionResult, LoopEvidence,
    RuntimePlanSnapshot,
};
use marlin_agent_runtime::{
    CancellationToken, RuntimeEnvironment, RuntimeEventStream, TokioAgentRuntime,
};

use crate::{HarnessAssertionError, assert_evidence_kinds};

/// Controlled runtime plus typed evidence captured for one harness scenario.
#[derive(Debug)]
pub struct HarnessRuntime {
    runtime: TokioAgentRuntime,
    events: RuntimeEventStream,
    evidence: Vec<LoopEvidence>,
}

impl HarnessRuntime {
    pub fn new(event_buffer: usize) -> Self {
        let (runtime, events) = TokioAgentRuntime::new(event_buffer);
        Self {
            runtime,
            events,
            evidence: Vec::new(),
        }
    }

    /// Create a harness runtime with an explicit runtime environment snapshot.
    pub fn with_environment(event_buffer: usize, environment: RuntimeEnvironment) -> Self {
        let (runtime, events) = TokioAgentRuntime::with_environment(
            event_buffer,
            CancellationToken::new(),
            environment,
        );
        Self {
            runtime,
            events,
            evidence: Vec::new(),
        }
    }

    pub fn runtime(&self) -> TokioAgentRuntime {
        self.runtime.clone()
    }

    /// Borrow the environment visible to harness-owned runtime work.
    pub fn environment(&self) -> &RuntimeEnvironment {
        self.runtime.environment()
    }

    pub fn events(&mut self) -> &mut RuntimeEventStream {
        &mut self.events
    }

    pub fn record_evidence(&mut self, evidence: LoopEvidence) {
        self.evidence.push(evidence);
    }

    pub fn evidence(&self) -> &[LoopEvidence] {
        self.evidence.as_slice()
    }

    pub fn into_parts(self) -> (TokioAgentRuntime, RuntimeEventStream, Vec<LoopEvidence>) {
        (self.runtime, self.events, self.evidence)
    }

    pub fn assert_scenario_evidence(
        &self,
        scenario: &AgentScenario,
    ) -> Result<(), HarnessAssertionError> {
        assert_evidence_kinds(self.evidence(), scenario.expected_evidence.as_slice())
    }

    pub async fn execute_graph<K>(
        &mut self,
        scenario: &AgentScenario,
        kernel: &K,
        request: GraphLoopExecutionRequest,
    ) -> HarnessExecutionReport
    where
        K: GraphLoopKernel,
    {
        let task = kernel.spawn_execution(request, &self.runtime);
        let result = match task.join().await {
            Ok(result) => result,
            Err(error) => GraphLoopExecutionResult::failed(
                RuntimePlanSnapshot {
                    run_id: scenario.id.clone(),
                    graph_id: "harness".to_owned(),
                    active_node: None,
                },
                vec![format!("harness task join failed: {error}")],
            ),
        };
        let assertion = self.assert_scenario_evidence(scenario).err();
        HarnessExecutionReport {
            scenario_id: scenario.id.clone(),
            result,
            evidence: self.evidence.clone(),
            assertion,
        }
    }
}

/// Result of running one graph-loop request through the harness.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HarnessExecutionReport {
    pub scenario_id: String,
    pub result: GraphLoopExecutionResult,
    pub evidence: Vec<LoopEvidence>,
    pub assertion: Option<HarnessAssertionError>,
}
