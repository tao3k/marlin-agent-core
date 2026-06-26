use marlin_agent_protocol::{
    ModelCommandMatcher, ModelEndpoint, ModelRouteRequest, ModelRouteRule,
};
use marlin_deck_runtime_native::{DeckRuntimeNativeRouteError, DeckRuntimeNativeRouteResolver};
use marlin_gerbil_scheme::{
    GerbilDeckRuntimeNativeAbiError, GerbilDeckRuntimeNativeModelRouteRequest,
    GerbilDeckRuntimeNativeModelRouteSelection, GerbilDeckRuntimeNativeModelRouteSelector,
    GerbilDeckRuntimeNativeStatus, GerbilDeckRuntimeNativeUtf8, GerbilDeckRuntimeNativeUtf8List,
};
use std::{
    slice,
    sync::{Mutex, OnceLock},
};

#[derive(Clone, Debug, Eq, PartialEq)]
struct CapturedNativePolicy {
    name: String,
    command_prefixes: Vec<String>,
    agent_scopes: Vec<String>,
}

static CAPTURED_NATIVE_POLICIES: OnceLock<Mutex<Vec<CapturedNativePolicy>>> = OnceLock::new();

fn captured_native_policies() -> &'static Mutex<Vec<CapturedNativePolicy>> {
    CAPTURED_NATIVE_POLICIES.get_or_init(|| Mutex::new(Vec::new()))
}

#[test]
fn native_route_resolver_maps_priority_order_to_original_policy_index() {
    let resolver = DeckRuntimeNativeRouteResolver::new(
        GerbilDeckRuntimeNativeModelRouteSelector::new(fake_select_native_index_zero),
        vec![
            ModelRouteRule::new(
                "cheap-original-zero",
                1,
                ModelCommandMatcher::new().with_argv_glob("cargo test*"),
                ModelEndpoint::new("openai", "gpt-5.4-mini"),
            ),
            ModelRouteRule::new(
                "expensive-original-one",
                100,
                ModelCommandMatcher::new().with_argv_glob("cargo test*"),
                ModelEndpoint::new("anthropic", "claude-opus-4-8"),
            ),
        ],
    )
    .expect("native route resolver compiles");

    let decision = resolver
        .resolve(&ModelRouteRequest::command(["cargo", "test", "-p", "demo"]))
        .expect("native route resolves")
        .expect("native policy matches");

    assert_eq!(decision.receipt.rule_id.as_str(), "expensive-original-one");
    assert_eq!(
        decision.receipt.litellm_model_id.as_str(),
        "anthropic/claude-opus-4-8"
    );
}

#[test]
fn native_route_resolver_sends_compiled_candidate_projection_to_native_selector() {
    captured_native_policies()
        .lock()
        .expect("capture lock should be available")
        .clear();

    let resolver = DeckRuntimeNativeRouteResolver::new(
        GerbilDeckRuntimeNativeModelRouteSelector::new(fake_assert_precompiled_projection),
        vec![
            ModelRouteRule::new(
                "low-original-zero",
                1,
                ModelCommandMatcher::new()
                    .with_argv_glob("cargo test*")
                    .with_argv_glob("cargo test*"),
                ModelEndpoint::new("openai", "gpt-5.4-mini"),
            ),
            ModelRouteRule::new(
                "high-original-one",
                100,
                ModelCommandMatcher::new()
                    .with_executable_glob("cargo")
                    .with_agent_scope_glob("*")
                    .with_sub_agent_role_glob("reviewer"),
                ModelEndpoint::new("anthropic", "claude-opus-4-8"),
            ),
        ],
    )
    .expect("native route resolver compiles");

    let decision = resolver
        .resolve(
            &ModelRouteRequest::command(["cargo", "test", "-p", "demo"])
                .with_agent_scope(marlin_agent_protocol::ModelRouteAgentScope::SubAgent)
                .with_sub_agent_role("reviewer"),
        )
        .expect("native route resolves")
        .expect("native policy matches");

    assert_eq!(decision.receipt.rule_id.as_str(), "high-original-one");

    let captured = captured_native_policies()
        .lock()
        .expect("capture lock should be available")
        .clone();

    assert_eq!(captured.len(), 1);
    assert_eq!(captured[0].name, "high-original-one");
    assert_eq!(captured[0].command_prefixes, vec!["cargo".to_owned()]);
    assert_eq!(
        captured[0].agent_scopes,
        vec!["SubAgent".to_owned(), "reviewer".to_owned()]
    );
}

#[test]
fn native_route_resolver_returns_none_without_native_match() {
    let resolver = DeckRuntimeNativeRouteResolver::new(
        GerbilDeckRuntimeNativeModelRouteSelector::new(fake_select_no_match),
        vec![ModelRouteRule::new(
            "route",
            1,
            ModelCommandMatcher::new().with_argv_glob("cargo test*"),
            ModelEndpoint::new("openai", "gpt-5.4-mini"),
        )],
    )
    .expect("native route resolver compiles");

    let decision = resolver
        .resolve(&ModelRouteRequest::command(["cargo", "test"]))
        .expect("native no-match resolves");

    assert!(decision.is_none());
}

#[test]
fn native_route_resolver_rejects_out_of_range_native_index() {
    let resolver = DeckRuntimeNativeRouteResolver::new(
        GerbilDeckRuntimeNativeModelRouteSelector::new(fake_select_out_of_range),
        vec![ModelRouteRule::new(
            "route",
            1,
            ModelCommandMatcher::new().with_argv_glob("cargo test*"),
            ModelEndpoint::new("openai", "gpt-5.4-mini"),
        )],
    )
    .expect("native route resolver compiles");

    let error = resolver
        .resolve(&ModelRouteRequest::command(["cargo", "test"]))
        .expect_err("native policy index should be rejected");

    assert_eq!(
        error,
        DeckRuntimeNativeRouteError::NativeAbi(
            GerbilDeckRuntimeNativeAbiError::InvalidPolicyIndex {
                index: 9,
                policies_len: 1,
            }
        )
    );
}

#[test]
fn native_route_resolver_revalidates_selected_policy_with_runtime_resolver() {
    let resolver = DeckRuntimeNativeRouteResolver::new(
        GerbilDeckRuntimeNativeModelRouteSelector::new(fake_panic_if_called),
        vec![ModelRouteRule::new(
            "cargo-route",
            1,
            ModelCommandMatcher::new().with_executable_glob("cargo"),
            ModelEndpoint::new("openai", "gpt-5.4-mini"),
        )],
    )
    .expect("native route resolver compiles");

    let decision = resolver
        .resolve(&ModelRouteRequest::command(["just", "test"]))
        .expect("fast native plan should return no match before native selector");

    assert!(decision.is_none());
}

#[test]
fn native_route_resolver_keeps_unsupported_command_globs_as_native_candidates() {
    let resolver = DeckRuntimeNativeRouteResolver::new(
        GerbilDeckRuntimeNativeModelRouteSelector::new(fake_select_native_index_zero),
        vec![
            ModelRouteRule::new(
                "low-prefix-route",
                1,
                ModelCommandMatcher::new().with_argv_glob("cargo test*"),
                ModelEndpoint::new("openai", "gpt-5.4-mini"),
            ),
            ModelRouteRule::new(
                "high-unsupported-glob-route",
                100,
                ModelCommandMatcher::new().with_argv_glob("cargo t?st*"),
                ModelEndpoint::new("anthropic", "claude-opus-4-8"),
            ),
        ],
    )
    .expect("native route resolver compiles");

    let decision = resolver
        .resolve(&ModelRouteRequest::command(["cargo", "test", "-p", "demo"]))
        .expect("unsupported native projection should still resolve")
        .expect("unsupported native projection should keep the high priority candidate");

    assert_eq!(
        decision.receipt.rule_id.as_str(),
        "high-unsupported-glob-route"
    );
}

#[test]
fn native_route_resolver_continues_after_broad_candidate_fails_runtime_projection() {
    let resolver = DeckRuntimeNativeRouteResolver::new(
        GerbilDeckRuntimeNativeModelRouteSelector::new(fake_select_native_index_zero),
        vec![
            ModelRouteRule::new(
                "broad-native-but-runtime-miss",
                100,
                ModelCommandMatcher::new().with_argv_glob("cargo bench [abc]"),
                ModelEndpoint::new("anthropic", "claude-opus-4-8"),
            ),
            ModelRouteRule::new(
                "lower-runtime-match",
                1,
                ModelCommandMatcher::new().with_argv_glob("cargo test*"),
                ModelEndpoint::new("openai", "gpt-5.4-mini"),
            ),
        ],
    )
    .expect("native route resolver compiles");

    let decision = resolver
        .resolve(&ModelRouteRequest::command(["cargo", "test", "-p", "demo"]))
        .expect("broad native candidate should not stop candidate traversal")
        .expect("lower candidate should match after runtime projection rejects the broad one");

    assert_eq!(decision.receipt.rule_id.as_str(), "lower-runtime-match");
}

#[test]
fn native_route_resolver_matches_role_only_candidate_when_agent_scope_is_present() {
    let resolver = DeckRuntimeNativeRouteResolver::new(
        GerbilDeckRuntimeNativeModelRouteSelector::new(fake_select_native_index_zero),
        vec![
            ModelRouteRule::new(
                "fallback-sub-agent",
                1,
                ModelCommandMatcher::new().with_agent_scope_glob("*"),
                ModelEndpoint::new("openai", "gpt-5.4-mini"),
            ),
            ModelRouteRule::new(
                "reviewer-role-route",
                100,
                ModelCommandMatcher::new().with_sub_agent_role_glob("reviewer"),
                ModelEndpoint::new("anthropic", "claude-opus-4-8"),
            ),
        ],
    )
    .expect("native route resolver compiles");

    let decision = resolver
        .resolve(
            &ModelRouteRequest::command(["cargo", "test", "-p", "demo"])
                .with_agent_scope(marlin_agent_protocol::ModelRouteAgentScope::SubAgent)
                .with_sub_agent_role("reviewer"),
        )
        .expect("role-only candidate should resolve")
        .expect("role-only candidate should be kept in the native candidate plan");

    assert_eq!(decision.receipt.rule_id.as_str(), "reviewer-role-route");
}

unsafe extern "C" fn fake_select_native_index_zero(
    request: *const GerbilDeckRuntimeNativeModelRouteRequest,
    selection: *mut GerbilDeckRuntimeNativeModelRouteSelection,
) -> GerbilDeckRuntimeNativeStatus {
    if request.is_null() || selection.is_null() {
        return GerbilDeckRuntimeNativeStatus::NULL_POINTER;
    }

    unsafe {
        *selection = GerbilDeckRuntimeNativeModelRouteSelection::matched(0);
    }

    GerbilDeckRuntimeNativeStatus::OK
}

unsafe extern "C" fn fake_select_no_match(
    request: *const GerbilDeckRuntimeNativeModelRouteRequest,
    selection: *mut GerbilDeckRuntimeNativeModelRouteSelection,
) -> GerbilDeckRuntimeNativeStatus {
    if request.is_null() || selection.is_null() {
        return GerbilDeckRuntimeNativeStatus::NULL_POINTER;
    }

    unsafe {
        *selection = GerbilDeckRuntimeNativeModelRouteSelection::empty();
    }

    GerbilDeckRuntimeNativeStatus::OK
}

unsafe extern "C" fn fake_assert_precompiled_projection(
    request: *const GerbilDeckRuntimeNativeModelRouteRequest,
    selection: *mut GerbilDeckRuntimeNativeModelRouteSelection,
) -> GerbilDeckRuntimeNativeStatus {
    if request.is_null() || selection.is_null() {
        return GerbilDeckRuntimeNativeStatus::NULL_POINTER;
    }

    unsafe {
        let request = &*request;
        let policies = slice::from_raw_parts(request.policies, request.policies_len);
        let captured = policies
            .iter()
            .map(|policy| CapturedNativePolicy {
                name: native_utf8(policy.name),
                command_prefixes: native_utf8_list(policy.command_prefixes),
                agent_scopes: native_utf8_list(policy.agent_scopes),
            })
            .collect::<Vec<_>>();

        match captured_native_policies().lock() {
            Ok(mut guard) => {
                *guard = captured;
            }
            Err(_) => return GerbilDeckRuntimeNativeStatus::INVALID_SELECTION,
        }

        *selection = GerbilDeckRuntimeNativeModelRouteSelection::matched(0);
    }

    GerbilDeckRuntimeNativeStatus::OK
}

unsafe extern "C" fn fake_select_out_of_range(
    request: *const GerbilDeckRuntimeNativeModelRouteRequest,
    selection: *mut GerbilDeckRuntimeNativeModelRouteSelection,
) -> GerbilDeckRuntimeNativeStatus {
    if request.is_null() || selection.is_null() {
        return GerbilDeckRuntimeNativeStatus::NULL_POINTER;
    }

    unsafe {
        *selection = GerbilDeckRuntimeNativeModelRouteSelection::matched(9);
    }

    GerbilDeckRuntimeNativeStatus::OK
}

unsafe extern "C" fn fake_panic_if_called(
    _request: *const GerbilDeckRuntimeNativeModelRouteRequest,
    _selection: *mut GerbilDeckRuntimeNativeModelRouteSelection,
) -> GerbilDeckRuntimeNativeStatus {
    panic!("native selector should not be called when the compiled matcher plan has no candidate")
}

unsafe fn native_utf8(value: GerbilDeckRuntimeNativeUtf8) -> String {
    if value.len == 0 {
        return String::new();
    }

    let bytes = unsafe { slice::from_raw_parts(value.ptr, value.len) };
    String::from_utf8_lossy(bytes).into_owned()
}

unsafe fn native_utf8_list(value: GerbilDeckRuntimeNativeUtf8List) -> Vec<String> {
    if value.len == 0 {
        return Vec::new();
    }

    unsafe { slice::from_raw_parts(value.items, value.len) }
        .iter()
        .map(|item| unsafe { native_utf8(*item) })
        .collect()
}
