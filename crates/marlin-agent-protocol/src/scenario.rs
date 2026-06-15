//! Declarative scenario contracts consumed by the agent harness.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{AgentEventTopic, AgentSpanName};

/// Stable schema id for serialized agent scenario contracts.
pub const AGENT_SCENARIO_CONTRACT_SCHEMA_ID: &str = "marlin.agent.scenario.v1";

/// Serializable fixture/document contract carrying one agent scenario.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentScenarioContract {
    #[serde(default = "default_agent_scenario_contract_schema_id")]
    pub schema_id: String,
    pub scenario: AgentScenario,
}

impl AgentScenarioContract {
    pub fn new(scenario: AgentScenario) -> Self {
        Self {
            schema_id: AGENT_SCENARIO_CONTRACT_SCHEMA_ID.to_owned(),
            scenario,
        }
    }

    pub fn is_supported_schema(&self) -> bool {
        self.schema_id == AGENT_SCENARIO_CONTRACT_SCHEMA_ID
    }

    pub fn into_scenario(self) -> AgentScenario {
        self.scenario
    }
}

/// Declarative scenario consumed by the agent harness.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentScenario {
    pub id: String,
    pub description: Option<String>,
    #[serde(default)]
    pub steps: Vec<AgentScenarioStep>,
}

impl AgentScenario {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            description: None,
            steps: Vec::new(),
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_step(mut self, step: AgentScenarioStep) -> Self {
        self.steps.push(step);
        self
    }
}

/// One named scenario step.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentScenarioStep {
    pub name: String,
    #[serde(default)]
    pub input: BTreeMap<String, String>,
    #[serde(default)]
    pub expected_event_topics: Vec<AgentEventTopic>,
    #[serde(default)]
    pub expected_span_names: Vec<AgentSpanName>,
}

impl AgentScenarioStep {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            input: BTreeMap::new(),
            expected_event_topics: Vec::new(),
            expected_span_names: Vec::new(),
        }
    }

    pub fn with_input(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.input.insert(key.into(), value.into());
        self
    }

    pub fn expecting_event_topic(mut self, topic: impl Into<AgentEventTopic>) -> Self {
        self.expected_event_topics.push(topic.into());
        self
    }

    pub fn expecting_span_name(mut self, span_name: impl Into<AgentSpanName>) -> Self {
        self.expected_span_names.push(span_name.into());
        self
    }
}

fn default_agent_scenario_contract_schema_id() -> String {
    AGENT_SCENARIO_CONTRACT_SCHEMA_ID.to_owned()
}
