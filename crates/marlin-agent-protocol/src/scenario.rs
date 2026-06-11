//! Declarative scenario contracts consumed by the agent harness.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{AgentEventTopic, AgentSpanName, LoopEvidenceKind};

/// Declarative scenario consumed by the agent harness.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentScenario {
    pub id: String,
    pub description: Option<String>,
    pub steps: Vec<AgentScenarioStep>,
    pub expected_evidence: Vec<LoopEvidenceKind>,
}

impl AgentScenario {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            description: None,
            steps: Vec::new(),
            expected_evidence: Vec::new(),
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

    pub fn expecting_evidence(mut self, kind: LoopEvidenceKind) -> Self {
        self.expected_evidence.push(kind);
        self
    }
}

/// One named scenario step.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentScenarioStep {
    pub name: String,
    pub input: BTreeMap<String, String>,
    pub expected_event_topics: Vec<AgentEventTopic>,
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
