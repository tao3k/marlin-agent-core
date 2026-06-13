use marlin_agent_core::{
    AgentExecutionTrace, AgentExecutionTraceSummary, AgentSpanName, AgentTraceSpanRecord,
    GerbilCommandSpec, GerbilHookPolicyCommandEvaluator, GerbilHookPolicyFinalizer,
    GerbilHookPolicyRuntimeBinding, GraphLoopExecutionStatus, HookAgentScope, HookDispatchPolicy,
    HookDispatcher, HookEventName, HookHandlerType, HookInvocation, HookPolicyDecisionReason,
    HookPolicyExtension, HookRegistration, HookRegistry, HookRunSummary, HookRuntime, LoopEvidence,
    LoopEvidenceKind, LoopPerformanceEvidence, ModelContextForkMode, ModelEndpoint,
    ModelGatewayMessageRole, ModelGatewayRequest, ModelGatewayTransport, ModelRouteConfig,
    ModelRouteRequest, PERFORMANCE_EVIDENCE_KEYS, RuntimeContext, RuntimeEnvironmentRequest,
    RuntimeEnvironmentResolver, RuntimeExecutionIdentity, RuntimeFuture, system_gateway_message,
};
use std::sync::Arc;
use tempfile::Builder;

#[test]
fn core_facade_exposes_environment_resolver() {
    let environment = RuntimeEnvironmentResolver::new().resolve(
        RuntimeEnvironmentRequest::default()
            .with_custom_home("/tmp/marlin-home")
            .with_cwd("/tmp/workspace"),
    );

    assert_eq!(
        environment
            .home
            .as_ref()
            .expect("home should be resolved")
            .path,
        std::path::PathBuf::from("/tmp/marlin-home")
    );
}

#[test]
fn core_facade_exposes_hook_dispatcher() {
    let dispatcher = HookDispatcher::new(HookRegistry::new());

    assert_eq!(dispatcher.registry().registrations().len(), 0);
}

#[derive(Clone, Debug)]
struct CoreSummaryHook;

impl HookRuntime for CoreSummaryHook {
    type Request = HookInvocation;
    type Output = HookRunSummary;

    fn run_hook(
        &self,
        request: Self::Request,
        _context: RuntimeContext,
    ) -> RuntimeFuture<Self::Output> {
        Box::pin(async move {
            HookRunSummary::running(
                "core-gerbil-run",
                request.event_name,
                HookHandlerType::Command,
            )
            .completed()
        })
    }
}

#[tokio::test]
async fn core_facade_wires_gerbil_hook_policy_finalizer() {
    let registry = HookRegistry::new().with_registration(HookRegistration::new(
        "core-gerbil",
        HookEventName::PreToolUse,
        HookHandlerType::Command,
        Arc::new(CoreSummaryHook),
    ));
    let evaluator = GerbilHookPolicyCommandEvaluator::new(
        GerbilCommandSpec::new("/bin/sh").arg("-c").arg(
            "cat >/dev/null; printf '%s\n' '{\"decision\":\"Rejected\",\"diagnostics\":[{\"message\":\"core finalizer rejected\"}]}'",
        ),
    );
    let finalizer = GerbilHookPolicyFinalizer::new(evaluator);
    let (runtime, _events) = marlin_agent_core::TokioAgentRuntime::new(4);

    let report = HookDispatcher::new(registry)
        .with_policy(HookDispatchPolicy::observe_only().with_extension(
            HookPolicyExtension::gerbil_scheme("marlin/hooks/policy", "decide-hook-policy"),
        ))
        .with_policy_finalizer(Arc::new(finalizer))
        .dispatch(
            &runtime,
            HookInvocation::new(HookEventName::PreToolUse)
                .with_agent_scope(HookAgentScope::CustomerAgent),
        )
        .await;

    assert_eq!(report.policy.allowed_count, 0);
    assert_eq!(report.policy.rejected_count, 1);
    assert_eq!(
        report.policy.decisions[0].reason,
        HookPolicyDecisionReason::ExtensionRejected
    );
    assert!(report.runs.is_empty());
    assert!(!report.is_success());
}

#[test]
fn core_facade_builds_gerbil_hook_policy_finalizer_from_runtime_binding() {
    let root = Builder::new()
        .prefix("marlin-core-gerbil-hook-policy-binding-")
        .tempdir()
        .expect("creates core hook policy binding root");
    let binding = GerbilHookPolicyRuntimeBinding::new("/bin/sh", root.path())
        .expect("runtime binding should write hook policy assets");
    let finalizer = GerbilHookPolicyFinalizer::from_runtime_binding(binding);

    assert_eq!(
        finalizer.evaluator().spec().program,
        std::path::Path::new("/bin/sh")
    );
    assert!(
        finalizer
            .evaluator()
            .spec()
            .args
            .iter()
            .any(|arg| arg.to_string_lossy().contains("hook-policy-adapter.ss"))
    );
}

#[test]
fn core_facade_exposes_runtime_execution_identity() {
    let identity = RuntimeExecutionIdentity::new("run-core", "graph-core");

    assert_eq!(identity.run_id(), "run-core");
    assert_eq!(identity.graph_id(), "graph-core");
}

#[test]
fn core_facade_exposes_model_route_config_loader() {
    let config = ModelRouteConfig::from_toml_str(
        r#"
[[rules]]
rule_id = "core-facade-test"
priority = 20

[rules.matcher]
executable_globs = ["cargo"]
command_kind_globs = ["test"]

[rules.endpoint]
provider = "openai"
model = "gpt-5-mini"

[rules.session]
context = "Minimal"

[rules.session.lifecycle.Persistent]
key = "core-facade"
"#,
    )
    .expect("config parses through core facade");

    let resolver = config
        .compile_resolver()
        .expect("resolver compiles through core facade");
    let decision = resolver
        .resolve(&ModelRouteRequest::command(["cargo", "test"]).with_command_kind("test"))
        .expect("route resolves through core facade");

    assert_eq!(decision.endpoint.provider.as_str(), "openai");
    assert_eq!(decision.endpoint.model.as_str(), "gpt-5-mini");
    assert_eq!(decision.receipt.context_fork, ModelContextForkMode::Minimal);
}

#[test]
fn core_facade_exposes_model_gateway_protocol_boundary() {
    let request = ModelGatewayRequest::new(
        ModelEndpoint::new("openai", "gpt-5-mini"),
        vec![system_gateway_message("system")],
    )
    .with_transport(ModelGatewayTransport::WebSocket);

    assert_eq!(request.transport(), &ModelGatewayTransport::WebSocket);
    assert_eq!(request.messages()[0].role, ModelGatewayMessageRole::System);
}

#[test]
fn core_facade_exposes_execution_trace_summary() {
    let result_span = AgentTraceSpanRecord::new("harness.result")
        .with_field("run_id", "run")
        .with_field("status", "Completed");
    let trace = AgentExecutionTrace::new("run", "graph", GraphLoopExecutionStatus::Completed)
        .with_spans(vec![result_span]);
    let summary: AgentExecutionTraceSummary = trace.summary();

    assert_eq!(summary.run_id.as_str(), "run");
    assert_eq!(summary.graph_id.as_str(), "graph");
    assert_eq!(summary.status, GraphLoopExecutionStatus::Completed);
    assert!(
        trace
            .find_span_with_field(&AgentSpanName::new("harness.result"), "run_id", "run")
            .is_some()
    );
}

#[test]
fn core_facade_exposes_performance_evidence_contract() {
    let evidence: LoopEvidence = LoopPerformanceEvidence {
        subject: "core-runtime".to_owned(),
        benchmark_command: "cargo bench -p marlin-agent-core".to_owned(),
        baseline: "p95=10ms".to_owned(),
        regression_threshold: "5%".to_owned(),
        latency_or_throughput: "throughput=1000/s".to_owned(),
        allocation_profile: "allocations=steady".to_owned(),
        profile_artifact: "target/criterion/core/index.html".to_owned(),
    }
    .into();
    let detail = evidence.detail.as_deref().expect("performance detail");

    assert_eq!(evidence.kind, LoopEvidenceKind::Performance);
    assert_eq!(evidence.subject, "core-runtime");
    for key in PERFORMANCE_EVIDENCE_KEYS {
        assert!(
            detail.contains(key),
            "missing performance evidence key {key}"
        );
    }
}
