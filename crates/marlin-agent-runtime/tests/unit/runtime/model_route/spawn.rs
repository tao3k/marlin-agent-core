use std::sync::Arc;

use marlin_agent_protocol::{
    ModelCommandMatcher, ModelContextForkMode, ModelEndpoint, ModelRouteRequest, ModelRouteRule,
    ModelSessionPolicy,
};
use marlin_agent_runtime::{
    AgentSessionContext, CancellationToken, CompiledModelRouteResolver, ContextNamespace,
    ContextVisibility, ModelRouteConfig, RuntimeContext, RuntimeEnvironment, RuntimeFuture,
    SubAgentRuntime, TokioAgentRuntime,
};

#[tokio::test]
async fn routed_sub_agent_spawn_runs_inside_model_route_session() {
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
            &ModelRouteRequest::command(["codex", "sub-agent", "review"])
                .with_sub_agent_role("reviewer"),
        )
        .expect("sub-agent route resolves");
    let parent_session = AgentSessionContext::root(
        "session/root",
        ContextVisibility::from_namespaces([
            ContextNamespace::System,
            ContextNamespace::Workspace,
            ContextNamespace::Memory,
        ]),
    );
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

    assert_eq!(
        binding.child_session_id().as_str(),
        "model-route/persistent/workspace:reviewer"
    );
    assert_eq!(output.session_id, binding.child_session_id().as_str());
    assert_eq!(output.parent_session_id.as_deref(), Some("session/root"));
    assert!(output.system_visible);
    assert!(!output.workspace_visible);
    assert!(!output.memory_visible);
}

#[tokio::test]
async fn routed_sub_agent_spawn_resolves_request_before_spawning() {
    let resolver = ModelRouteConfig::from_toml_str(
        r#"
[[rules]]
rule_id = "reviewer-opus"
priority = 100

[rules.matcher]
sub_agent_role_globs = ["reviewer"]
command_kind_globs = ["review"]

[rules.endpoint]
provider = "anthropic"
model = "claude-opus-4-8"

[rules.session]
context = "Isolated"

[rules.session.lifecycle.Persistent]
key = "workspace:reviewer"
"#,
    )
    .expect("route config parses")
    .into_resolver()
    .expect("route resolver compiles");
    let parent_session = AgentSessionContext::root(
        "session/root",
        ContextVisibility::from_namespaces([
            ContextNamespace::System,
            ContextNamespace::Workspace,
            ContextNamespace::Memory,
        ]),
    );
    let (runtime, _events) = TokioAgentRuntime::with_session(
        4,
        CancellationToken::new(),
        RuntimeEnvironment::default(),
        parent_session,
    );
    let request = ModelRouteRequest::command(["codex", "sub-agent", "review"])
        .with_sub_agent_role("reviewer")
        .with_command_kind("review");

    let (task, binding, decision) = runtime
        .spawn_routed_sub_agent(
            Arc::new(ModelRouteSessionEchoSubAgent),
            (),
            &resolver,
            &request,
        )
        .expect("route resolves and sub-agent is spawned");
    let output = task.join().await.expect("routed sub-agent should finish");

    assert_eq!(decision.endpoint.provider.as_str(), "anthropic");
    assert_eq!(decision.endpoint.model.as_str(), "claude-opus-4-8");
    assert_eq!(
        binding.child_session_id().as_str(),
        "model-route/persistent/workspace:reviewer"
    );
    assert_eq!(output.session_id, binding.child_session_id().as_str());
    assert!(output.system_visible);
    assert!(!output.workspace_visible);
    assert!(!output.memory_visible);
}

#[tokio::test]
async fn routed_sub_agent_spawn_returns_none_without_route_match() {
    let resolver = CompiledModelRouteResolver::new(vec![]).expect("empty resolver compiles");
    let (runtime, _events) = TokioAgentRuntime::new(4);

    let spawn = runtime.spawn_routed_sub_agent(
        Arc::new(ModelRouteSessionEchoSubAgent),
        (),
        &resolver,
        &ModelRouteRequest::command(["codex", "sub-agent", "review"]),
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
