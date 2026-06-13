//! Reusable sub-agent session and memory visibility fixtures.

use marlin_agent_protocol::{
    LoopEvidence, LoopEvidenceKind, SubAgentContextVisibility, SubAgentSpawnConfig,
};
use marlin_agent_sessions::{
    AgentSessionContext, ContextNamespace, ContextVisibility, SessionIsolationReceipt, SessionKind,
};

/// Expected memory visibility for a configured sub-agent session fixture.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SubAgentMemoryExpectation {
    Granted,
    Denied,
}

/// Fixture for testing configured sub-agent session isolation around Memory.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubAgentMemorySessionFixture {
    parent_session: AgentSessionContext,
    config: SubAgentSpawnConfig,
    expectation: SubAgentMemoryExpectation,
    expected_history_limit: Option<usize>,
    history_limit_applied: bool,
}

impl SubAgentMemorySessionFixture {
    /// Parent runtime session used to spawn the configured sub-agent.
    pub fn parent_session(&self) -> &AgentSessionContext {
        &self.parent_session
    }

    /// Configured sub-agent profile under test.
    pub fn config(&self) -> &SubAgentSpawnConfig {
        &self.config
    }

    /// Expected Memory visibility outcome.
    pub fn expectation(&self) -> SubAgentMemoryExpectation {
        self.expectation
    }

    /// Expected child-session history limit after parent narrowing.
    pub fn expected_history_limit(&self) -> Option<usize> {
        self.expected_history_limit
    }

    /// Expected isolation receipt history-limit flag.
    pub fn history_limit_applied(&self) -> bool {
        self.history_limit_applied
    }

    /// Requested child-session visibility derived from the sub-agent config.
    pub fn requested_visibility(&self) -> ContextVisibility {
        context_visibility_from_sub_agent_config(&self.config)
    }
}

/// Fixture where parent context grants Memory to the configured sub-agent.
pub fn sub_agent_memory_allowed_fixture() -> SubAgentMemorySessionFixture {
    SubAgentMemorySessionFixture {
        parent_session: AgentSessionContext::root(
            "session/root",
            ContextVisibility::from_namespaces([
                ContextNamespace::System,
                ContextNamespace::User,
                ContextNamespace::Workspace,
                ContextNamespace::Memory,
            ])
            .with_max_history_items(Some(16)),
        ),
        config: SubAgentSpawnConfig::toml("reviewer", "reviewer", "memory-aware reviewer"),
        expectation: SubAgentMemoryExpectation::Granted,
        expected_history_limit: Some(16),
        history_limit_applied: true,
    }
}

/// Fixture where parent context denies Memory to the configured sub-agent.
pub fn sub_agent_memory_denied_fixture() -> SubAgentMemorySessionFixture {
    SubAgentMemorySessionFixture {
        parent_session: AgentSessionContext::root(
            "session/root",
            ContextVisibility::from_namespaces([
                ContextNamespace::System,
                ContextNamespace::User,
                ContextNamespace::Workspace,
            ])
            .with_max_history_items(Some(32)),
        ),
        config: SubAgentSpawnConfig::toml("auditor", "auditor", "memory-limited auditor"),
        expectation: SubAgentMemoryExpectation::Denied,
        expected_history_limit: Some(32),
        history_limit_applied: false,
    }
}

/// Assert that a configured sub-agent spawn obeyed the fixture's Memory policy.
pub fn assert_sub_agent_memory_session_fixture(
    fixture: &SubAgentMemorySessionFixture,
    child_session: &AgentSessionContext,
    config: &SubAgentSpawnConfig,
    isolation_receipt: &SessionIsolationReceipt,
) {
    assert_eq!(config, fixture.config());
    assert!(
        config_requests_memory(config),
        "fixture config must request Memory visibility",
    );
    assert_eq!(child_session.kind(), &SessionKind::SubAgent);
    assert_eq!(
        child_session
            .parent_session_id()
            .expect("sub-agent child session should carry parent"),
        fixture.parent_session().session_id(),
    );
    assert_eq!(
        child_session.root_session_id(),
        fixture.parent_session().root_session_id(),
    );
    assert_eq!(
        isolation_receipt.parent_session_id(),
        fixture.parent_session().session_id(),
    );
    assert_eq!(
        isolation_receipt.child_session_id().as_str(),
        config.child_session_id(),
    );
    assert_eq!(
        child_session.visibility().max_history_items(),
        fixture.expected_history_limit(),
    );
    assert_eq!(
        isolation_receipt.history_limit_applied(),
        fixture.history_limit_applied(),
    );

    match fixture.expectation() {
        SubAgentMemoryExpectation::Granted => {
            assert!(
                child_session
                    .visibility()
                    .contains(&ContextNamespace::Memory)
            );
            assert!(
                isolation_receipt
                    .granted_visibility()
                    .contains(&ContextNamespace::Memory)
            );
            assert!(
                !isolation_receipt
                    .denied_namespaces()
                    .contains(&ContextNamespace::Memory)
            );
        }
        SubAgentMemoryExpectation::Denied => {
            assert!(
                !child_session
                    .visibility()
                    .contains(&ContextNamespace::Memory)
            );
            assert_eq!(
                isolation_receipt.denied_namespaces(),
                &[ContextNamespace::Memory],
            );
        }
    }
}

/// Project a sub-agent memory isolation receipt into runtime visibility evidence.
pub fn sub_agent_memory_session_visibility_evidence(
    child_session: &AgentSessionContext,
    isolation_receipt: &SessionIsolationReceipt,
) -> LoopEvidence {
    let parent_session_id = child_session
        .parent_session_id()
        .map(|session_id| session_id.as_str())
        .unwrap_or("none");
    let memory_visible = child_session
        .visibility()
        .contains(&ContextNamespace::Memory);
    let denied_memory = isolation_receipt
        .denied_namespaces()
        .contains(&ContextNamespace::Memory);
    let detail = format!(
        "session_id={} parent_session_id={} root_session_id={} memory_visible={} denied_memory={} denied_namespace_count={} max_history_items={:?} history_limit_applied={}",
        child_session.session_id().as_str(),
        parent_session_id,
        child_session.root_session_id().as_str(),
        memory_visible,
        denied_memory,
        isolation_receipt.denied_namespaces().len(),
        child_session.visibility().max_history_items(),
        isolation_receipt.history_limit_applied(),
    );

    LoopEvidence::present(
        LoopEvidenceKind::Visibility,
        format!(
            "sub-agent-memory-session:{}",
            child_session.session_id().as_str()
        ),
    )
    .with_detail(detail)
}

/// Project a child-session isolation receipt into replay evidence for contraction checks.
pub fn sub_agent_memory_session_replay_evidence(
    child_session: &AgentSessionContext,
    isolation_receipt: &SessionIsolationReceipt,
) -> LoopEvidence {
    let parent_session_id = child_session
        .parent_session_id()
        .map(|session_id| session_id.as_str())
        .unwrap_or("none");
    let requested_namespaces = namespace_list(isolation_receipt.requested_visibility());
    let granted_namespaces = namespace_list(isolation_receipt.granted_visibility());
    let denied_namespaces = isolation_receipt
        .denied_namespaces()
        .iter()
        .map(namespace_name)
        .collect::<Vec<_>>()
        .join(",");
    let visibility_contracted = !isolation_receipt.denied_namespaces().is_empty()
        || isolation_receipt.history_limit_applied();
    let detail = format!(
        "session_id={} parent_session_id={} root_session_id={} requested_namespaces=[{}] granted_namespaces=[{}] denied_namespaces=[{}] requested_history_limit={:?} granted_history_limit={:?} history_limit_applied={} visibility_contracted={} live_llm=false",
        child_session.session_id().as_str(),
        parent_session_id,
        child_session.root_session_id().as_str(),
        requested_namespaces,
        granted_namespaces,
        denied_namespaces,
        isolation_receipt.requested_visibility().max_history_items(),
        isolation_receipt.granted_visibility().max_history_items(),
        isolation_receipt.history_limit_applied(),
        visibility_contracted,
    );

    LoopEvidence::present(
        LoopEvidenceKind::Visibility,
        format!(
            "sub-agent-session-replay:{}",
            child_session.session_id().as_str()
        ),
    )
    .with_detail(detail)
}

fn config_requests_memory(config: &SubAgentSpawnConfig) -> bool {
    config
        .policy
        .context
        .visible_context()
        .iter()
        .any(|visibility| matches!(visibility, SubAgentContextVisibility::Memory))
}

fn context_visibility_from_sub_agent_config(config: &SubAgentSpawnConfig) -> ContextVisibility {
    ContextVisibility::from_namespaces(
        config
            .policy
            .context
            .visible_context()
            .iter()
            .map(context_namespace_from_protocol),
    )
    .with_max_history_items(config.policy.context.max_history_items)
}

fn context_namespace_from_protocol(visibility: &SubAgentContextVisibility) -> ContextNamespace {
    match visibility {
        SubAgentContextVisibility::System => ContextNamespace::System,
        SubAgentContextVisibility::User => ContextNamespace::User,
        SubAgentContextVisibility::Workspace => ContextNamespace::Workspace,
        SubAgentContextVisibility::Memory => ContextNamespace::Memory,
    }
}

fn namespace_list(visibility: &ContextVisibility) -> String {
    visibility
        .namespaces()
        .map(namespace_name)
        .collect::<Vec<_>>()
        .join(",")
}

fn namespace_name(namespace: &ContextNamespace) -> &'static str {
    match namespace {
        ContextNamespace::System => "System",
        ContextNamespace::User => "User",
        ContextNamespace::Workspace => "Workspace",
        ContextNamespace::Memory => "Memory",
        ContextNamespace::Tools => "Tools",
        ContextNamespace::Hooks => "Hooks",
        ContextNamespace::SubAgents => "SubAgents",
        ContextNamespace::Secrets => "Secrets",
    }
}
