//! Typed discovery of project-scoped Org fact roots.

use crate::OrgSourceStore;

/// Project runtime Org root family discovered from a source store.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OrgProjectRootKind {
    ProjectMemory,
    SessionSummary,
    WorktreeProvenance,
    ToolCapability,
    Topology,
    ContractRegistry,
    EvidenceReceipt,
}

/// Caller-provided candidate root with typed project-memory semantics.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrgProjectRootCandidate {
    pub document: String,
    pub kind: OrgProjectRootKind,
}

impl OrgProjectRootCandidate {
    pub fn new(document: impl Into<String>, kind: OrgProjectRootKind) -> Self {
        Self {
            document: document.into(),
            kind,
        }
    }

    pub fn project_memory(document: impl Into<String>) -> Self {
        Self::new(document, OrgProjectRootKind::ProjectMemory)
    }

    pub fn session_summary(document: impl Into<String>) -> Self {
        Self::new(document, OrgProjectRootKind::SessionSummary)
    }

    pub fn worktree_provenance(document: impl Into<String>) -> Self {
        Self::new(document, OrgProjectRootKind::WorktreeProvenance)
    }

    pub fn tool_capability(document: impl Into<String>) -> Self {
        Self::new(document, OrgProjectRootKind::ToolCapability)
    }

    pub fn topology(document: impl Into<String>) -> Self {
        Self::new(document, OrgProjectRootKind::Topology)
    }

    pub fn contract_registry(document: impl Into<String>) -> Self {
        Self::new(document, OrgProjectRootKind::ContractRegistry)
    }

    pub fn evidence_receipt(document: impl Into<String>) -> Self {
        Self::new(document, OrgProjectRootKind::EvidenceReceipt)
    }
}

/// Existing Org root discovered from a source store.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrgProjectRoot {
    pub document: String,
    pub kind: OrgProjectRootKind,
    pub body: String,
}

impl OrgProjectRoot {
    fn from_candidate(candidate: OrgProjectRootCandidate, body: String) -> Self {
        Self {
            document: candidate.document,
            kind: candidate.kind,
            body,
        }
    }
}

/// Discover existing project-scoped Org roots from typed candidates.
pub fn discover_project_roots(
    store: &impl OrgSourceStore,
    candidates: impl IntoIterator<Item = OrgProjectRootCandidate>,
) -> Vec<OrgProjectRoot> {
    candidates
        .into_iter()
        .filter_map(|candidate| {
            store
                .read_document(&candidate.document)
                .map(|body| OrgProjectRoot::from_candidate(candidate, body))
        })
        .collect()
}
