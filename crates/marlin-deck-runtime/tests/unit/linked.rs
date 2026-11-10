use marlin_agent_protocol::{
    ModelCommandMatcher, ModelEndpoint, ModelRouteRequest, ModelRouteRule,
};
use marlin_deck_runtime::{DeckLinkedRouteResolver, linked_deck_route_selector};

#[test]
fn linked_route_resolver_calls_real_deck_selector() {
    let resolver = DeckLinkedRouteResolver::new(
        linked_deck_route_selector(),
        vec![ModelRouteRule::new(
            "linked-cargo-test",
            100,
            ModelCommandMatcher::new().with_argv_glob("cargo test*"),
            ModelEndpoint::new("openai", "gpt-5.4-mini"),
        )],
    )
    .expect("linked native resolver compiles");

    let decision = resolver
        .resolve(&ModelRouteRequest::command(["cargo", "test", "-p", "demo"]))
        .expect("linked native selector should resolve")
        .expect("linked native selector should match cargo test");

    assert_eq!(decision.receipt.rule_id.as_str(), "linked-cargo-test");
    assert_eq!(
        decision.receipt.litellm_model_id.as_str(),
        "openai/gpt-5.4-mini"
    );
}
