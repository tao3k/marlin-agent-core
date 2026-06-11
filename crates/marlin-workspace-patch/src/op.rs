//! Patch operation model for controlled workspace mutation.

use marlin_org_model::{CheckboxState, OrgLink, OrgNodeId, TodoState};
use serde::{Deserialize, Serialize};

/// Batch of typed workspace patch operations from an actor.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorkspacePatch {
    pub reason: String,
    pub source_agent: Option<String>,
    pub ops: Vec<WorkspacePatchOp>,
}

impl WorkspacePatch {
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
            source_agent: None,
            ops: Vec::new(),
        }
    }
}

/// Single typed workspace mutation operation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum WorkspacePatchOp {
    SetTodo {
        node: OrgNodeId,
        state: TodoState,
    },
    SetProperty {
        node: OrgNodeId,
        key: String,
        value: String,
    },
    AddCheckbox {
        node: OrgNodeId,
        text: String,
        state: CheckboxState,
    },
    MarkCheckbox {
        node: OrgNodeId,
        index: usize,
        state: CheckboxState,
    },
    AppendSection {
        node: OrgNodeId,
        heading: String,
        body: String,
    },
    AddLink {
        node: OrgNodeId,
        link: OrgLink,
    },
    AddEvidenceRef {
        node: OrgNodeId,
        evidence: EvidenceRef,
    },
    AddMetricPoint {
        node: OrgNodeId,
        metric: MetricPoint,
    },
    AddDecision {
        node: OrgNodeId,
        decision: DecisionRecord,
    },
    AddTraceEvent {
        node: OrgNodeId,
        body: String,
    },
    MarkMemoryCandidate {
        node: OrgNodeId,
        dispatch: String,
    },
}

/// Evidence reference attached to a workspace node.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EvidenceRef {
    pub target: String,
    pub summary: String,
    pub trust: EvidenceTrust,
}

/// Trust state for evidence before it enters rendered views.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum EvidenceTrust {
    Internal,
    External,
    Quarantined,
    Verified,
}

/// Numeric metric sample recorded in workspace state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MetricPoint {
    pub name: String,
    pub value: f64,
    pub unit: Option<String>,
}

/// Decision record with rationale.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DecisionRecord {
    pub decision: String,
    pub rationale: String,
}
