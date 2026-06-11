//! Evidence assertion helpers for agent-system harness scenarios.

use std::{collections::BTreeSet, error::Error, fmt};

use marlin_agent_protocol::{LoopEvidence, LoopEvidenceKind};

/// Assertion failure emitted by harness evidence checks.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HarnessAssertionError {
    message: String,
}

impl HarnessAssertionError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        self.message.as_str()
    }
}

impl fmt::Display for HarnessAssertionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message.as_str())
    }
}

impl Error for HarnessAssertionError {}

/// Validate that all expected evidence kinds are present in a harness capture.
pub fn assert_evidence_kinds(
    evidence: &[LoopEvidence],
    expected: &[LoopEvidenceKind],
) -> Result<(), HarnessAssertionError> {
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

    Err(HarnessAssertionError::new(format!(
        "missing harness evidence kinds: {missing:?}"
    )))
}
