//! Typed evidence facts captured by the agent harness.

use serde::{Deserialize, Serialize};

/// Evidence category captured by the agent harness.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum LoopEvidenceKind {
    Content,
    Safety,
    Budget,
    Registry,
    Workflow,
    RunLog,
    Provider,
    Tool,
    SubAgent,
    Runtime,
    Visibility,
}

/// Typed fact captured while validating an agent loop.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LoopEvidence {
    pub kind: LoopEvidenceKind,
    pub subject: String,
    pub present: bool,
    pub detail: Option<String>,
}

impl LoopEvidence {
    pub fn present(kind: LoopEvidenceKind, subject: impl Into<String>) -> Self {
        Self {
            kind,
            subject: subject.into(),
            present: true,
            detail: None,
        }
    }

    pub fn missing(kind: LoopEvidenceKind, subject: impl Into<String>) -> Self {
        Self {
            kind,
            subject: subject.into(),
            present: false,
            detail: None,
        }
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}
