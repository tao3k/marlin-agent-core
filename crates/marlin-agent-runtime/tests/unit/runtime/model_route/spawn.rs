use std::sync::Arc;

use marlin_agent_protocol::ModelRouteRequest;
use marlin_agent_runtime::{
    CancellationToken, CompiledModelRouteResolver, ContextNamespace, RuntimeContext,
    RuntimeEnvironment, RuntimeFuture, SubAgentRuntime, TokioAgentRuntime,
};
use marlin_agent_test_support::{
    DeterministicRoutedSubAgentExecutionReceipt, DeterministicSubAgentScenarioFixture,
    assert_deterministic_routed_sub_agent_execution, assert_deterministic_sub_agent_route_decision,
    deterministic_reviewer_sub_agent_scenario_fixture,
};

#[tokio::test]
async fn routed_sub_agent_spawn_runs_inside_model_route_session() {
    let fixture = deterministic_reviewer_sub_agent_scenario_fixture();
    let resolver = resolver_from_fixture(&fixture);
    let decision = resolver
        .resolve(fixture.route_request())
        .expect("sub-agent route resolves");
    let parent_session = fixture.session_fixture().parent_session().clone();
    let (runtime, _events) = TokioAgentRuntime::with_session(
        4,
        CancellationToken::new(),
        RuntimeEnvironment::default(),
        parent_session,
    );

    let (task, binding) = runtime.spawn_sub_agent_with_model_route(
        Arc::new(ModelRouteSessionEchoSubAgent),
        (),
        &decision,
    );
    let output = task.join().await.expect("routed sub-agent should finish");

    assert_deterministic_sub_agent_route_decision(&fixture, &decision);
    assert_eq!(
        binding.child_session_id().as_str(),
        fixture.expected_route_child_session_id(),
    );
    assert_deterministic_routed_sub_agent_execution(&fixture, &output.to_execution_receipt());
}

#[tokio::test]
async fn routed_sub_agent_spawn_resolves_request_before_spawning() {
    let fixture = deterministic_reviewer_sub_agent_scenario_fixture();
    let resolver = resolver_from_fixture(&fixture);
    let parent_session = fixture.session_fixture().parent_session().clone();
    let (runtime, _events) = TokioAgentRuntime::with_session(
        4,
        CancellationToken::new(),
        RuntimeEnvironment::default(),
        parent_session,
    );

    let (task, binding, decision) = runtime
        .spawn_routed_sub_agent(
            Arc::new(ModelRouteSessionEchoSubAgent),
            (),
            &resolver,
            fixture.route_request(),
        )
        .expect("route resolves and sub-agent is spawned");
    let output = task.join().await.expect("routed sub-agent should finish");

    assert_deterministic_sub_agent_route_decision(&fixture, &decision);
    assert_eq!(
        binding.child_session_id().as_str(),
        fixture.expected_route_child_session_id(),
    );
    assert_deterministic_routed_sub_agent_execution(&fixture, &output.to_execution_receipt());
}

#[tokio::test]
async fn routed_sub_agent_spawn_returns_none_without_route_match() {
    let resolver = CompiledModelRouteResolver::new(vec![]).expect("empty resolver compiles");
    let (runtime, _events) = TokioAgentRuntime::new(4);

    let spawn = runtime.spawn_routed_sub_agent(
        Arc::new(ModelRouteSessionEchoSubAgent),
        (),
        &resolver,
        &ModelRouteRequest::command(["gpt-5.5", "sub-agent", "review"]),
    );

    assert!(spawn.is_none());
}

#[derive(Clone, Debug)]
struct ModelRouteSessionEchoSubAgent;

impl SubAgentRuntime for ModelRouteSessionEchoSubAgent {
    type Input = ();
    type Output = ModelRouteSessionEcho;

    fn run_sub_agent(
        &self,
        _input: Self::Input,
        context: RuntimeContext,
    ) -> RuntimeFuture<Self::Output> {
        let session = context.session().clone();
        Box::pin(async move {
            ModelRouteSessionEcho {
                session_id: session.session_id().as_str().to_owned(),
                parent_session_id: session
                    .parent_session_id()
                    .map(|session_id| session_id.as_str().to_owned()),
                system_visible: session.visibility().contains(&ContextNamespace::System),
                workspace_visible: session.visibility().contains(&ContextNamespace::Workspace),
                memory_visible: session.visibility().contains(&ContextNamespace::Memory),
            }
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ModelRouteSessionEcho {
    session_id: String,
    parent_session_id: Option<String>,
    system_visible: bool,
    workspace_visible: bool,
    memory_visible: bool,
}

impl ModelRouteSessionEcho {
    fn to_execution_receipt(&self) -> DeterministicRoutedSubAgentExecutionReceipt {
        DeterministicRoutedSubAgentExecutionReceipt {
            session_id: self.session_id.clone(),
            parent_session_id: self.parent_session_id.clone(),
            system_visible: self.system_visible,
            workspace_visible: self.workspace_visible,
            memory_visible: self.memory_visible,
        }
    }
}

fn resolver_from_fixture(
    fixture: &DeterministicSubAgentScenarioFixture,
) -> CompiledModelRouteResolver {
    CompiledModelRouteResolver::new(vec![fixture.route_rule().clone()])
        .expect("fixture route rule compiles")
}
