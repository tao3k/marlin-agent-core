//! Deterministic sub-agent scenario fixtures spanning routing, session, and hooks.

use marlin_agent_protocol::{
    HookDispatchPolicyReceipt, HookDispatchSelectionReceipt, HookRunSummary, ModelCommandMatcher,
    ModelContextForkMode, ModelEndpoint, ModelGatewayRequest, ModelGatewayTransport,
    ModelRouteDecision, ModelRouteRequest, ModelRouteRule, ModelSessionLifecycle,
    ModelSessionPolicy, SubAgentSpawnProfile, user_gateway_message,
};
use marlin_agent_sessions::{AgentSessionContext, ContextNamespace, SessionIsolationReceipt};

use crate::{
    ScriptedGatewayRequestReceipt, SubAgentMemorySessionFixture, assert_custom_hook_policy_receipt,
    assert_custom_sub_agent_start_hook_summary, assert_sub_agent_hook_dispatch_selection,
    custom_hook_policy_receipt_fixture, custom_sub_agent_start_hook_summary_fixture,
    sub_agent_hook_dispatch_selection_fixture, sub_agent_memory_allowed_fixture,
};

const REVIEWER_MODEL_ID: &str = "anthropic/claude-opus-4-8";
const REVIEWER_ROUTE_RULE_ID: &str = "reviewer-opus";
const REVIEWER_PERSISTENCE_KEY: &str = "workspace:reviewer";
const REVIEWER_CHILD_SESSION_ID: &str = "model-route/persistent/workspace:reviewer";

/// Fixture for one no-LLM sub-agent path across route, session, hook, and gateway.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeterministicSubAgentScenarioFixture {
    session_fixture: SubAgentMemorySessionFixture,
    route_rule: ModelRouteRule,
    route_request: ModelRouteRequest,
    endpoint: ModelEndpoint,
    start_hook_summary: HookRunSummary,
    hook_selection: HookDispatchSelectionReceipt,
    hook_policy: HookDispatchPolicyReceipt,
}

/// Runtime output projection captured from a routed sub-agent execution.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeterministicRoutedSubAgentExecutionReceipt {
    pub session_id: String,
    pub parent_session_id: Option<String>,
    pub system_visible: bool,
    pub workspace_visible: bool,
    pub memory_visible: bool,
}

impl DeterministicSubAgentScenarioFixture {
    /// Parent session and configured sub-agent memory expectation.
    pub fn session_fixture(&self) -> &SubAgentMemorySessionFixture {
        &self.session_fixture
    }

    /// Model route rule expected to match the sub-agent role.
    pub fn route_rule(&self) -> &ModelRouteRule {
        &self.route_rule
    }

    /// Runtime request expected to resolve to `route_rule`.
    pub fn route_request(&self) -> &ModelRouteRequest {
        &self.route_request
    }

    /// Hook summary expected when the sub-agent starts.
    pub fn start_hook_summary(&self) -> &HookRunSummary {
        &self.start_hook_summary
    }

    /// Hook selection receipt expected before dispatch.
    pub fn hook_selection(&self) -> &HookDispatchSelectionReceipt {
        &self.hook_selection
    }

    /// Hook policy receipt expected for custom hook enforcement.
    pub fn hook_policy(&self) -> &HookDispatchPolicyReceipt {
        &self.hook_policy
    }

    /// Expected child session id for the routed model session.
    pub fn expected_route_child_session_id(&self) -> &str {
        REVIEWER_CHILD_SESSION_ID
    }

    /// Expected LiteLLM model id for the routed request.
    pub fn expected_litellm_model_id(&self) -> &str {
        REVIEWER_MODEL_ID
    }

    /// Model request for exercising a scripted gateway without live LLM access.
    pub fn model_request(&self, prompt: impl Into<String>) -> ModelGatewayRequest {
        ModelGatewayRequest::new(self.endpoint.clone(), vec![user_gateway_message(prompt)])
            .with_transport(ModelGatewayTransport::Sse)
    }
}

/// Fixture for a reviewer sub-agent that routes to a deterministic model endpoint.
pub fn deterministic_reviewer_sub_agent_scenario_fixture() -> DeterministicSubAgentScenarioFixture {
    let session_fixture = sub_agent_memory_allowed_fixture();
    let endpoint = ModelEndpoint::new("anthropic", "claude-opus-4-8");
    let route_rule = ModelRouteRule::new(
        REVIEWER_ROUTE_RULE_ID,
        100,
        ModelCommandMatcher::new()
            .with_sub_agent_role_glob("reviewer")
            .with_command_kind_glob("review"),
        endpoint.clone(),
    )
    .with_session(ModelSessionPolicy::persistent(
        REVIEWER_PERSISTENCE_KEY,
        ModelContextForkMode::ForkSnapshot,
    ));
    let route_request = ModelRouteRequest::command(["codex", "sub-agent", "review"])
        .with_sub_agent_role("reviewer")
        .with_command_kind("review");

    DeterministicSubAgentScenarioFixture {
        session_fixture,
        route_rule,
        route_request,
        endpoint,
        start_hook_summary: custom_sub_agent_start_hook_summary_fixture(),
        hook_selection: sub_agent_hook_dispatch_selection_fixture(),
        hook_policy: custom_hook_policy_receipt_fixture(),
    }
}

/// Assert the fixture's protocol-owned route, session, and hook setup.
pub fn assert_deterministic_sub_agent_scenario_fixture(
    fixture: &DeterministicSubAgentScenarioFixture,
) {
    assert_sub_agent_profile(fixture);
    assert_sub_agent_route_inputs(fixture);
    assert_custom_sub_agent_start_hook_summary(fixture.start_hook_summary());
    assert_sub_agent_hook_dispatch_selection(fixture.hook_selection());
    assert_custom_hook_policy_receipt(fixture.hook_policy());
}

/// Assert a model-route decision produced by the runtime resolver for this fixture.
pub fn assert_deterministic_sub_agent_route_decision(
    fixture: &DeterministicSubAgentScenarioFixture,
    decision: &ModelRouteDecision,
) {
    assert_eq!(
        decision.endpoint.litellm_model_id().as_str(),
        fixture.expected_litellm_model_id(),
    );
    assert_eq!(decision.receipt.rule_id.as_str(), REVIEWER_ROUTE_RULE_ID);
    assert_eq!(decision.receipt.command_line, "codex sub-agent review");
    assert_eq!(
        decision.receipt.context_fork,
        ModelContextForkMode::ForkSnapshot
    );
    assert!(decision.receipt.fallback_reason.is_none());
    assert!(matches!(
        &decision.session.lifecycle,
        ModelSessionLifecycle::Persistent { key } if key.as_str() == REVIEWER_PERSISTENCE_KEY
    ));
}

/// Assert the routed model child session produced by runtime session binding.
pub fn assert_deterministic_routed_sub_agent_session(
    fixture: &DeterministicSubAgentScenarioFixture,
    child_session: &AgentSessionContext,
    isolation_receipt: &SessionIsolationReceipt,
) {
    assert_eq!(
        child_session.session_id().as_str(),
        fixture.expected_route_child_session_id(),
    );
    assert_eq!(
        child_session
            .parent_session_id()
            .expect("routed sub-agent should keep parent"),
        fixture.session_fixture().parent_session().session_id(),
    );
    assert!(
        child_session
            .visibility()
            .contains(&ContextNamespace::Memory)
    );
    assert!(isolation_receipt.denied_namespaces().is_empty());
}

/// Assert a routed sub-agent execution observed the fixture's child session.
pub fn assert_deterministic_routed_sub_agent_execution(
    fixture: &DeterministicSubAgentScenarioFixture,
    receipt: &DeterministicRoutedSubAgentExecutionReceipt,
) {
    assert_eq!(
        receipt.session_id,
        fixture.expected_route_child_session_id()
    );
    assert_eq!(receipt.parent_session_id.as_deref(), Some("session/root"));
    assert!(receipt.system_visible);
    assert!(receipt.workspace_visible);
    assert!(receipt.memory_visible);
}

/// Assert a scripted gateway request for this routed sub-agent fixture.
pub fn assert_deterministic_sub_agent_gateway_request(
    fixture: &DeterministicSubAgentScenarioFixture,
    request: &ScriptedGatewayRequestReceipt,
) {
    assert_eq!(
        request.litellm_model_id,
        fixture.expected_litellm_model_id(),
    );
    assert_eq!(request.message_count, 1);
    assert_eq!(request.transport, ModelGatewayTransport::Sse);
}

fn assert_sub_agent_profile(fixture: &DeterministicSubAgentScenarioFixture) {
    let config = fixture.session_fixture().config();
    let profile = SubAgentSpawnProfile::from_config(config);

    assert_eq!(profile.profile_id, "reviewer");
    assert_eq!(profile.role, "memory-aware reviewer");
    assert_eq!(profile.hook_agent_scope, config.hook_agent_scope);
}

fn assert_sub_agent_route_inputs(fixture: &DeterministicSubAgentScenarioFixture) {
    assert_eq!(
        fixture.route_rule().rule_id.as_str(),
        REVIEWER_ROUTE_RULE_ID
    );
    assert_eq!(fixture.route_rule().priority, 100);
    assert_eq!(
        fixture.route_rule().endpoint.litellm_model_id().as_str(),
        fixture.expected_litellm_model_id(),
    );
    assert_eq!(
        fixture.route_request().sub_agent_role.as_deref(),
        Some("reviewer"),
    );
    assert!(fixture.route_request().command_kind.is_some());
}
