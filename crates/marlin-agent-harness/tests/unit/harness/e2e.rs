use std::sync::Arc;

use marlin_agent_harness::{
    AgentHarness, HarnessExecutionReport, HarnessGraphBuilder, HarnessRuntime,
};
use marlin_agent_hooks::{HookDispatcher, HookInvocation, HookRegistration, HookRegistry};
use marlin_agent_kernel::{
    GraphLoopExecutionRequest, GraphLoopExecutionStatus, GraphNodeInvocation, ProviderNodeAdapter,
    SubAgentNodeAdapter, TokioGraphLoopKernel, ToolNodeAdapter,
};
use marlin_agent_protocol::{
    AgentScenario, AgentScenarioStep, HookEventName, HookHandlerType, HookRunSummary, LoopEvidence,
    LoopEvidenceKind, RuntimeHome,
};
use marlin_agent_runtime::{
    HookRuntime, ProviderRuntime, RuntimeContext, RuntimeEnvironment, RuntimeEvent, RuntimeFuture,
    SubAgentRuntime, ToolRuntime, observability,
};

#[tokio::test]
async fn harness_runs_provider_tool_sub_agent_scenario_with_hooks_and_environment() {
    let environment = RuntimeEnvironment::default()
        .with_home(RuntimeHome::custom("/tmp/marlin-e2e-home").with_profile("e2e"))
        .with_cwd("/tmp/marlin-e2e-workspace");
    let scenario = AgentScenario::new("provider-tool-sub-agent")
        .with_step(
            AgentScenarioStep::new("run")
                .expecting_event_topic(observability::TOPIC_KERNEL_EXECUTION)
                .expecting_event_topic(observability::TOPIC_KERNEL_HOOK)
                .expecting_event_topic(observability::TOPIC_KERNEL_SUB_AGENT)
                .expecting_event_topic("test.e2e")
                .expecting_span_name(observability::runtime_task_span_name())
                .expecting_span_name(observability::agent_provider_span_name())
                .expecting_span_name(observability::agent_tool_span_name())
                .expecting_span_name(observability::agent_sub_agent_span_name())
                .expecting_span_name(observability::hook_dispatch_span_name())
                .expecting_span_name(observability::hook_run_span_name())
                .expecting_span_name(observability::harness_execution_span_name())
                .expecting_span_name(observability::harness_result_span_name()),
        )
        .expecting_evidence(LoopEvidenceKind::Runtime);
    let hook_dispatcher = e2e_hook_dispatcher();
    let kernel = TokioGraphLoopKernel::new("run", "graph")
        .with_executor(
            "provider",
            ProviderNodeAdapter::new(E2eProvider, |invocation: GraphNodeInvocation| {
                invocation.node_id.into_string()
            })
            .with_hook_dispatcher(hook_dispatcher.clone()),
        )
        .with_executor(
            "tool",
            ToolNodeAdapter::new(E2eTool, |invocation: GraphNodeInvocation| {
                invocation.node_id.into_string()
            })
            .with_hook_dispatcher(hook_dispatcher.clone()),
        )
        .with_executor(
            "sub-agent",
            SubAgentNodeAdapter::new(E2eSubAgent, |invocation: GraphNodeInvocation| {
                invocation.node_id.into_string()
            })
            .with_hook_dispatcher(hook_dispatcher),
        );
    let graph = HarnessGraphBuilder::new("graph")
        .linear([
            ("plan", "provider"),
            ("apply", "tool"),
            ("review", "sub-agent"),
        ])
        .build();
    let request = GraphLoopExecutionRequest::new("run-e2e", graph);
    let mut harness = HarnessRuntime::with_environment(64, environment);
    harness.record_evidence(LoopEvidence::present(LoopEvidenceKind::Runtime, "tokio"));

    let report = harness.execute_graph(&scenario, &kernel, request).await;
    let evaluated = AgentHarness::evaluate_execution_report(&scenario, &report);
    let e2e_messages = report
        .events
        .iter()
        .filter(|event| event.topic == "test.e2e")
        .map(|event| event.message.as_str())
        .collect::<Vec<_>>();

    assert_eq!(report.result.status, GraphLoopExecutionStatus::Completed);
    assert_eq!(report.summary.status, GraphLoopExecutionStatus::Completed);
    assert_eq!(report.summary.event_count, report.events.len());
    assert_eq!(report.summary.span_count, report.trace_spans.len());
    assert_eq!(
        report.summary.diagnostic_count,
        report.result.diagnostics.len()
    );
    assert_eq!(report.result.visited_nodes, vec!["plan", "apply", "review"]);
    assert!(report.assertion.is_none());
    assert!(evaluated.is_success());
    for expected_span in [
        observability::runtime_task_span_name(),
        observability::agent_provider_span_name(),
        observability::agent_tool_span_name(),
        observability::agent_sub_agent_span_name(),
        observability::hook_dispatch_span_name(),
        observability::hook_run_span_name(),
        observability::harness_execution_span_name(),
        observability::harness_result_span_name(),
    ] {
        assert!(
            report.has_span(&expected_span),
            "missing expected harness span {}",
            expected_span.as_str()
        );
    }
    assert!(e2e_messages.contains(&"hook pre-tool"));
    assert!(e2e_messages.contains(&"provider plan home /tmp/marlin-e2e-home profile e2e"));
    assert!(e2e_messages.contains(&"tool apply home /tmp/marlin-e2e-home profile e2e"));
    assert!(e2e_messages.contains(&"hook sub-agent-start"));
    assert!(e2e_messages.contains(&"sub-agent review home /tmp/marlin-e2e-home profile e2e"));
    assert!(e2e_messages.contains(&"hook sub-agent-stop"));
    assert!(
        report
            .events
            .iter()
            .any(|event| event.topic == observability::TOPIC_KERNEL_SUB_AGENT
                && event.message.contains("Started"))
    );
    assert!(
        report
            .events
            .iter()
            .any(|event| event.topic == observability::TOPIC_KERNEL_SUB_AGENT
                && event.message.contains("Stopped"))
    );

    assert_agent_core_trace_spans(&report);
}

fn assert_agent_core_trace_spans(report: &HarnessExecutionReport) {
    let span_names = report
        .span_names
        .iter()
        .map(|span_name| span_name.as_str())
        .collect::<Vec<_>>();
    assert!(
        report.has_span(&observability::runtime_task_span_name()),
        "captured spans: {span_names:?}"
    );
    assert_eq!(
        report.count_span(&observability::hook_dispatch_span_name()),
        6
    );
    assert_eq!(report.count_span(&observability::hook_run_span_name()), 6);

    let provider_span = report
        .find_span(&observability::agent_provider_span_name())
        .expect("expected provider trace span");
    assert_eq!(
        provider_span.name,
        observability::agent_provider_span_name()
    );
    assert_eq!(
        provider_span
            .fields
            .get(observability::FIELD_NODE_ID)
            .map(String::as_str),
        Some("plan")
    );
    assert_eq!(
        provider_span
            .fields
            .get(observability::FIELD_EXECUTOR)
            .map(String::as_str),
        Some("provider")
    );

    let result_span = report
        .find_span(&observability::harness_result_span_name())
        .expect("expected harness result trace span");
    assert_eq!(
        result_span
            .fields
            .get(observability::FIELD_STATUS)
            .map(String::as_str),
        Some("Completed")
    );
    assert_eq!(
        result_span
            .fields
            .get(observability::FIELD_DIAGNOSTIC_COUNT)
            .map(String::as_str),
        Some("0")
    );
    let event_count = report.summary.event_count.to_string();
    assert_eq!(
        result_span.fields.get(observability::FIELD_EVENT_COUNT),
        Some(&event_count)
    );
    result_span
        .fields
        .get(observability::FIELD_DURATION_MS)
        .expect("expected duration_ms field")
        .parse::<u64>()
        .expect("duration_ms field should be numeric");
}

fn e2e_hook_dispatcher() -> HookDispatcher {
    HookDispatcher::new(
        HookRegistry::new()
            .with_registration(HookRegistration::new(
                "pre-tool",
                HookEventName::PreToolUse,
                HookHandlerType::Command,
                Arc::new(E2eHook::new("pre-tool-run", "pre-tool")),
            ))
            .with_registration(HookRegistration::new(
                "post-tool",
                HookEventName::PostToolUse,
                HookHandlerType::Command,
                Arc::new(E2eHook::new("post-tool-run", "post-tool")),
            ))
            .with_registration(HookRegistration::new(
                "sub-agent-start",
                HookEventName::SubAgentStart,
                HookHandlerType::Command,
                Arc::new(E2eHook::new("sub-agent-start-run", "sub-agent-start")),
            ))
            .with_registration(HookRegistration::new(
                "sub-agent-stop",
                HookEventName::SubAgentStop,
                HookHandlerType::Command,
                Arc::new(E2eHook::new("sub-agent-stop-run", "sub-agent-stop")),
            )),
    )
}

fn environment_label(context: &RuntimeContext) -> String {
    let environment = context.environment();
    let home = environment
        .home
        .as_ref()
        .expect("custom runtime home should be present");
    let profile = home.profile.as_deref().unwrap_or("none");

    format!("home {} profile {profile}", home.path.display())
}

#[derive(Clone, Debug)]
struct E2eProvider;

impl ProviderRuntime for E2eProvider {
    type Request = String;
    type Response = String;

    fn run_provider(
        &self,
        request: Self::Request,
        context: RuntimeContext,
    ) -> RuntimeFuture<Self::Response> {
        Box::pin(async move {
            context
                .emit(RuntimeEvent::new(
                    "test.e2e",
                    format!("provider {request} {}", environment_label(&context)),
                ))
                .await
                .expect("provider event should be emitted");
            request
        })
    }
}

#[derive(Clone, Debug)]
struct E2eTool;

impl ToolRuntime for E2eTool {
    type Invocation = String;
    type Output = String;

    fn run_tool(
        &self,
        invocation: Self::Invocation,
        context: RuntimeContext,
    ) -> RuntimeFuture<Self::Output> {
        Box::pin(async move {
            context
                .emit(RuntimeEvent::new(
                    "test.e2e",
                    format!("tool {invocation} {}", environment_label(&context)),
                ))
                .await
                .expect("tool event should be emitted");
            invocation
        })
    }
}

#[derive(Clone, Debug)]
struct E2eSubAgent;

impl SubAgentRuntime for E2eSubAgent {
    type Input = String;
    type Output = String;

    fn run_sub_agent(
        &self,
        input: Self::Input,
        context: RuntimeContext,
    ) -> RuntimeFuture<Self::Output> {
        Box::pin(async move {
            context
                .emit(RuntimeEvent::new(
                    "test.e2e",
                    format!("sub-agent {input} {}", environment_label(&context)),
                ))
                .await
                .expect("sub-agent event should be emitted");
            input
        })
    }
}

#[derive(Clone, Debug)]
struct E2eHook {
    id: &'static str,
    label: &'static str,
}

impl E2eHook {
    fn new(id: &'static str, label: &'static str) -> Self {
        Self { id, label }
    }
}

impl HookRuntime for E2eHook {
    type Request = HookInvocation;
    type Output = HookRunSummary;

    fn run_hook(
        &self,
        request: Self::Request,
        context: RuntimeContext,
    ) -> RuntimeFuture<Self::Output> {
        let id = self.id;
        let label = self.label;
        Box::pin(async move {
            context
                .emit(RuntimeEvent::new("test.e2e", format!("hook {label}")))
                .await
                .expect("hook event should be emitted");
            HookRunSummary::running(id, request.event_name, HookHandlerType::Command).completed()
        })
    }
}
