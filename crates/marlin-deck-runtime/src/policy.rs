//! Projection from protocol model-route rules into native Deck policies.

use std::collections::HashMap;

use marlin_agent_protocol::{ModelCommandMatcher, ModelRouteRequest, ModelRouteRule};
use marlin_gerbil_scheme::{
    GerbilDeckRuntimeModelRoutePolicy, GerbilDeckRuntimeModelRoutePolicyRequest,
};

#[derive(Clone, Debug)]
pub(crate) struct DeckRouteCandidates {
    rules: Vec<ModelRouteRule>,
    deck_candidates: Vec<DeckRouteCandidate>,
    matcher_plan: DeckCandidateIndex,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct DeckRouteCandidate {
    original_index: usize,
    name: String,
    provider: String,
    model: String,
    command_prefixes: Vec<String>,
    fixed_agent_scopes: Vec<String>,
    include_request_scope: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct DeckCandidateIndex {
    request_scope_policy_indices: Vec<usize>,
    fixed_scope_policy_indices: HashMap<String, Vec<usize>>,
}

impl DeckRouteCandidates {
    pub(crate) fn new(rules: Vec<ModelRouteRule>) -> Self {
        let mut deck_candidates = rules
            .iter()
            .enumerate()
            .map(|(index, rule)| (index, rule.priority))
            .collect::<Vec<_>>();
        deck_candidates.sort_by(|left, right| right.1.cmp(&left.1).then(left.0.cmp(&right.0)));
        let deck_candidates = deck_candidates
            .into_iter()
            .map(|(index, _priority)| candidate_from_rule(index, &rules[index]))
            .collect::<Vec<_>>();
        let matcher_plan = DeckCandidateIndex::from_candidates(deck_candidates.as_slice());

        Self {
            rules,
            deck_candidates,
            matcher_plan,
        }
    }

    pub(crate) fn rules(&self) -> &[ModelRouteRule] {
        &self.rules
    }

    pub(crate) fn original_index(&self, candidate_index: usize) -> Option<usize> {
        self.deck_candidates
            .get(candidate_index)
            .map(|policy| policy.original_index)
    }

    pub(crate) fn candidate_count(&self) -> usize {
        self.deck_candidates.len()
    }

    pub(crate) fn try_for_each_candidate<T, E>(
        &self,
        request: &ModelRouteRequest,
        visit: impl FnMut(usize) -> Result<Option<T>, E>,
    ) -> Result<Option<T>, E> {
        let command = request.command_line();
        let request_scope_labels = request_agent_scope_labels(request);
        self.matcher_plan.try_for_each_candidate(
            self.deck_candidates.as_slice(),
            command.as_str(),
            request_scope_labels.as_slice(),
            visit,
        )
    }

    pub(crate) fn project_candidate_request(
        &self,
        request: &ModelRouteRequest,
        candidate_index: usize,
    ) -> Option<GerbilDeckRuntimeModelRoutePolicyRequest> {
        let policy =
            self.project_candidate(candidate_index, &request_agent_scope_label(request))?;
        Some(GerbilDeckRuntimeModelRoutePolicyRequest::from_model_route_request([policy], request))
    }

    pub(crate) fn project_candidate(
        &self,
        index: usize,
        scope: &str,
    ) -> Option<GerbilDeckRuntimeModelRoutePolicy> {
        self.deck_candidates
            .get(index)
            .map(|policy| project_candidate(policy, scope))
    }
}

impl DeckCandidateIndex {
    fn from_candidates(policies: &[DeckRouteCandidate]) -> Self {
        let mut plan = Self::default();

        for (candidate_index, policy) in policies.iter().enumerate() {
            if policy.include_request_scope {
                plan.request_scope_policy_indices.push(candidate_index);
            }
            for scope in &policy.fixed_agent_scopes {
                plan.fixed_scope_policy_indices
                    .entry(scope.clone())
                    .or_default()
                    .push(candidate_index);
            }
        }

        plan
    }

    fn try_for_each_candidate<T, E>(
        &self,
        policies: &[DeckRouteCandidate],
        command: &str,
        agent_scope_labels: &[String],
        mut visit: impl FnMut(usize) -> Result<Option<T>, E>,
    ) -> Result<Option<T>, E> {
        let request_scope_indices = self.request_scope_policy_indices.as_slice();
        let fixed_scope_indices = agent_scope_labels
            .iter()
            .filter_map(|agent_scope| self.fixed_scope_policy_indices.get(agent_scope))
            .map(Vec::as_slice)
            .collect::<Vec<_>>();
        let mut request_position = 0;
        let mut fixed_positions = vec![0; fixed_scope_indices.len()];

        loop {
            let next_request_index = request_scope_indices.get(request_position).copied();
            let next_fixed_index = fixed_scope_indices
                .iter()
                .zip(fixed_positions.iter())
                .filter_map(|(indices, position)| indices.get(*position).copied())
                .min();
            let Some(next_index) = next_request_index.into_iter().chain(next_fixed_index).min()
            else {
                return Ok(None);
            };

            while request_scope_indices.get(request_position).copied() == Some(next_index) {
                request_position += 1;
            }
            for (indices, position) in fixed_scope_indices.iter().zip(fixed_positions.iter_mut()) {
                while indices.get(*position).copied() == Some(next_index) {
                    *position += 1;
                }
            }

            if policies
                .get(next_index)
                .is_some_and(|policy| policy.command_matches(command))
                && let Some(output) = visit(next_index)?
            {
                return Ok(Some(output));
            }
        }
    }
}

impl DeckRouteCandidate {
    fn command_matches(&self, command: &str) -> bool {
        self.command_prefixes
            .iter()
            .any(|prefix| command.starts_with(prefix))
    }
}

fn candidate_from_rule(original_index: usize, rule: &ModelRouteRule) -> DeckRouteCandidate {
    let scope_projection = agent_scope_projection_from_matcher(&rule.matcher);

    DeckRouteCandidate {
        original_index,
        name: rule.rule_id.as_str().to_string(),
        provider: rule.endpoint.provider.as_str().to_string(),
        model: rule.endpoint.model.as_str().to_string(),
        command_prefixes: command_prefixes_from_matcher(&rule.matcher),
        fixed_agent_scopes: scope_projection.fixed_agent_scopes,
        include_request_scope: scope_projection.include_request_scope,
    }
}

fn project_candidate(
    compiled: &DeckRouteCandidate,
    request_scope: &str,
) -> GerbilDeckRuntimeModelRoutePolicy {
    let mut policy = GerbilDeckRuntimeModelRoutePolicy::new(
        compiled.name.as_str(),
        compiled.provider.as_str(),
        compiled.model.as_str(),
    );

    for prefix in &compiled.command_prefixes {
        policy = policy.with_command_prefix(prefix.as_str());
    }
    let mut agent_scopes = compiled.fixed_agent_scopes.clone();
    agent_scopes.push(request_scope.to_string());
    agent_scopes.sort();
    agent_scopes.dedup();
    for scope in agent_scopes {
        policy = policy.with_agent_scope(scope);
    }

    policy
}

fn command_prefixes_from_matcher(matcher: &ModelCommandMatcher) -> Vec<String> {
    let has_command_dimensions =
        !matcher.argv_globs.is_empty() || !matcher.executable_globs.is_empty();
    let native_prefix_projections = matcher
        .argv_globs
        .iter()
        .chain(matcher.executable_globs.iter())
        .map(|pattern| native_prefix_from_glob(pattern))
        .collect::<Vec<_>>();
    let has_unsupported_command_projection = native_prefix_projections.iter().any(Option::is_none);
    let mut prefixes = native_prefix_projections
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

    if !has_command_dimensions || has_unsupported_command_projection {
        prefixes.push(String::new());
    }

    prefixes.sort();
    prefixes.dedup();
    prefixes
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CompiledAgentScopeProjection {
    fixed_agent_scopes: Vec<String>,
    include_request_scope: bool,
}

fn agent_scope_projection_from_matcher(
    matcher: &ModelCommandMatcher,
) -> CompiledAgentScopeProjection {
    let has_scope_dimensions =
        !matcher.agent_scope_globs.is_empty() || !matcher.sub_agent_role_globs.is_empty();
    let projections = matcher
        .agent_scope_globs
        .iter()
        .chain(matcher.sub_agent_role_globs.iter())
        .map(|pattern| deck_scope_projection_from_glob(pattern))
        .collect::<Vec<_>>();
    let include_request_scope = !has_scope_dimensions
        || projections.iter().any(|projection| {
            matches!(
                projection,
                DeckScopeProjection::RequestScope | DeckScopeProjection::Unsupported
            )
        });
    let mut fixed_agent_scopes = projections
        .into_iter()
        .filter_map(|projection| match projection {
            DeckScopeProjection::Fixed(scope) => Some(scope),
            DeckScopeProjection::RequestScope | DeckScopeProjection::Unsupported => None,
        })
        .collect::<Vec<_>>();

    fixed_agent_scopes.sort();
    fixed_agent_scopes.dedup();

    CompiledAgentScopeProjection {
        fixed_agent_scopes,
        include_request_scope,
    }
}

fn native_prefix_from_glob(pattern: &str) -> Option<String> {
    if pattern == "*" {
        return Some(String::new());
    }
    let Some((index, character)) = pattern
        .char_indices()
        .find(|(_, character)| is_glob_metacharacter(*character))
    else {
        return Some(pattern.to_string());
    };

    if character == '*' && index > 0 && index + character.len_utf8() == pattern.len() {
        return Some(pattern[..index].to_string());
    }

    None
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum DeckScopeProjection {
    RequestScope,
    Fixed(String),
    Unsupported,
}

fn deck_scope_projection_from_glob(pattern: &str) -> DeckScopeProjection {
    if pattern == "*" {
        return DeckScopeProjection::RequestScope;
    }
    if pattern.chars().any(is_glob_metacharacter) {
        return DeckScopeProjection::Unsupported;
    }

    DeckScopeProjection::Fixed(pattern.to_string())
}

fn request_agent_scope_label(request: &ModelRouteRequest) -> String {
    request
        .agent_scope
        .as_ref()
        .map(|scope| scope.as_str().to_string())
        .or_else(|| request.sub_agent_role.clone())
        .unwrap_or_else(|| "any".to_string())
}

fn request_agent_scope_labels(request: &ModelRouteRequest) -> Vec<String> {
    let mut labels = Vec::new();
    if let Some(scope) = &request.agent_scope {
        labels.push(scope.as_str().to_string());
    }
    if let Some(role) = &request.sub_agent_role {
        labels.push(role.clone());
    }
    if labels.is_empty() {
        labels.push("any".to_string());
    }
    labels.sort();
    labels.dedup();
    labels
}

fn is_glob_metacharacter(character: char) -> bool {
    matches!(character, '*' | '?' | '[' | ']' | '{' | '}')
}
