//! Deterministic sub-agent scenario fixtures spanning routing, session, and hooks.

use marlin_agent_harness_types::{AgentHarnessEvidence, AgentHarnessEvidenceKind};
use marlin_agent_protocol::{
    HookDispatchPolicyReceipt, HookDispatchSelectionReceipt, HookRunSummary, ModelCommandMatcher,
    ModelContextForkMode, ModelEndpoint, ModelGatewayRequest, ModelGatewayTransport,
    ModelRouteDecision, ModelRouteRequest, ModelRouteRule, ModelSessionLifecycle,
    ModelSessionPolicy, RuntimeEnvironmentActivation, RuntimeEnvironmentActivationReceipt,
    RuntimeEnvironmentActivationStatus, RuntimeEnvironmentDelta, RuntimeEnvrcPolicy,
    RuntimeShellIsolationPolicy, SubAgentSpawnConfig, SubAgentSpawnConfigSet, SubAgentSpawnProfile,
    SubAgentSpawnProfileId, user_gateway_message,
};
use marlin_agent_sessions::{AgentSessionContext, ContextNamespace, SessionIsolationReceipt};

use crate::{
    ScriptedGatewayRequestReceipt, SubAgentMemorySessionFixture, assert_custom_hook_policy_receipt,
    assert_custom_sub_agent_start_hook_summary, assert_sub_agent_hook_dispatch_selection,
    custom_hook_policy_receipt_fixture, custom_sub_agent_start_hook_summary_fixture,
    sub_agent_hook_dispatch_selection_fixture, sub_agent_memory_allowed_fixture_with_config,
};

const REVIEWER_MODEL_ID: &str = "anthropic/claude-opus-4-8";
const REVIEWER_ROUTE_RULE_ID: &str = "reviewer-opus";
const REVIEWER_PERSISTENCE_KEY: &str = "workspace:reviewer";
const REVIEWER_CHILD_SESSION_ID: &str = "model-route/persistent/workspace:reviewer";
const REVIEWER_ROUTE_COMMAND: [&str; 3] = ["gpt-5.5", "sub-agent", "review"];
const REVIEWER_ROUTE_COMMAND_LINE: &str = "gpt-5.5 sub-agent review";
const REVIEWER_PROFILE_TOML: &str = r#"
[[profiles]]
profile_id = "reviewer"
agent_type = "reviewer"
role = "memory-aware reviewer"

[profiles.environment_activation.activation.Direnv]
envrc = "Project"
capture_delta = true

[profiles.environment_activation.shell]
isolate_host_environment = true
allowlist = ["PATH", "HOME"]
denylist = ["AWS_SECRET_ACCESS_KEY"]

[profiles.policy.permissions]
read_only = true
workspace_write = false
network_access = false
process_spawn = false
descendant_spawn = false
tool_access = true
hook_access = false
secret_access = false

[profiles.policy.context]
session_id = "reviewer"
visibility = ["System", "User", "Workspace", "Memory"]
max_history_items = 32

[profiles.policy.performance]
max_concurrency = 1
timeout_ms = 300000
max_depth = 1
"#;

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
    let session_fixture = sub_agent_memory_allowed_fixture_with_config(
        deterministic_reviewer_sub_agent_spawn_config(),
    );
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
    let route_request = ModelRouteRequest::command(REVIEWER_ROUTE_COMMAND)
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

/// Deterministic reviewer sub-agent profile loaded from TOML.
pub fn deterministic_reviewer_sub_agent_spawn_config() -> SubAgentSpawnConfig {
    let config_set = SubAgentSpawnConfigSet::from_toml_str(REVIEWER_PROFILE_TOML)
        .expect("deterministic reviewer profile TOML compiles");
    config_set
        .profile(&SubAgentSpawnProfileId::from("reviewer"))
        .expect("deterministic reviewer profile exists")
        .clone()
}

/// Deterministic applied environment activation receipt for the reviewer profile.
pub fn deterministic_reviewer_applied_environment_activation_receipt_fixture()
-> RuntimeEnvironmentActivationReceipt {
    let profile = deterministic_reviewer_sub_agent_spawn_config();
    let policy = profile
        .environment_activation
        .as_ref()
        .expect("deterministic reviewer profile carries environment activation");

    RuntimeEnvironmentActivationReceipt::applied(
        policy,
        RuntimeEnvironmentDelta {
            added: vec!["REVIEWER_ENV".to_owned()],
            changed: vec!["PATH".to_owned()],
            removed: vec!["REMOVE_ME".to_owned()],
        },
    )
}

/// Runtime evidence replaying the routed reviewer receipt family without live LLMs.
pub fn deterministic_reviewer_routed_receipt_family_evidence() -> AgentHarnessEvidence {
    let fixture = deterministic_reviewer_sub_agent_scenario_fixture();
    let environment = deterministic_reviewer_applied_environment_activation_receipt_fixture();
    let detail = format!(
        "route_rule_id={} command_line={} session_child_id={} session_lifecycle=Persistent provider_model_id={} provider_transport={:?} environment_status={:?} environment_delta_added=[{}] environment_delta_changed=[{}] environment_delta_removed=[{}] metadata_format=org live_llm=false",
        REVIEWER_ROUTE_RULE_ID,
        REVIEWER_ROUTE_COMMAND_LINE,
        fixture.expected_route_child_session_id(),
        fixture.expected_litellm_model_id(),
        ModelGatewayTransport::Sse,
        environment.status,
        environment.delta.added.join(","),
        environment.delta.changed.join(","),
        environment.delta.removed.join(","),
    );

    AgentHarnessEvidence::present(
        AgentHarnessEvidenceKind::Runtime,
        "routed-sub-agent-receipt-family:reviewer",
    )
    .with_detail(detail)
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
    assert_eq!(decision.receipt.command_line, REVIEWER_ROUTE_COMMAND_LINE);
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

/// Assert the deterministic reviewer profile's environment activation receipt.
pub fn assert_deterministic_reviewer_environment_activation_receipt(
    receipt: &RuntimeEnvironmentActivationReceipt,
) {
    assert_eq!(receipt.status, RuntimeEnvironmentActivationStatus::Planned);
    assert_deterministic_reviewer_environment_activation_shape(receipt);
    assert!(receipt.delta.is_empty());
}

/// Assert the deterministic reviewer profile's applied environment activation receipt.
pub fn assert_deterministic_reviewer_applied_environment_activation_receipt(
    receipt: &RuntimeEnvironmentActivationReceipt,
) {
    assert_eq!(receipt.status, RuntimeEnvironmentActivationStatus::Applied);
    assert_deterministic_reviewer_environment_activation_shape(receipt);
    assert_eq!(receipt.delta.added, vec!["REVIEWER_ENV"]);
    assert_eq!(receipt.delta.changed, vec!["PATH"]);
    assert_eq!(receipt.delta.removed, vec!["REMOVE_ME"]);
}

fn assert_deterministic_reviewer_environment_activation_shape(
    receipt: &RuntimeEnvironmentActivationReceipt,
) {
    assert!(matches!(
        &receipt.activation,
        RuntimeEnvironmentActivation::Direnv {
            envrc: RuntimeEnvrcPolicy::Project,
            capture_delta: true,
        }
    ));
    assert_eq!(
        receipt.shell,
        RuntimeShellIsolationPolicy::isolated()
            .with_allowed("PATH")
            .with_allowed("HOME")
            .with_denied("AWS_SECRET_ACCESS_KEY")
    );
}

fn assert_sub_agent_profile(fixture: &DeterministicSubAgentScenarioFixture) {
    let config = fixture.session_fixture().config();
    let profile = SubAgentSpawnProfile::from_config(config);

    assert_eq!(profile.profile_id.as_str(), "reviewer");
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
