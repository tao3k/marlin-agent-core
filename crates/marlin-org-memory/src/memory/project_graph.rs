//! Project-scoped memory graph projection over structured `Org` nodes.

use marlin_agent_protocol::{
    GraphQueryExternalProjectPolicy, GraphQueryFallbackScope, GraphQueryFamily, GraphQueryMatch,
    GraphQueryMatchRelationship, GraphQueryRelationshipFact, GraphQueryRequest,
    GraphQueryScoreBasisPoints, GraphQueryVisibleSurface,
};
use marlin_org_model::OrgNode;

/// `Org` property carrying the project scope for a projected memory fact.
pub const PROJECT_MEMORY_PROJECT_ID_PROPERTY: &str = "PROJECT_ID";
/// `Org` property carrying the source workspace for a projected memory fact.
pub const PROJECT_MEMORY_WORKSPACE_ID_PROPERTY: &str = "WORKSPACE_ID";
/// `Org` property carrying the worktree provenance for a projected memory fact.
pub const PROJECT_MEMORY_WORKTREE_ID_PROPERTY: &str = "WORKTREE_ID";
/// `Org` property carrying the root session that produced a memory fact.
pub const PROJECT_MEMORY_ROOT_SESSION_ID_PROPERTY: &str = "ROOT_SESSION_ID";
/// `Org` property carrying the runtime session that produced a memory fact.
pub const PROJECT_MEMORY_SESSION_ID_PROPERTY: &str = "SESSION_ID";
/// `Org` property carrying the content anchor for a projected memory fact.
pub const PROJECT_MEMORY_CONTENT_ID_PROPERTY: &str = "CONTENT_ID";
/// `Org` property carrying the stable project memory identifier.
pub const PROJECT_MEMORY_ID_PROPERTY: &str = "MEMORY_ID";
/// `Org` property carrying recall terms for hybrid memory frontier generation.
pub const PROJECT_MEMORY_RECALL_QUERY_PROPERTY: &str = "RECALL_QUERY";
/// `Org` property indicating that the memory fact passed contract validation.
pub const PROJECT_MEMORY_CONTRACT_VALIDATED_PROPERTY: &str = "CONTRACT_VALIDATED";

const MEMORY_DISPATCH_PROPERTY: &str = "MEMORY_DISPATCH";
const EXPLICIT_BACKLINK_PROPERTY: &str = "EXPLICIT_BACKLINK";
const PROJECT_MEMORY_FRONTIER_PROPERTY_KEYS: &[&str] = &[
    PROJECT_MEMORY_RECALL_QUERY_PROPERTY,
    PROJECT_MEMORY_ID_PROPERTY,
    MEMORY_DISPATCH_PROPERTY,
    EXPLICIT_BACKLINK_PROPERTY,
];

pub(super) fn project_memory_matches<'a>(
    nodes: impl IntoIterator<Item = &'a OrgNode>,
    request: &GraphQueryRequest,
) -> Vec<GraphQueryMatch> {
    if request.family != GraphQueryFamily::Memory
        || !request
            .context
            .visibility
            .allows_surface(GraphQueryVisibleSurface::Memory)
    {
        return Vec::new();
    }

    let mut matches = nodes
        .into_iter()
        .filter(|node| is_project_memory_node(node))
        .filter(|node| matches_query(node, &request.query))
        .filter_map(|node| project_memory_match(node, request))
        .collect::<Vec<_>>();

    matches.sort_by(|left, right| {
        right
            .score_basis_points
            .as_u16()
            .cmp(&left.score_basis_points.as_u16())
            .then_with(|| left.summary.cmp(&right.summary))
    });

    if let Some(limit) = request.limit {
        matches.truncate(limit.as_u16() as usize);
    }

    matches
}

fn project_memory_match(node: &OrgNode, request: &GraphQueryRequest) -> Option<GraphQueryMatch> {
    let source_project_id = property(node, PROJECT_MEMORY_PROJECT_ID_PROPERTY)?;
    let mut relationship = Vec::new();

    if source_project_id == request.context.project_id.as_str() {
        relationship.push(GraphQueryRelationshipFact::SameProject);
    } else {
        relationship.push(GraphQueryRelationshipFact::ExternalProject);
    }

    push_if_same(
        &mut relationship,
        GraphQueryRelationshipFact::SameWorkspace,
        property(node, PROJECT_MEMORY_WORKSPACE_ID_PROPERTY),
        request.context.workspace_id.as_ref().map(|id| id.as_str()),
    );
    push_if_same(
        &mut relationship,
        GraphQueryRelationshipFact::SameWorktreeProvenance,
        property(node, PROJECT_MEMORY_WORKTREE_ID_PROPERTY),
        request.context.worktree_id.as_ref().map(|id| id.as_str()),
    );
    push_if_same(
        &mut relationship,
        GraphQueryRelationshipFact::SameRootSession,
        property(node, PROJECT_MEMORY_ROOT_SESSION_ID_PROPERTY),
        request
            .context
            .root_session_id
            .as_ref()
            .map(|id| id.as_str()),
    );
    push_if_same(
        &mut relationship,
        GraphQueryRelationshipFact::SameSessionLineage,
        property(node, PROJECT_MEMORY_SESSION_ID_PROPERTY),
        request.context.session_id.as_ref().map(|id| id.as_str()),
    );
    push_if_same(
        &mut relationship,
        GraphQueryRelationshipFact::SameContentAncestry,
        property(node, PROJECT_MEMORY_CONTENT_ID_PROPERTY),
        request
            .context
            .content_anchor
            .as_ref()
            .map(|id| id.as_str()),
    );

    if property_is_truthy(node, EXPLICIT_BACKLINK_PROPERTY) {
        relationship.push(GraphQueryRelationshipFact::ExplicitBacklink);
    }
    if property_is_truthy(node, PROJECT_MEMORY_CONTRACT_VALIDATED_PROPERTY) {
        relationship.push(GraphQueryRelationshipFact::ContractValidated);
    }

    let score = score_basis_points(&relationship);
    if !is_allowed_by_scope(&relationship, request, score) {
        return None;
    }

    let mut query_match = GraphQueryMatch::new(source_project_id, compact_summary(node), score)
        .with_relationship(GraphQueryMatchRelationship::new(relationship));

    if let Some(workspace_id) = property(node, PROJECT_MEMORY_WORKSPACE_ID_PROPERTY) {
        query_match = query_match.with_source_workspace(workspace_id);
    }
    if let Some(worktree_id) = property(node, PROJECT_MEMORY_WORKTREE_ID_PROPERTY) {
        query_match = query_match.with_source_worktree(worktree_id);
    }
    if let Some(root_session_id) = property(node, PROJECT_MEMORY_ROOT_SESSION_ID_PROPERTY) {
        query_match = query_match.with_source_root_session(root_session_id);
    }
    if let Some(session_id) = property(node, PROJECT_MEMORY_SESSION_ID_PROPERTY) {
        query_match = query_match.with_source_session(session_id);
    }
    if let Some(content_id) = property(node, PROJECT_MEMORY_CONTENT_ID_PROPERTY) {
        query_match = query_match.with_content(content_id);
    }
    if let Some(memory_id) = property(node, PROJECT_MEMORY_ID_PROPERTY) {
        query_match = query_match.with_memory(memory_id);
    }

    Some(query_match)
}

fn is_allowed_by_scope(
    relationship: &[GraphQueryRelationshipFact],
    request: &GraphQueryRequest,
    score: GraphQueryScoreBasisPoints,
) -> bool {
    if relationship.contains(&GraphQueryRelationshipFact::ExternalProject) {
        return match request.context.fallback_policy.external_projects {
            GraphQueryExternalProjectPolicy::Disabled => false,
            GraphQueryExternalProjectPolicy::Enabled {
                min_score_basis_points,
            } => score >= min_score_basis_points,
        };
    }

    let policy = &request.context.fallback_policy;
    (relationship.contains(&GraphQueryRelationshipFact::SameProject)
        && policy.includes_scope(GraphQueryFallbackScope::Project))
        || (relationship.contains(&GraphQueryRelationshipFact::SameWorkspace)
            && policy.includes_scope(GraphQueryFallbackScope::Workspace))
        || (relationship.contains(&GraphQueryRelationshipFact::SameWorktreeProvenance)
            && policy.includes_scope(GraphQueryFallbackScope::WorktreeProvenance))
        || ((relationship.contains(&GraphQueryRelationshipFact::SameRootSession)
            || relationship.contains(&GraphQueryRelationshipFact::SameSessionLineage))
            && policy.includes_scope(GraphQueryFallbackScope::SessionLocal))
}

fn score_basis_points(relationship: &[GraphQueryRelationshipFact]) -> GraphQueryScoreBasisPoints {
    let mut score = if relationship.contains(&GraphQueryRelationshipFact::ExternalProject) {
        2_000
    } else {
        7_000
    };

    score += relationship
        .iter()
        .map(relationship_score_bonus)
        .sum::<u16>();

    GraphQueryScoreBasisPoints::new(score.min(10_000))
}

fn relationship_score_bonus(fact: &GraphQueryRelationshipFact) -> u16 {
    match fact {
        GraphQueryRelationshipFact::SameWorkspace => 700,
        GraphQueryRelationshipFact::SameRootSession => 600,
        GraphQueryRelationshipFact::SameSessionLineage => 500,
        GraphQueryRelationshipFact::SameContentAncestry => 600,
        GraphQueryRelationshipFact::SameWorktreeProvenance => 400,
        GraphQueryRelationshipFact::ExplicitBacklink => 500,
        GraphQueryRelationshipFact::ContractValidated => 700,
        GraphQueryRelationshipFact::SameProject | GraphQueryRelationshipFact::ExternalProject => 0,
    }
}

fn is_project_memory_node(node: &OrgNode) -> bool {
    node.properties.contains_key(PROJECT_MEMORY_ID_PROPERTY)
        || node.properties.contains_key(MEMORY_DISPATCH_PROPERTY)
}

fn matches_query(node: &OrgNode, query: &str) -> bool {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return true;
    }

    let haystack = compact_search_text(node);
    query.split_whitespace().all(|term| haystack.contains(term))
}

fn compact_search_text(node: &OrgNode) -> String {
    let mut text = String::new();
    if let Some(title) = &node.title {
        text.push_str(title);
        text.push('\n');
    }
    if let Some(body) = &node.body {
        text.push_str(body);
        text.push('\n');
    }
    for key in PROJECT_MEMORY_FRONTIER_PROPERTY_KEYS {
        if let Some(value) = property(node, key) {
            text.push_str(value);
            text.push('\n');
        }
    }
    text.to_lowercase()
}

fn compact_summary(node: &OrgNode) -> String {
    node.title
        .as_ref()
        .or(node.body.as_ref())
        .map(|text| text.lines().next().unwrap_or_default().trim())
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| node.id.as_str())
        .to_string()
}

fn property<'a>(node: &'a OrgNode, key: &str) -> Option<&'a str> {
    node.properties.get(key).map(String::as_str)
}

fn property_is_truthy(node: &OrgNode, key: &str) -> bool {
    property(node, key).is_some_and(|value| {
        matches!(
            value.to_ascii_lowercase().as_str(),
            "true" | "yes" | "accepted" | "validated"
        )
    })
}

fn push_if_same(
    relationship: &mut Vec<GraphQueryRelationshipFact>,
    fact: GraphQueryRelationshipFact,
    source_value: Option<&str>,
    context_value: Option<&str>,
) {
    if source_value
        .zip(context_value)
        .is_some_and(|(left, right)| left == right)
    {
        relationship.push(fact);
    }
}
