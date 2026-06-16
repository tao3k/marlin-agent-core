//! Project-scoped topology overview projection over structured `Org` nodes.

use marlin_agent_protocol::{
    GraphQueryExternalProjectPolicy, GraphQueryFallbackScope, GraphQueryFamily, GraphQueryMatch,
    GraphQueryMatchRelationship, GraphQueryRelationshipFact, GraphQueryRequest,
    GraphQueryScoreBasisPoints, GraphQueryVisibleSurface,
};
use marlin_org_model::{LinkKind, OrgNode, OrgSourceSpan};

/// `Org` property key carrying the stable topology node id.
pub const TOPOLOGY_ID_PROPERTY: &str = "TOPOLOGY_ID";
/// `Org` property key scoping topology to a project.
pub const TOPOLOGY_PROJECT_ID_PROPERTY: &str = "PROJECT_ID";
/// `Org` property key scoping topology to a workspace.
pub const TOPOLOGY_WORKSPACE_ID_PROPERTY: &str = "WORKSPACE_ID";
/// `Org` property key scoping topology to a worktree.
pub const TOPOLOGY_WORKTREE_ID_PROPERTY: &str = "WORKTREE_ID";
/// `Org` property key scoping topology to a root session.
pub const TOPOLOGY_ROOT_SESSION_ID_PROPERTY: &str = "ROOT_SESSION_ID";
/// `Org` property key scoping topology to a runtime session.
pub const TOPOLOGY_SESSION_ID_PROPERTY: &str = "SESSION_ID";
/// `Org` property key carrying the agent associated with a topology edge.
pub const TOPOLOGY_AGENT_ID_PROPERTY: &str = "AGENT_ID";
/// `Org` property key carrying the content anchor associated with a topology edge.
pub const TOPOLOGY_CONTENT_ID_PROPERTY: &str = "CONTENT_ID";
/// `Org` property key declaring the topology overview scope.
pub const TOPOLOGY_SCOPE_PROPERTY: &str = "TOPOLOGY_SCOPE";
/// `Org` property key declaring the coarse topology node kind.
pub const TOPOLOGY_NODE_KIND_PROPERTY: &str = "TOPOLOGY_NODE_KIND";
/// `Org` property key declaring the coarse topology edge kind.
pub const TOPOLOGY_EDGE_KIND_PROPERTY: &str = "TOPOLOGY_EDGE_KIND";
/// `Org` property key pointing back to the detailed Org fact layer.
pub const TOPOLOGY_SOURCE_ANCHOR_PROPERTY: &str = "SOURCE_ANCHOR";
/// `Org` property key recording whether the topology contract is validated.
pub const TOPOLOGY_CONTRACT_VALIDATED_PROPERTY: &str = "CONTRACT_VALIDATED";

const EXPLICIT_BACKLINK_PROPERTY: &str = "EXPLICIT_BACKLINK";
const TOPOLOGY_SCOPE_PROPERTY_KEYS: &[&str] = &[
    TOPOLOGY_ID_PROPERTY,
    TOPOLOGY_PROJECT_ID_PROPERTY,
    TOPOLOGY_WORKSPACE_ID_PROPERTY,
    TOPOLOGY_WORKTREE_ID_PROPERTY,
    TOPOLOGY_ROOT_SESSION_ID_PROPERTY,
    TOPOLOGY_SESSION_ID_PROPERTY,
    TOPOLOGY_AGENT_ID_PROPERTY,
    TOPOLOGY_CONTENT_ID_PROPERTY,
    TOPOLOGY_SCOPE_PROPERTY,
    TOPOLOGY_NODE_KIND_PROPERTY,
    TOPOLOGY_EDGE_KIND_PROPERTY,
    TOPOLOGY_SOURCE_ANCHOR_PROPERTY,
];

pub(super) fn topology_matches<'a>(
    nodes: impl IntoIterator<Item = &'a OrgNode>,
    request: &GraphQueryRequest,
) -> Vec<GraphQueryMatch> {
    if request.family != GraphQueryFamily::Topology
        || !request
            .context
            .visibility
            .allows_surface(GraphQueryVisibleSurface::Topology)
    {
        return Vec::new();
    }

    let mut matches = nodes
        .into_iter()
        .filter(|node| is_topology_node(node))
        .filter(|node| matches_content_anchor(node, request))
        .filter(|node| matches_query(node, &request.query))
        .filter_map(|node| topology_match(node, request))
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

fn topology_match(node: &OrgNode, request: &GraphQueryRequest) -> Option<GraphQueryMatch> {
    let source_project_id = property(node, TOPOLOGY_PROJECT_ID_PROPERTY)?;
    let source_anchor = property(node, TOPOLOGY_SOURCE_ANCHOR_PROPERTY)?;
    let mut relationship = Vec::new();

    if source_project_id == request.context.project_id.as_str() {
        relationship.push(GraphQueryRelationshipFact::SameProject);
    } else {
        relationship.push(GraphQueryRelationshipFact::ExternalProject);
    }

    push_if_same(
        &mut relationship,
        GraphQueryRelationshipFact::SameWorkspace,
        property(node, TOPOLOGY_WORKSPACE_ID_PROPERTY),
        request.context.workspace_id.as_ref().map(|id| id.as_str()),
    );
    push_if_same(
        &mut relationship,
        GraphQueryRelationshipFact::SameWorktreeProvenance,
        property(node, TOPOLOGY_WORKTREE_ID_PROPERTY),
        request.context.worktree_id.as_ref().map(|id| id.as_str()),
    );
    push_if_same(
        &mut relationship,
        GraphQueryRelationshipFact::SameRootSession,
        property(node, TOPOLOGY_ROOT_SESSION_ID_PROPERTY),
        request
            .context
            .root_session_id
            .as_ref()
            .map(|id| id.as_str()),
    );
    push_if_same(
        &mut relationship,
        GraphQueryRelationshipFact::SameSessionLineage,
        property(node, TOPOLOGY_SESSION_ID_PROPERTY),
        request.context.session_id.as_ref().map(|id| id.as_str()),
    );
    push_if_same(
        &mut relationship,
        GraphQueryRelationshipFact::SameContentAncestry,
        property(node, TOPOLOGY_CONTENT_ID_PROPERTY),
        request
            .context
            .content_anchor
            .as_ref()
            .map(|id| id.as_str()),
    );

    if property_is_truthy(node, EXPLICIT_BACKLINK_PROPERTY) {
        relationship.push(GraphQueryRelationshipFact::ExplicitBacklink);
    }
    if property_is_truthy(node, TOPOLOGY_CONTRACT_VALIDATED_PROPERTY) {
        relationship.push(GraphQueryRelationshipFact::ContractValidated);
    }

    let score = score_basis_points(&relationship);
    if !is_allowed_by_scope(&relationship, request, score) {
        return None;
    }

    let mut query_match = GraphQueryMatch::new(source_project_id, compact_summary(node), score)
        .with_source_anchor(source_anchor)
        .with_relationship(GraphQueryMatchRelationship::new(relationship));

    if let Some(workspace_id) = property(node, TOPOLOGY_WORKSPACE_ID_PROPERTY) {
        query_match = query_match.with_source_workspace(workspace_id);
    }
    if let Some(worktree_id) = property(node, TOPOLOGY_WORKTREE_ID_PROPERTY) {
        query_match = query_match.with_source_worktree(worktree_id);
    }
    if let Some(root_session_id) = property(node, TOPOLOGY_ROOT_SESSION_ID_PROPERTY) {
        query_match = query_match.with_source_root_session(root_session_id);
    }
    if let Some(session_id) = property(node, TOPOLOGY_SESSION_ID_PROPERTY) {
        query_match = query_match.with_source_session(session_id);
    }
    if let Some(agent_id) = property(node, TOPOLOGY_AGENT_ID_PROPERTY) {
        query_match = query_match.with_source_agent(agent_id);
    }
    if let Some(content_id) = property(node, TOPOLOGY_CONTENT_ID_PROPERTY) {
        query_match = query_match.with_content(content_id);
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

fn is_topology_node(node: &OrgNode) -> bool {
    [
        TOPOLOGY_ID_PROPERTY,
        TOPOLOGY_PROJECT_ID_PROPERTY,
        TOPOLOGY_NODE_KIND_PROPERTY,
        TOPOLOGY_EDGE_KIND_PROPERTY,
        TOPOLOGY_SOURCE_ANCHOR_PROPERTY,
    ]
    .iter()
    .all(|key| node.properties.contains_key(*key))
}

fn matches_content_anchor(node: &OrgNode, request: &GraphQueryRequest) -> bool {
    request.content_id.as_ref().is_none_or(|content_id| {
        property(node, TOPOLOGY_CONTENT_ID_PROPERTY) == Some(content_id.as_str())
    })
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
        push_search_text(&mut text, title);
    }
    if let Some(body) = &node.body {
        push_search_text(&mut text, body);
    }

    for tag in &node.tags {
        push_search_text(&mut text, "tag");
        push_search_text(&mut text, tag);
        push_search_text(&mut text, &format!("tag:{tag}"));
    }

    for (key, value) in &node.properties {
        if !TOPOLOGY_SCOPE_PROPERTY_KEYS.contains(&key.as_str()) {
            push_search_text(&mut text, key);
            push_search_text(&mut text, value);
            push_search_text(&mut text, &format!("{key}:{value}"));
        }
    }

    for key in TOPOLOGY_SCOPE_PROPERTY_KEYS {
        if let Some(value) = property(node, key) {
            push_search_text(&mut text, value);
            push_search_text(&mut text, &format!("{key}:{value}"));
        }
    }

    for link in &node.links {
        push_search_text(&mut text, "link");
        push_link_kind_text(&mut text, &link.kind);
        push_search_text(&mut text, &link.target);
        if let Some(description) = &link.description {
            push_search_text(&mut text, description);
        }
    }

    if let Some(source) = &node.source {
        push_source_span_text(&mut text, source);
    }
    for source in node.tokens.property_values.values() {
        push_source_span_text(&mut text, source);
    }

    text.to_lowercase()
}

fn push_search_text(text: &mut String, value: &str) {
    text.push_str(value);
    text.push('\n');
}

fn push_link_kind_text(text: &mut String, kind: &LinkKind) {
    let label = match kind {
        LinkKind::Id => "id",
        LinkKind::File => "file",
        LinkKind::Url => "url",
        LinkKind::Custom(label) => label.as_str(),
    };
    push_search_text(text, label);
}

fn push_source_span_text(text: &mut String, source: &OrgSourceSpan) {
    push_search_text(text, "source");
    push_search_text(text, &source.document);
    push_search_text(text, &source.start_line.to_string());
    push_search_text(text, &source.end_line.to_string());
}

fn compact_summary(node: &OrgNode) -> String {
    node.title
        .as_deref()
        .or(node.body.as_deref())
        .unwrap_or_else(|| node.id.as_str())
        .lines()
        .next()
        .unwrap_or_else(|| node.id.as_str())
        .trim()
        .to_string()
}

fn property<'a>(node: &'a OrgNode, key: &str) -> Option<&'a str> {
    node.properties.get(key).map(String::as_str)
}

fn property_is_truthy(node: &OrgNode, key: &str) -> bool {
    property(node, key).is_some_and(|value| matches!(value, "true" | "yes" | "1"))
}

fn push_if_same(
    relationship: &mut Vec<GraphQueryRelationshipFact>,
    fact: GraphQueryRelationshipFact,
    left: Option<&str>,
    right: Option<&str>,
) {
    if left.zip(right).is_some_and(|(left, right)| left == right) {
        relationship.push(fact);
    }
}
