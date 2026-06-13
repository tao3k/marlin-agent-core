use std::sync::Arc;

use marlin_agent_runtime::{
    AgentSessionContext, CancellationToken, ContextNamespace, ContextVisibility, HookRuntime,
    RuntimeContext, RuntimeEnvironment, RuntimeFuture, SessionKind, SubAgentRuntime,
    TokioAgentRuntime,
};
use marlin_agent_test_support::{
    assert_sub_agent_memory_session_fixture, sub_agent_memory_allowed_fixture,
    sub_agent_memory_denied_fixture,
};

#[tokio::test]
async fn sub_agent_session_context_isolated_from_parent() {
    let parent_session = AgentSessionContext::root(
        "session/root",
        ContextVisibility::from_namespaces([
            ContextNamespace::Workspace,
            ContextNamespace::Memory,
            ContextNamespace::SubAgents,
        ])
        .with_max_history_items(Some(8)),
    );
    let (runtime, _events) = TokioAgentRuntime::with_session(
        4,
        CancellationToken::new(),
        RuntimeEnvironment::default(),
        parent_session.clone(),
    );

    let (task, receipt) = runtime.spawn_sub_agent_with_session(
        Arc::new(SessionEchoSubAgent),
        (),
        "session/sub-agent/reviewer",
        ContextVisibility::from_namespaces([
            ContextNamespace::Workspace,
            ContextNamespace::Secrets,
        ])
        .with_max_history_items(Some(64)),
    );
    let child_session = task
        .join()
        .await
        .expect("sub-agent session task should finish");

    assert_eq!(runtime.session(), &parent_session);
    assert_eq!(child_session.kind(), &SessionKind::SubAgent);
    assert_eq!(child_session.root_session_id().as_str(), "session/root");
    assert_eq!(
        child_session
            .parent_session_id()
            .expect("sub-agent session should carry parent")
            .as_str(),
        "session/root"
    );
    assert!(
        child_session
            .visibility()
            .contains(&ContextNamespace::Workspace)
    );
    assert!(
        !child_session
            .visibility()
            .contains(&ContextNamespace::Secrets)
    );
    assert_eq!(child_session.visibility().max_history_items(), Some(8));
    assert_eq!(receipt.denied_namespaces(), &[ContextNamespace::Secrets]);
    assert!(receipt.history_limit_applied());
}

#[tokio::test]
async fn configured_sub_agent_inherits_memory_when_parent_session_allows_it() {
    let fixture = sub_agent_memory_allowed_fixture();
    let (runtime, _events) = TokioAgentRuntime::with_session(
        4,
        CancellationToken::new(),
        RuntimeEnvironment::default(),
        fixture.parent_session().clone(),
    );

    let (task, receipt) = runtime.spawn_sub_agent_with_config(
        Arc::new(SessionEchoSubAgent),
        (),
        fixture.config().clone(),
    );
    let child_session = task
        .join()
        .await
        .expect("configured sub-agent session task should finish");

    assert_sub_agent_memory_session_fixture(
        &fixture,
        &child_session,
        receipt.config(),
        receipt.isolation_receipt(),
    );
}

#[tokio::test]
async fn configured_sub_agent_memory_visibility_is_denied_without_parent_grant() {
    let fixture = sub_agent_memory_denied_fixture();
    let (runtime, _events) = TokioAgentRuntime::with_session(
        4,
        CancellationToken::new(),
        RuntimeEnvironment::default(),
        fixture.parent_session().clone(),
    );

    let (task, receipt) = runtime.spawn_sub_agent_with_config(
        Arc::new(SessionEchoSubAgent),
        (),
        fixture.config().clone(),
    );
    let child_session = task
        .join()
        .await
        .expect("configured sub-agent session task should finish");

    assert_sub_agent_memory_session_fixture(
        &fixture,
        &child_session,
        receipt.config(),
        receipt.isolation_receipt(),
    );
}

#[tokio::test]
async fn hook_session_context_isolated_from_parent() {
    let parent_session = AgentSessionContext::root(
        "session/root",
        ContextVisibility::from_namespaces([ContextNamespace::Hooks]),
    );
    let (runtime, _events) = TokioAgentRuntime::with_session(
        4,
        CancellationToken::new(),
        RuntimeEnvironment::default(),
        parent_session,
    );

    let (task, receipt) = runtime.spawn_hook_with_session(
        Arc::new(SessionEchoHook),
        "pre-tool".to_owned(),
        "session/hook/pre-tool",
        ContextVisibility::from_namespaces([ContextNamespace::Hooks, ContextNamespace::Secrets]),
    );
    let (request, hook_session) = task.join().await.expect("hook session task should finish");

    assert_eq!(request, "pre-tool");
    assert_eq!(hook_session.kind(), &SessionKind::Hook);
    assert!(hook_session.visibility().contains(&ContextNamespace::Hooks));
    assert!(
        !hook_session
            .visibility()
            .contains(&ContextNamespace::Secrets)
    );
    assert_eq!(receipt.denied_namespaces(), &[ContextNamespace::Secrets]);
}

#[derive(Clone, Debug)]
struct SessionEchoSubAgent;

impl SubAgentRuntime for SessionEchoSubAgent {
    type Input = ();
    type Output = AgentSessionContext;

    fn run_sub_agent(
        &self,
        _input: Self::Input,
        context: RuntimeContext,
    ) -> RuntimeFuture<Self::Output> {
        let session = context.session().clone();
        Box::pin(async move { session })
    }
}

#[derive(Clone, Debug)]
struct SessionEchoHook;

impl HookRuntime for SessionEchoHook {
    type Request = String;
    type Output = (String, AgentSessionContext);

    fn run_hook(
        &self,
        request: Self::Request,
        context: RuntimeContext,
    ) -> RuntimeFuture<Self::Output> {
        let session = context.session().clone();
        Box::pin(async move { (request, session) })
    }
}
