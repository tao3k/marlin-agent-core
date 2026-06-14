use marlin_agent_protocol::{
    ModelCommandMatcher, ModelContextForkMode, ModelEndpoint, ModelRouteAgentScope,
    ModelRouteRequest, ModelRouteRule, ModelSessionLifecycle, ModelSessionPolicy,
};
use marlin_agent_runtime::CompiledModelRouteResolver;

#[test]
fn resolver_selects_highest_priority_glob_route() {
    let rules = vec![
        ModelRouteRule::new(
            "workspace-default",
            1,
            ModelCommandMatcher::new().with_argv_glob("*"),
            ModelEndpoint::new("openai", "gpt-5-mini"),
        ),
        ModelRouteRule::new(
            "cargo-test",
            50,
            ModelCommandMatcher::new()
                .with_executable_glob("cargo")
                .with_argv_glob("cargo test*"),
            ModelEndpoint::new("openai", "gpt-5-mini"),
        )
        .with_session(ModelSessionPolicy::persistent(
            "workspace:test-runner",
            ModelContextForkMode::ForkSnapshot,
        )),
    ];
    let resolver = CompiledModelRouteResolver::new(rules).expect("route rules compile");

    let decision = resolver
        .resolve(&ModelRouteRequest::command([
            "cargo",
            "test",
            "--workspace",
            "--no-fail-fast",
        ]))
        .expect("cargo test route resolves");

    assert_eq!(decision.endpoint.provider.as_str(), "openai");
    assert_eq!(decision.endpoint.model.as_str(), "gpt-5-mini");
    assert_eq!(decision.receipt.rule_id.as_str(), "cargo-test");
    assert_eq!(
        decision.receipt.litellm_model_id.as_str(),
        "openai/gpt-5-mini"
    );
    assert_eq!(
        decision.receipt.session_lifecycle,
        ModelSessionLifecycle::Persistent {
            key: "workspace:test-runner".into()
        }
    );
    assert!(
        decision
            .receipt
            .matched_globs
            .contains(&"argv:cargo test*".to_owned())
    );
}

#[test]
fn resolver_keeps_wildcard_only_routes_matchable() {
    let resolver = CompiledModelRouteResolver::new(vec![ModelRouteRule::new(
        "fallback",
        1,
        ModelCommandMatcher::new().with_argv_glob("*"),
        ModelEndpoint::new("openai", "gpt-5-mini"),
    )])
    .expect("wildcard route compiles");

    let decision = resolver
        .resolve(&ModelRouteRequest::command(["unknown", "tool", "run"]))
        .expect("wildcard route resolves");

    assert_eq!(decision.receipt.rule_id.as_str(), "fallback");
    assert!(
        decision
            .receipt
            .matched_globs
            .contains(&"argv:*".to_owned())
    );
}

#[test]
fn resolver_keeps_alternation_globs_matchable() {
    let resolver = CompiledModelRouteResolver::new(vec![ModelRouteRule::new(
        "gpt-subcommands",
        1,
        ModelCommandMatcher::new().with_argv_glob("gpt-5.5 {exec,review}*"),
        ModelEndpoint::new("openai", "gpt-5-mini"),
    )])
    .expect("alternation route compiles");

    let decision = resolver
        .resolve(&ModelRouteRequest::command(["gpt-5.5", "exec", "task"]))
        .expect("alternation route resolves");

    assert_eq!(decision.receipt.rule_id.as_str(), "gpt-subcommands");
    assert!(
        decision
            .receipt
            .matched_globs
            .contains(&"argv:gpt-5.5 {exec,review}*".to_owned())
    );
}

#[test]
fn resolver_literal_index_uses_cwd_and_not_only_command_line() {
    let resolver = CompiledModelRouteResolver::new(vec![
        ModelRouteRule::new(
            "workspace-cwd",
            100,
            ModelCommandMatcher::new().with_cwd_glob("*/marlin-agent-core"),
            ModelEndpoint::new("anthropic", "claude-opus-4-8"),
        ),
        ModelRouteRule::new(
            "fallback",
            1,
            ModelCommandMatcher::new().with_argv_glob("*"),
            ModelEndpoint::new("openai", "gpt-5-mini"),
        ),
    ])
    .expect("route rules compile");

    let decision = resolver
        .resolve(
            &ModelRouteRequest::command(["cargo", "check"]).with_cwd("/tmp/work/marlin-agent-core"),
        )
        .expect("cwd route resolves");

    assert_eq!(decision.receipt.rule_id.as_str(), "workspace-cwd");
    assert!(
        decision
            .receipt
            .matched_globs
            .contains(&"cwd:*/marlin-agent-core".to_owned())
    );
}

#[test]
fn resolver_can_route_sub_agent_roles_to_isolated_models() {
    let resolver = CompiledModelRouteResolver::new(vec![
        ModelRouteRule::new(
            "reviewer-opus",
            100,
            ModelCommandMatcher::new().with_sub_agent_role_glob("reviewer"),
            ModelEndpoint::new("anthropic", "claude-opus-4-8"),
        )
        .with_session(ModelSessionPolicy::persistent(
            "workspace:reviewer",
            ModelContextForkMode::Isolated,
        )),
    ])
    .expect("route rule compiles");

    let decision = resolver
        .resolve(
            &ModelRouteRequest::command(["gpt-5.5", "sub-agent", "review"])
                .with_sub_agent_role("reviewer"),
        )
        .expect("sub-agent role route resolves");

    assert_eq!(decision.receipt.rule_id.as_str(), "reviewer-opus");
    assert_eq!(
        decision.receipt.litellm_model_id.as_str(),
        "anthropic/claude-opus-4-8"
    );
    assert_eq!(
        decision.receipt.context_fork,
        ModelContextForkMode::Isolated
    );
}

#[test]
fn resolver_can_route_by_agent_scope() {
    let resolver = CompiledModelRouteResolver::new(vec![ModelRouteRule::new(
        "customer-agent-cheap",
        100,
        ModelCommandMatcher::new().with_agent_scope_glob("CustomerAgent"),
        ModelEndpoint::new("openai", "gpt-5-mini"),
    )])
    .expect("route rule compiles");

    let decision = resolver
        .resolve(
            &ModelRouteRequest::command(["gpt-5.5", "sub-agent", "run"])
                .with_agent_scope(ModelRouteAgentScope::CustomerAgent),
        )
        .expect("agent scope route resolves");

    assert_eq!(decision.receipt.rule_id.as_str(), "customer-agent-cheap");
    assert_eq!(
        decision.receipt.agent_scope,
        Some(ModelRouteAgentScope::CustomerAgent)
    );
    assert!(
        decision
            .receipt
            .matched_globs
            .contains(&"agent_scope:CustomerAgent".to_owned())
    );
}

#[test]
fn resolver_rejects_invalid_globs_with_dimension_context() {
    let error = CompiledModelRouteResolver::new(vec![ModelRouteRule::new(
        "bad",
        0,
        ModelCommandMatcher::new().with_argv_glob("["),
        ModelEndpoint::new("openai", "gpt-5-mini"),
    )])
    .expect_err("invalid glob should fail");

    assert_eq!(error.dimension(), "argv");
    assert_eq!(error.pattern(), "[");
}
