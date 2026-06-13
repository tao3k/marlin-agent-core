//! Projection from protocol model-route rules into native Deck policies.

use marlin_agent_protocol::{ModelCommandMatcher, ModelRouteRequest, ModelRouteRule};
use marlin_gerbil_scheme::{
    GerbilDeckRuntimeModelRoutePolicy, GerbilDeckRuntimeModelRoutePolicyRequest,
};

#[derive(Clone, Debug)]
pub(crate) struct DeckRuntimeNativePolicySet {
    rules: Vec<ModelRouteRule>,
    native_order: Vec<usize>,
}

impl DeckRuntimeNativePolicySet {
    pub(crate) fn new(rules: Vec<ModelRouteRule>) -> Self {
        let mut native_order = rules
            .iter()
            .enumerate()
            .map(|(index, rule)| (index, rule.priority))
            .collect::<Vec<_>>();
        native_order.sort_by(|left, right| right.1.cmp(&left.1).then(left.0.cmp(&right.0)));

        Self {
            rules,
            native_order: native_order
                .into_iter()
                .map(|(index, _priority)| index)
                .collect(),
        }
    }

    pub(crate) fn rules(&self) -> &[ModelRouteRule] {
        &self.rules
    }

    pub(crate) fn original_index_for_native(&self, native_policy_index: usize) -> Option<usize> {
        self.native_order.get(native_policy_index).copied()
    }

    pub(crate) fn native_policies_len(&self) -> usize {
        self.native_order.len()
    }

    pub(crate) fn native_request_for(
        &self,
        request: &ModelRouteRequest,
    ) -> GerbilDeckRuntimeModelRoutePolicyRequest {
        GerbilDeckRuntimeModelRoutePolicyRequest::from_model_route_request(
            self.native_order
                .iter()
                .map(|index| native_policy_from_rule(&self.rules[*index], request)),
            request,
        )
    }
}

fn native_policy_from_rule(
    rule: &ModelRouteRule,
    request: &ModelRouteRequest,
) -> GerbilDeckRuntimeModelRoutePolicy {
    let mut policy = GerbilDeckRuntimeModelRoutePolicy::new(
        rule.rule_id.as_str(),
        rule.endpoint.provider.as_str(),
        rule.endpoint.model.as_str(),
    );

    for prefix in command_prefixes_from_matcher(&rule.matcher) {
        policy = policy.with_command_prefix(prefix);
    }
    for scope in agent_scopes_from_matcher(&rule.matcher, request) {
        policy = policy.with_agent_scope(scope);
    }

    policy
}

fn command_prefixes_from_matcher(matcher: &ModelCommandMatcher) -> Vec<String> {
    let has_command_dimensions =
        !matcher.argv_globs.is_empty() || !matcher.executable_globs.is_empty();
    let mut prefixes = matcher
        .argv_globs
        .iter()
        .chain(matcher.executable_globs.iter())
        .filter_map(|pattern| native_prefix_from_glob(pattern))
        .collect::<Vec<_>>();

    if prefixes.is_empty() && !has_command_dimensions {
        prefixes.push(String::new());
    }

    prefixes.sort();
    prefixes.dedup();
    prefixes
}

fn agent_scopes_from_matcher(
    matcher: &ModelCommandMatcher,
    request: &ModelRouteRequest,
) -> Vec<String> {
    let request_scope = request_agent_scope_label(request);
    let has_scope_dimensions =
        !matcher.agent_scope_globs.is_empty() || !matcher.sub_agent_role_globs.is_empty();
    let mut scopes = matcher
        .agent_scope_globs
        .iter()
        .chain(matcher.sub_agent_role_globs.iter())
        .filter_map(|pattern| native_scope_from_glob(pattern, request_scope.as_str()))
        .collect::<Vec<_>>();

    if scopes.is_empty() && !has_scope_dimensions {
        scopes.push(request_scope);
    }

    scopes.sort();
    scopes.dedup();
    scopes
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

fn native_scope_from_glob(pattern: &str, request_scope: &str) -> Option<String> {
    if pattern == "*" {
        return Some(request_scope.to_string());
    }
    if pattern.chars().any(is_glob_metacharacter) {
        return None;
    }

    Some(pattern.to_string())
}

fn request_agent_scope_label(request: &ModelRouteRequest) -> String {
    request
        .agent_scope
        .as_ref()
        .map(|scope| scope.as_str().to_string())
        .or_else(|| request.sub_agent_role.clone())
        .unwrap_or_else(|| "any".to_string())
}

fn is_glob_metacharacter(character: char) -> bool {
    matches!(character, '*' | '?' | '[' | ']' | '{' | '}')
}
