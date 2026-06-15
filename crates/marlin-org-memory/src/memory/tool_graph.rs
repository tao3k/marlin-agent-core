//! Project-scoped tool capability graph projection over structured `Org` nodes.

use marlin_agent_protocol::{
    GraphQueryExternalProjectPolicy, GraphQueryFallbackScope, GraphQueryFamily, GraphQueryMatch,
    GraphQueryMatchRelationship, GraphQueryRelationshipFact, GraphQueryRequest,
    GraphQueryScoreBasisPoints, GraphQueryVisibleSurface,
};
use marlin_org_model::{LinkKind, OrgNode, OrgSourceSpan};

/// `Org` property key that stores the stable tool capability id.
pub const TOOL_CAPABILITY_ID_PROPERTY: &str = "TOOL_CAPABILITY_ID";
/// `Org` property key that scopes a tool capability to a project.
pub const TOOL_CAPABILITY_PROJECT_ID_PROPERTY: &str = "PROJECT_ID";
/// `Org` property key that scopes a tool capability to a workspace.
pub const TOOL_CAPABILITY_WORKSPACE_ID_PROPERTY: &str = "WORKSPACE_ID";
/// `Org` property key that scopes a tool capability to a worktree.
pub const TOOL_CAPABILITY_WORKTREE_ID_PROPERTY: &str = "WORKTREE_ID";
/// `Org` property key that scopes a tool capability to a root session.
pub const TOOL_CAPABILITY_ROOT_SESSION_ID_PROPERTY: &str = "ROOT_SESSION_ID";
/// `Org` property key that scopes a tool capability to a session.
pub const TOOL_CAPABILITY_SESSION_ID_PROPERTY: &str = "SESSION_ID";
/// `Org` property key that lists receipts required before using a capability.
pub const TOOL_CAPABILITY_REQUIRED_RECEIPTS_PROPERTY: &str = "REQUIRED_RECEIPTS";
/// `Org` property key that records whether the capability contract is validated.
pub const TOOL_CAPABILITY_CONTRACT_VALIDATED_PROPERTY: &str = "CONTRACT_VALIDATED";

const EXPLICIT_BACKLINK_PROPERTY: &str = "EXPLICIT_BACKLINK";
const TOOL_CAPABILITY_SCOPE_PROPERTY_KEYS: &[&str] = &[
    TOOL_CAPABILITY_PROJECT_ID_PROPERTY,
    TOOL_CAPABILITY_WORKSPACE_ID_PROPERTY,
    TOOL_CAPABILITY_WORKTREE_ID_PROPERTY,
    TOOL_CAPABILITY_ROOT_SESSION_ID_PROPERTY,
    TOOL_CAPABILITY_SESSION_ID_PROPERTY,
];

pub(super) fn tool_capability_matches<'a>(
    nodes: impl IntoIterator<Item = &'a OrgNode>,
    request: &GraphQueryRequest,
) -> Vec<GraphQueryMatch> {
    if request.family != GraphQueryFamily::Tool
        || !request
            .context
            .visibility
            .allows_surface(GraphQueryVisibleSurface::Tools)
    {
        return Vec::new();
    }

    let mut matches = nodes
        .into_iter()
        .filter(|node| is_tool_capability_node(node))
        .filter(|node| matches_capability_anchor(node, request))
        .filter(|node| matches_query(node, &request.query))
        .filter_map(|node| tool_capability_match(node, request))
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

fn tool_capability_match(node: &OrgNode, request: &GraphQueryRequest) -> Option<GraphQueryMatch> {
    let source_project_id = property(node, TOOL_CAPABILITY_PROJECT_ID_PROPERTY)?;
    let capability_id = property(node, TOOL_CAPABILITY_ID_PROPERTY)?;
    let mut relationship = Vec::new();

    if source_project_id == request.context.project_id.as_str() {
        relationship.push(GraphQueryRelationshipFact::SameProject);
    } else {
        relationship.push(GraphQueryRelationshipFact::ExternalProject);
    }

    push_if_same(
        &mut relationship,
        GraphQueryRelationshipFact::SameWorkspace,
        property(node, TOOL_CAPABILITY_WORKSPACE_ID_PROPERTY),
        request.context.workspace_id.as_ref().map(|id| id.as_str()),
    );
    push_if_same(
        &mut relationship,
        GraphQueryRelationshipFact::SameWorktreeProvenance,
        property(node, TOOL_CAPABILITY_WORKTREE_ID_PROPERTY),
        request.context.worktree_id.as_ref().map(|id| id.as_str()),
    );
    push_if_same(
        &mut relationship,
        GraphQueryRelationshipFact::SameRootSession,
        property(node, TOOL_CAPABILITY_ROOT_SESSION_ID_PROPERTY),
        request
            .context
            .root_session_id
            .as_ref()
            .map(|id| id.as_str()),
    );
    push_if_same(
        &mut relationship,
        GraphQueryRelationshipFact::SameSessionLineage,
        property(node, TOOL_CAPABILITY_SESSION_ID_PROPERTY),
        request.context.session_id.as_ref().map(|id| id.as_str()),
    );

    if property_is_truthy(node, EXPLICIT_BACKLINK_PROPERTY) {
        relationship.push(GraphQueryRelationshipFact::ExplicitBacklink);
    }
    if property_is_truthy(node, TOOL_CAPABILITY_CONTRACT_VALIDATED_PROPERTY) {
        relationship.push(GraphQueryRelationshipFact::ContractValidated);
    }

    let score = score_basis_points(&relationship);
    if !is_allowed_by_scope(&relationship, request, score) {
        return None;
    }

    let mut query_match = GraphQueryMatch::new(source_project_id, compact_summary(node), score)
        .with_tool_capability(capability_id)
        .with_source_anchor(node.id.as_str())
        .with_relationship(GraphQueryMatchRelationship::new(relationship));

    if let Some(workspace_id) = property(node, TOOL_CAPABILITY_WORKSPACE_ID_PROPERTY) {
        query_match = query_match.with_source_workspace(workspace_id);
    }
    if let Some(worktree_id) = property(node, TOOL_CAPABILITY_WORKTREE_ID_PROPERTY) {
        query_match = query_match.with_source_worktree(worktree_id);
    }
    if let Some(root_session_id) = property(node, TOOL_CAPABILITY_ROOT_SESSION_ID_PROPERTY) {
        query_match = query_match.with_source_root_session(root_session_id);
    }
    if let Some(session_id) = property(node, TOOL_CAPABILITY_SESSION_ID_PROPERTY) {
        query_match = query_match.with_source_session(session_id);
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

fn is_tool_capability_node(node: &OrgNode) -> bool {
    node.properties.contains_key(TOOL_CAPABILITY_ID_PROPERTY)
}

fn matches_capability_anchor(node: &OrgNode, request: &GraphQueryRequest) -> bool {
    request.capability_id.as_ref().is_none_or(|capability_id| {
        property(node, TOOL_CAPABILITY_ID_PROPERTY) == Some(capability_id.as_str())
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
        if !TOOL_CAPABILITY_SCOPE_PROPERTY_KEYS.contains(&key.as_str()) {
            push_search_text(&mut text, key);
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
    push_search_text(
        text,
        &format!(
            "{}:{}-{}",
            source.document, source.start_line, source.end_line
        ),
    );
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
