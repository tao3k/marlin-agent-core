//! Evidence assertion helpers for agent-system harness scenarios.

use std::{collections::BTreeSet, error::Error, fmt};

use crate::{AgentHarnessEvidence, AgentHarnessEvidenceKind};

/// Assertion failure emitted by agent harness evidence checks.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentHarnessAssertionError {
    message: String,
}

impl AgentHarnessAssertionError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        self.message.as_str()
    }
}

impl fmt::Display for AgentHarnessAssertionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message.as_str())
    }
}

impl Error for AgentHarnessAssertionError {}

/// Validate that all expected evidence kinds are present in an agent harness capture.
pub fn assert_agent_harness_evidence_kinds(
    evidence: &[AgentHarnessEvidence],
    expected: &[AgentHarnessEvidenceKind],
) -> Result<(), AgentHarnessAssertionError> {
    let present: BTreeSet<_> = evidence
        .iter()
        .filter(|fact| fact.present)
        .map(|fact| fact.kind.clone())
        .collect();

    let missing: Vec<_> = expected
        .iter()
        .filter(|kind| !present.contains(*kind))
        .cloned()
        .collect();

    if missing.is_empty() {
        return Ok(());
    }

    Err(AgentHarnessAssertionError::new(format!(
        "missing harness evidence kinds: {missing:?}"
    )))
}
