//! Project-scoped runtime graph query envelopes.

use serde::{Deserialize, Serialize};

use super::ids::{
    GraphQueryLimit, GraphQueryScoreBasisPoints, ProjectRuntimeAgentId,
    ProjectRuntimeBackendRequirementId, ProjectRuntimeBranchRef, ProjectRuntimeContentId,
    ProjectRuntimeContextPackId, ProjectRuntimeEvidenceId, ProjectRuntimeIsolationRequirementId,
    ProjectRuntimeMemoryId, ProjectRuntimeProjectId, ProjectRuntimeReceiptId,
    ProjectRuntimeRootSessionId, ProjectRuntimeSessionId, ProjectRuntimeSourceAnchorId,
    ProjectRuntimeSourceSpanRef, ProjectRuntimeToolCapabilityId, ProjectRuntimeWorkspaceId,
    ProjectRuntimeWorktreeId,
};

/// Project graph surface visible to a query.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GraphQueryVisibleSurface {
    Workspace,
    Memory,
    Tools,
    Sessions,
    Content,
    Topology,
    Evidence,
    Failures,
}

/// Secret visibility gate for project graph reads.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum GraphQuerySecretVisibility {
    #[default]
    Denied,
    Allowed,
}

fn default_graph_query_visible_surfaces() -> Vec<GraphQueryVisibleSurface> {
    vec![
        GraphQueryVisibleSurface::Workspace,
        GraphQueryVisibleSurface::Memory,
        GraphQueryVisibleSurface::Tools,
        GraphQueryVisibleSurface::Sessions,
        GraphQueryVisibleSurface::Content,
        GraphQueryVisibleSurface::Topology,
        GraphQueryVisibleSurface::Evidence,
        GraphQueryVisibleSurface::Failures,
    ]
}

/// Read surfaces visible to a graph query.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GraphQueryVisibility {
    #[serde(default = "default_graph_query_visible_surfaces")]
    pub surfaces: Vec<GraphQueryVisibleSurface>,
    #[serde(default)]
    pub secrets: GraphQuerySecretVisibility,
}

impl GraphQueryVisibility {
    pub fn standard() -> Self {
        Self::default()
    }

    pub fn allows_surface(&self, surface: GraphQueryVisibleSurface) -> bool {
        self.surfaces.contains(&surface)
    }

    pub fn allows_secrets(&self) -> bool {
        self.secrets == GraphQuerySecretVisibility::Allowed
    }
}

impl Default for GraphQueryVisibility {
    fn default() -> Self {
        Self {
            surfaces: default_graph_query_visible_surfaces(),
            secrets: GraphQuerySecretVisibility::Denied,
        }
    }
}

/// Fallback scope that may contribute graph matches.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GraphQueryFallbackScope {
    SessionLocal,
    Project,
    Workspace,
    WorktreeProvenance,
    Global,
}

/// External project fallback gate.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum GraphQueryExternalProjectPolicy {
    #[default]
    Disabled,
    Enabled {
        min_score_basis_points: GraphQueryScoreBasisPoints,
    },
}

fn default_graph_query_fallback_scopes() -> Vec<GraphQueryFallbackScope> {
    vec![
        GraphQueryFallbackScope::SessionLocal,
        GraphQueryFallbackScope::Project,
        GraphQueryFallbackScope::Workspace,
        GraphQueryFallbackScope::WorktreeProvenance,
    ]
}

/// Explicit fallback policy for project graph reads.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GraphQueryFallbackPolicy {
    #[serde(default = "default_graph_query_fallback_scopes")]
    pub scopes: Vec<GraphQueryFallbackScope>,
    #[serde(default)]
    pub external_projects: GraphQueryExternalProjectPolicy,
}

impl GraphQueryFallbackPolicy {
    pub fn same_project() -> Self {
        Self::default()
    }

    pub fn with_external_projects(mut self, min_score_basis_points: u16) -> Self {
        self.external_projects = GraphQueryExternalProjectPolicy::Enabled {
            min_score_basis_points: GraphQueryScoreBasisPoints::new(min_score_basis_points),
        };
        self
    }

    pub fn includes_scope(&self, scope: GraphQueryFallbackScope) -> bool {
        self.scopes.contains(&scope)
    }
}

impl Default for GraphQueryFallbackPolicy {
    fn default() -> Self {
        Self {
            scopes: default_graph_query_fallback_scopes(),
            external_projects: GraphQueryExternalProjectPolicy::Disabled,
        }
    }
}

/// Runtime context carried by every project graph query.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GraphQueryContext {
    pub project_id: ProjectRuntimeProjectId,
    pub workspace_id: Option<ProjectRuntimeWorkspaceId>,
    pub worktree_id: Option<ProjectRuntimeWorktreeId>,
    pub branch: Option<ProjectRuntimeBranchRef>,
    pub root_session_id: Option<ProjectRuntimeRootSessionId>,
    pub session_id: Option<ProjectRuntimeSessionId>,
    #[serde(default)]
    pub agent_lineage: Vec<ProjectRuntimeAgentId>,
    pub content_anchor: Option<ProjectRuntimeContentId>,
    #[serde(default)]
    pub visibility: GraphQueryVisibility,
    #[serde(default)]
    pub fallback_policy: GraphQueryFallbackPolicy,
}

impl GraphQueryContext {
    pub fn new(project_id: impl Into<String>) -> Self {
        Self {
            project_id: ProjectRuntimeProjectId::new(project_id),
            workspace_id: None,
            worktree_id: None,
            branch: None,
            root_session_id: None,
            session_id: None,
            agent_lineage: Vec::new(),
            content_anchor: None,
            visibility: GraphQueryVisibility::default(),
            fallback_policy: GraphQueryFallbackPolicy::default(),
        }
    }

    pub fn with_workspace(mut self, workspace_id: impl Into<String>) -> Self {
        self.workspace_id = Some(ProjectRuntimeWorkspaceId::new(workspace_id));
        self
    }

    pub fn with_worktree(mut self, worktree_id: impl Into<String>) -> Self {
        self.worktree_id = Some(ProjectRuntimeWorktreeId::new(worktree_id));
        self
    }

    pub fn with_branch(mut self, branch: impl Into<String>) -> Self {
        self.branch = Some(ProjectRuntimeBranchRef::new(branch));
        self
    }

    pub fn with_root_session(mut self, root_session_id: impl Into<String>) -> Self {
        self.root_session_id = Some(ProjectRuntimeRootSessionId::new(root_session_id));
        self
    }

    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(ProjectRuntimeSessionId::new(session_id));
        self
    }

    pub fn with_agent(mut self, agent_id: impl Into<String>) -> Self {
        self.agent_lineage
            .push(ProjectRuntimeAgentId::new(agent_id));
        self
    }

    pub fn with_content_anchor(mut self, content_id: impl Into<String>) -> Self {
        self.content_anchor = Some(ProjectRuntimeContentId::new(content_id));
        self
    }

    pub fn with_visibility(mut self, visibility: GraphQueryVisibility) -> Self {
        self.visibility = visibility;
        self
    }

    pub fn with_fallback_policy(mut self, fallback_policy: GraphQueryFallbackPolicy) -> Self {
        self.fallback_policy = fallback_policy;
        self
    }
}

/// Query family selected by the project runtime graph router.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GraphQueryFamily {
    Org,
    Memory,
    Tool,
    Session,
    Content,
    Topology,
    Evidence,
    Failure,
}

/// Project runtime graph query request.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GraphQueryRequest {
    pub context: GraphQueryContext,
    pub family: GraphQueryFamily,
    pub query: String,
    pub capability_id: Option<ProjectRuntimeToolCapabilityId>,
    pub memory_id: Option<ProjectRuntimeMemoryId>,
    pub content_id: Option<ProjectRuntimeContentId>,
    pub limit: Option<GraphQueryLimit>,
}

impl GraphQueryRequest {
    pub fn new(
        context: GraphQueryContext,
        family: GraphQueryFamily,
        query: impl Into<String>,
    ) -> Self {
        Self {
            context,
            family,
            query: query.into(),
            capability_id: None,
            memory_id: None,
            content_id: None,
            limit: None,
        }
    }

    pub fn with_tool_capability(mut self, capability_id: impl Into<String>) -> Self {
        self.capability_id = Some(ProjectRuntimeToolCapabilityId::new(capability_id));
        self
    }

    pub fn with_memory_anchor(mut self, memory_id: impl Into<String>) -> Self {
        self.memory_id = Some(ProjectRuntimeMemoryId::new(memory_id));
        self
    }

    pub fn with_content_anchor(mut self, content_id: impl Into<String>) -> Self {
        self.content_id = Some(ProjectRuntimeContentId::new(content_id));
        self
    }

    pub fn with_limit(mut self, limit: impl Into<GraphQueryLimit>) -> Self {
        self.limit = Some(limit.into());
        self
    }
}

/// Agent intent for project memory recall.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ProjectMemoryRecallIntent {
    ContinueWork,
    RecoverDecision,
    ExplainEvidence,
    PrepareToolUse,
    AnswerQuestion,
}

/// Contract-indexed query term used to build a project memory frontier.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash, Serialize, Deserialize)]
pub struct ProjectMemoryRecallTerm(String);

impl ProjectMemoryRecallTerm {
    pub fn new(term: impl Into<String>) -> Self {
        Self(term.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl From<&str> for ProjectMemoryRecallTerm {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for ProjectMemoryRecallTerm {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

/// Intent/context based project memory recall request.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProjectMemoryRecallRequest {
    pub context: GraphQueryContext,
    pub intent: ProjectMemoryRecallIntent,
    #[serde(default)]
    pub query_terms: Vec<ProjectMemoryRecallTerm>,
    pub memory_id: Option<ProjectRuntimeMemoryId>,
    pub content_id: Option<ProjectRuntimeContentId>,
    pub limit: Option<GraphQueryLimit>,
}

impl ProjectMemoryRecallRequest {
    pub fn new(context: GraphQueryContext, intent: ProjectMemoryRecallIntent) -> Self {
        Self {
            context,
            intent,
            query_terms: Vec::new(),
            memory_id: None,
            content_id: None,
            limit: None,
        }
    }

    pub fn with_query_term(mut self, term: impl Into<ProjectMemoryRecallTerm>) -> Self {
        self.query_terms.push(term.into());
        self
    }

    pub fn with_memory_anchor(mut self, memory_id: impl Into<String>) -> Self {
        self.memory_id = Some(ProjectRuntimeMemoryId::new(memory_id));
        self
    }

    pub fn with_content_anchor(mut self, content_id: impl Into<String>) -> Self {
        self.content_id = Some(ProjectRuntimeContentId::new(content_id));
        self
    }

    pub fn with_limit(mut self, limit: impl Into<GraphQueryLimit>) -> Self {
        self.limit = Some(limit.into());
        self
    }

    pub fn query_text(&self) -> String {
        self.query_terms
            .iter()
            .map(ProjectMemoryRecallTerm::as_str)
            .collect::<Vec<_>>()
            .join(" ")
    }

    pub fn as_graph_query_request(&self) -> GraphQueryRequest {
        let mut request = GraphQueryRequest::new(
            self.context.clone(),
            GraphQueryFamily::Memory,
            self.query_text(),
        );
        if let Some(memory_id) = &self.memory_id {
            request = request.with_memory_anchor(memory_id.as_str());
        }
        if let Some(content_id) = &self.content_id {
            request = request.with_content_anchor(content_id.as_str());
        }
        if let Some(limit) = self.limit {
            request = request.with_limit(limit);
        }
        request
    }
}

/// One source-spanned memory fact packed for an agent.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProjectMemoryContextFact {
    pub graph_match: GraphQueryMatch,
    pub claim: String,
    pub source_span: Option<ProjectRuntimeSourceSpanRef>,
    #[serde(default)]
    pub evidence_ids: Vec<ProjectRuntimeEvidenceId>,
}

impl ProjectMemoryContextFact {
    pub fn new(graph_match: GraphQueryMatch, claim: impl Into<String>) -> Self {
        Self {
            graph_match,
            claim: claim.into(),
            source_span: None,
            evidence_ids: Vec::new(),
        }
    }

    pub fn with_source_span(mut self, source_span: impl Into<String>) -> Self {
        self.source_span = Some(ProjectRuntimeSourceSpanRef::new(source_span));
        self
    }

    pub fn with_evidence(mut self, evidence_id: impl Into<String>) -> Self {
        self.evidence_ids
            .push(ProjectRuntimeEvidenceId::new(evidence_id));
        self
    }
}

/// Bounded project memory context pack produced from recall.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProjectMemoryContextPack {
    pub context_pack_id: ProjectRuntimeContextPackId,
    pub request: ProjectMemoryRecallRequest,
    #[serde(default)]
    pub facts: Vec<ProjectMemoryContextFact>,
    #[serde(default)]
    pub source_receipts: Vec<ProjectRuntimeReceiptId>,
}

impl ProjectMemoryContextPack {
    pub fn new(context_pack_id: impl Into<String>, request: ProjectMemoryRecallRequest) -> Self {
        Self {
            context_pack_id: ProjectRuntimeContextPackId::new(context_pack_id),
            request,
            facts: Vec::new(),
            source_receipts: Vec::new(),
        }
    }

    pub fn with_fact(mut self, fact: ProjectMemoryContextFact) -> Self {
        self.facts.push(fact);
        self
    }

    pub fn with_source_receipt(mut self, receipt_id: impl Into<String>) -> Self {
        self.source_receipts
            .push(ProjectRuntimeReceiptId::new(receipt_id));
        self
    }
}

/// Relationship fact used by graph match ranking and audit receipts.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GraphQueryRelationshipFact {
    SameProject,
    SameWorkspace,
    SameRootSession,
    SameSessionLineage,
    SameContentAncestry,
    SameWorktreeProvenance,
    ExplicitBacklink,
    ContractValidated,
    ExternalProject,
}

/// Relationship facts used by ranking and audit receipts.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct GraphQueryMatchRelationship {
    #[serde(default)]
    pub facts: Vec<GraphQueryRelationshipFact>,
}

impl GraphQueryMatchRelationship {
    pub fn new(facts: impl IntoIterator<Item = GraphQueryRelationshipFact>) -> Self {
        Self {
            facts: facts.into_iter().collect(),
        }
    }

    pub fn has_fact(&self, fact: GraphQueryRelationshipFact) -> bool {
        self.facts.contains(&fact)
    }
}

/// One ranked match returned from a project graph query.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GraphQueryMatch {
    pub source_project_id: ProjectRuntimeProjectId,
    pub source_workspace_id: Option<ProjectRuntimeWorkspaceId>,
    pub source_worktree_id: Option<ProjectRuntimeWorktreeId>,
    pub source_root_session_id: Option<ProjectRuntimeRootSessionId>,
    pub source_session_id: Option<ProjectRuntimeSessionId>,
    pub source_agent_id: Option<ProjectRuntimeAgentId>,
    pub content_id: Option<ProjectRuntimeContentId>,
    pub memory_id: Option<ProjectRuntimeMemoryId>,
    pub tool_capability_id: Option<ProjectRuntimeToolCapabilityId>,
    pub evidence_id: Option<ProjectRuntimeEvidenceId>,
    pub receipt_id: Option<ProjectRuntimeReceiptId>,
    pub source_anchor_id: Option<ProjectRuntimeSourceAnchorId>,
    pub relationship: GraphQueryMatchRelationship,
    pub score_basis_points: GraphQueryScoreBasisPoints,
    pub summary: String,
}

impl GraphQueryMatch {
    pub fn new(
        source_project_id: impl Into<String>,
        summary: impl Into<String>,
        score_basis_points: impl Into<GraphQueryScoreBasisPoints>,
    ) -> Self {
        Self {
            source_project_id: ProjectRuntimeProjectId::new(source_project_id),
            source_workspace_id: None,
            source_worktree_id: None,
            source_root_session_id: None,
            source_session_id: None,
            source_agent_id: None,
            content_id: None,
            memory_id: None,
            tool_capability_id: None,
            evidence_id: None,
            receipt_id: None,
            source_anchor_id: None,
            relationship: GraphQueryMatchRelationship::default(),
            score_basis_points: score_basis_points.into(),
            summary: summary.into(),
        }
    }

    pub fn with_source_workspace(mut self, workspace_id: impl Into<String>) -> Self {
        self.source_workspace_id = Some(ProjectRuntimeWorkspaceId::new(workspace_id));
        self
    }

    pub fn with_source_worktree(mut self, worktree_id: impl Into<String>) -> Self {
        self.source_worktree_id = Some(ProjectRuntimeWorktreeId::new(worktree_id));
        self
    }

    pub fn with_source_root_session(mut self, root_session_id: impl Into<String>) -> Self {
        self.source_root_session_id = Some(ProjectRuntimeRootSessionId::new(root_session_id));
        self
    }

    pub fn with_source_session(mut self, session_id: impl Into<String>) -> Self {
        self.source_session_id = Some(ProjectRuntimeSessionId::new(session_id));
        self
    }

    pub fn with_source_agent(mut self, agent_id: impl Into<String>) -> Self {
        self.source_agent_id = Some(ProjectRuntimeAgentId::new(agent_id));
        self
    }

    pub fn with_content(mut self, content_id: impl Into<String>) -> Self {
        self.content_id = Some(ProjectRuntimeContentId::new(content_id));
        self
    }

    pub fn with_memory(mut self, memory_id: impl Into<String>) -> Self {
        self.memory_id = Some(ProjectRuntimeMemoryId::new(memory_id));
        self
    }

    pub fn with_tool_capability(mut self, capability_id: impl Into<String>) -> Self {
        self.tool_capability_id = Some(ProjectRuntimeToolCapabilityId::new(capability_id));
        self
    }

    pub fn with_evidence(mut self, evidence_id: impl Into<String>) -> Self {
        self.evidence_id = Some(ProjectRuntimeEvidenceId::new(evidence_id));
        self
    }

    pub fn with_receipt(mut self, receipt_id: impl Into<String>) -> Self {
        self.receipt_id = Some(ProjectRuntimeReceiptId::new(receipt_id));
        self
    }

    pub fn with_source_anchor(mut self, source_anchor_id: impl Into<String>) -> Self {
        self.source_anchor_id = Some(ProjectRuntimeSourceAnchorId::new(source_anchor_id));
        self
    }

    pub fn with_relationship(mut self, relationship: GraphQueryMatchRelationship) -> Self {
        self.relationship = relationship;
        self
    }
}

/// Replayable response receipt for one project runtime graph query.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GraphQueryResponse {
    pub receipt_id: ProjectRuntimeReceiptId,
    pub request: GraphQueryRequest,
    #[serde(default)]
    pub matches: Vec<GraphQueryMatch>,
}

impl GraphQueryResponse {
    pub fn new(receipt_id: impl Into<String>, request: GraphQueryRequest) -> Self {
        Self {
            receipt_id: ProjectRuntimeReceiptId::new(receipt_id),
            request,
            matches: Vec::new(),
        }
    }

    pub fn with_match(mut self, query_match: GraphQueryMatch) -> Self {
        self.matches.push(query_match);
        self
    }
}

/// Tool-specific context card consumed before backend policy selection.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProjectRuntimeToolCapabilityCard {
    pub graph_match: GraphQueryMatch,
    #[serde(default)]
    pub required_receipt_ids: Vec<ProjectRuntimeReceiptId>,
    #[serde(default)]
    pub required_capability_ids: Vec<ProjectRuntimeToolCapabilityId>,
    #[serde(default)]
    pub isolation_requirement_ids: Vec<ProjectRuntimeIsolationRequirementId>,
    #[serde(default)]
    pub backend_requirement_ids: Vec<ProjectRuntimeBackendRequirementId>,
}

impl ProjectRuntimeToolCapabilityCard {
    pub fn new(graph_match: GraphQueryMatch) -> Self {
        Self {
            graph_match,
            required_receipt_ids: Vec::new(),
            required_capability_ids: Vec::new(),
            isolation_requirement_ids: Vec::new(),
            backend_requirement_ids: Vec::new(),
        }
    }

    pub fn with_required_receipt(mut self, receipt_id: impl Into<String>) -> Self {
        self.required_receipt_ids
            .push(ProjectRuntimeReceiptId::new(receipt_id));
        self
    }

    pub fn with_required_capability(mut self, capability_id: impl Into<String>) -> Self {
        self.required_capability_ids
            .push(ProjectRuntimeToolCapabilityId::new(capability_id));
        self
    }

    pub fn with_isolation_requirement(mut self, requirement_id: impl Into<String>) -> Self {
        self.isolation_requirement_ids
            .push(ProjectRuntimeIsolationRequirementId::new(requirement_id));
        self
    }

    pub fn with_backend_requirement(mut self, requirement_id: impl Into<String>) -> Self {
        self.backend_requirement_ids
            .push(ProjectRuntimeBackendRequirementId::new(requirement_id));
        self
    }
}
