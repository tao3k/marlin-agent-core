//! Agent-harness scenario DTOs with agent-system evidence expectations.

use marlin_agent_protocol::{AgentScenario, AgentScenarioStep};
use serde::{Deserialize, Serialize};

use crate::AgentHarnessEvidenceKind;

/// Stable schema id for serialized agent-harness scenario contracts.
pub const AGENT_HARNESS_SCENARIO_CONTRACT_SCHEMA_ID: &str =
    "marlin.agent.harness_scenario.v1";

/// Serializable fixture contract carrying one agent-harness scenario.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentHarnessScenarioContract {
    /// Schema id used to reject stale or ambiguous agent-harness fixtures.
    #[serde(default = "default_agent_harness_scenario_contract_schema_id")]
    pub schema_id: String,
    /// Scenario and evidence expectations consumed by the agent harness.
    pub scenario: AgentHarnessScenario,
}

impl AgentHarnessScenarioContract {
    /// Creates a supported agent-harness scenario contract.
    pub fn new(scenario: AgentHarnessScenario) -> Self {
        Self {
            schema_id: AGENT_HARNESS_SCENARIO_CONTRACT_SCHEMA_ID.to_owned(),
            scenario,
        }
    }

    /// Returns true when this contract uses the supported schema id.
    pub fn is_supported_schema(&self) -> bool {
        self.schema_id == AGENT_HARNESS_SCENARIO_CONTRACT_SCHEMA_ID
    }

    /// Consumes this contract and returns its scenario.
    pub fn into_scenario(self) -> AgentHarnessScenario {
        self.scenario
    }
}

/// Declarative scenario consumed by the agent harness.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentHarnessScenario {
    /// Protocol-level scenario shape for events, spans, and input steps.
    pub agent_scenario: AgentScenario,
    /// Agent-harness evidence categories required by this scenario.
    #[serde(default)]
    pub expected_evidence: Vec<AgentHarnessEvidenceKind>,
}

impl AgentHarnessScenario {
    /// Creates an agent-harness scenario from a stable id.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            agent_scenario: AgentScenario::new(id),
            expected_evidence: Vec::new(),
        }
    }

    /// Wraps an existing protocol scenario without evidence requirements.
    pub fn from_agent_scenario(agent_scenario: AgentScenario) -> Self {
        Self {
            agent_scenario,
            expected_evidence: Vec::new(),
        }
    }

    /// Returns the stable scenario id.
    pub fn id(&self) -> &str {
        self.agent_scenario.id.as_str()
    }

    /// Returns the optional scenario description.
    pub fn description(&self) -> Option<&str> {
        self.agent_scenario.description.as_deref()
    }

    /// Returns protocol-level scenario steps.
    pub fn steps(&self) -> &[AgentScenarioStep] {
        self.agent_scenario.steps.as_slice()
    }

    /// Sets the scenario description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.agent_scenario = self.agent_scenario.with_description(description);
        self
    }

    /// Adds one protocol-level scenario step.
    pub fn with_step(mut self, step: AgentScenarioStep) -> Self {
        self.agent_scenario = self.agent_scenario.with_step(step);
        self
    }

    /// Adds one agent-harness evidence expectation.
    pub fn expecting_evidence(mut self, kind: impl Into<AgentHarnessEvidenceKind>) -> Self {
        self.expected_evidence.push(kind.into());
        self
    }
}

fn default_agent_harness_scenario_contract_schema_id() -> String {
    AGENT_HARNESS_SCENARIO_CONTRACT_SCHEMA_ID.to_owned()
}
