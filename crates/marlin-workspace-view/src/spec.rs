//! View specification model for task-scoped workspace read-outs.

use marlin_org_model::OrgNodeId;
use serde::{Deserialize, Serialize};

/// Request describing how workspace nodes should be rendered.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceViewSpec {
    pub roots: Vec<OrgNodeId>,
    pub include: Vec<WorkspaceField>,
    pub exclude: Vec<WorkspaceField>,
    pub max_tokens: usize,
    pub render_mode: RenderMode,
}

impl WorkspaceViewSpec {
    pub fn compact(roots: Vec<OrgNodeId>) -> Self {
        Self {
            roots,
            include: vec![
                WorkspaceField::Title,
                WorkspaceField::Todo,
                WorkspaceField::SourceSpan,
                WorkspaceField::SelectedProperties,
                WorkspaceField::OpenCheckboxes,
                WorkspaceField::EvidenceLinks,
                WorkspaceField::Blockers,
            ],
            exclude: vec![WorkspaceField::Archived, WorkspaceField::RawBlockOutput],
            max_tokens: 1_800,
            render_mode: RenderMode::AgentCompact,
        }
    }
}

/// Field class included or excluded from a rendered workspace view.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum WorkspaceField {
    Title,
    Todo,
    SourceSpan,
    Tags,
    SelectedProperties,
    OpenCheckboxes,
    EvidenceLinks,
    Decisions,
    Metrics,
    Blockers,
    Trace,
    Archived,
    RawBlockOutput,
    StaleLogs,
    Custom(String),
}

/// Output mode for a rendered workspace view.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RenderMode {
    AgentCompact,
    UiTree,
    ReceiptSummary,
    MemoryCandidate,
}
