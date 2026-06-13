use std::sync::Arc;

use marlin_agent_harness::{
    AgentHarness, HarnessExecutionReport, HarnessGraphBuilder, HarnessRuntime,
};
use marlin_agent_hooks::{HookDispatcher, HookInvocation, HookRegistration, HookRegistry};
use marlin_agent_kernel::{
    GraphLoopExecutionBudget, GraphLoopExecutionRequest, GraphLoopExecutionStatus,
    GraphNodeInvocation, ProviderNodeAdapter, SubAgentNodeAdapter, TokioGraphLoopKernel,
    ToolNodeAdapter,
};
use marlin_agent_protocol::{
    AgentScenario, AgentScenarioStep, HookEventName, HookHandlerType, HookRunSummary, LoopEvidence,
    LoopEvidenceKind, RuntimeHome,
};
use marlin_agent_runtime::{
    HookRuntime, ProviderRuntime, RuntimeContext, RuntimeEnvironment, RuntimeEvent, RuntimeFuture,
    SubAgentRuntime, ToolRuntime, observability,
};
use marlin_agent_sessions::SessionKind;
use marlin_agent_test_support::{
    accepted_gerbil_ir_graph_policy_proposal_fixture,
    assert_accepted_gerbil_ir_graph_policy_proposal_fixture,
    assert_budgeted_graph_policy_execution_request,
    assert_deterministic_sub_agent_scenario_fixture, assert_sub_agent_memory_session_fixture,
    budgeted_graph_policy_execution_request_fixture,
    deterministic_reviewer_sub_agent_scenario_fixture, hook_dispatch_replay_evidence,
    sub_agent_memory_session_replay_evidence, sub_agent_memory_session_visibility_evidence,
};

#[tokio::test]
async fn harness_runs_provider_tool_sub_agent_scenario_with_hooks_and_environment() {
    let sub_agent_fixture = deterministic_reviewer_sub_agent_scenario_fixture();
    assert_deterministic_sub_agent_scenario_fixture(&sub_agent_fixture);
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

#[tokio::test]
async fn harness_e2e_combines_graph_policy_environment_hooks_and_sub_agent_session_without_live_llm()
 {
    let graph_policy = accepted_gerbil_ir_graph_policy_proposal_fixture();
    let graph_policy_request = budgeted_graph_policy_execution_request_fixture(&graph_policy, 2);
    assert_accepted_gerbil_ir_graph_policy_proposal_fixture(&graph_policy);
    assert_budgeted_graph_policy_execution_request(&graph_policy_request, 2);

    let sub_agent_fixture = deterministic_reviewer_sub_agent_scenario_fixture();
    let session = sub_agent_fixture.session_fixture();
    let (child_session, isolation_receipt) = session.parent_session().child_session(
        SessionKind::SubAgent,
        session.config().child_session_id(),
        session.requested_visibility(),
    );
    assert_deterministic_sub_agent_scenario_fixture(&sub_agent_fixture);
    assert_sub_agent_memory_session_fixture(
        session,
        &child_session,
        session.config(),
        &isolation_receipt,
    );

    let environment = RuntimeEnvironment::default()
        .with_home(RuntimeHome::custom("/tmp/marlin-policy-e2e-home").with_profile("policy-e2e"))
        .with_cwd("/tmp/marlin-policy-e2e-workspace");
    let scenario = AgentScenario::new("graph-policy-provider-tool-sub-agent")
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
        .expecting_evidence(LoopEvidenceKind::Runtime)
        .expecting_evidence(LoopEvidenceKind::Visibility);
    let request = GraphLoopExecutionRequest::new(
        "run-policy-e2e",
        HarnessGraphBuilder::new("policy-e2e-graph")
            .linear([
                ("plan", "provider"),
                ("apply", "tool"),
                ("review", "sub-agent"),
            ])
            .build(),
    )
    .with_budget(GraphLoopExecutionBudget::max_node_executions(3));
    let hook_dispatcher = e2e_hook_dispatcher();
    let kernel = TokioGraphLoopKernel::new("run-policy-e2e", "policy-e2e-graph")
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
    let mut harness = HarnessRuntime::with_environment(64, environment);
    harness.record_evidence(LoopEvidence::present(LoopEvidenceKind::Runtime, "tokio"));
    harness.record_graph_policy_proposal_visibility(&graph_policy.compilation().receipt);
    harness.record_evidence(hook_dispatch_replay_evidence(
        sub_agent_fixture.start_hook_summary(),
        sub_agent_fixture.hook_selection(),
        sub_agent_fixture.hook_policy(),
    ));
    harness.record_evidence(sub_agent_memory_session_visibility_evidence(
        &child_session,
        &isolation_receipt,
    ));
    harness.record_evidence(sub_agent_memory_session_replay_evidence(
        &child_session,
        &isolation_receipt,
    ));

    let report = harness.execute_graph(&scenario, &kernel, request).await;
    let evaluated = AgentHarness::evaluate_execution_report(&scenario, &report);
    let e2e_messages = report
        .events
        .iter()
        .filter(|event| event.topic == "test.e2e")
        .map(|event| event.message.as_str())
        .collect::<Vec<_>>();

    assert_eq!(report.result.status, GraphLoopExecutionStatus::Completed);
    assert_eq!(report.result.visited_nodes, vec!["plan", "apply", "review"]);
    assert!(report.assertion.is_none());
    assert!(evaluated.is_success());
    assert!(report.has_graph_policy_proposal_visibility_status(
        &graph_policy.proposal().strategy.strategy_id,
        graph_policy.compilation().receipt.status.clone(),
    ));
    assert_eq!(
        report
            .evidence
            .iter()
            .filter(|evidence| evidence.kind == LoopEvidenceKind::Visibility)
            .count(),
        3
    );
    assert_eq!(
        report
            .evidence
            .iter()
            .filter(|evidence| evidence.kind == LoopEvidenceKind::Runtime)
            .count(),
        2
    );
    assert!(evidence_detail_contains(&report, "memory_visible=true"));
    assert!(evidence_detail_contains(&report, "denied_memory=false"));
    assert!(evidence_detail_contains(
        &report,
        "requested_namespaces=[System,User,Workspace,Memory]"
    ));
    assert!(evidence_detail_contains(
        &report,
        "granted_namespaces=[System,User,Workspace,Memory]"
    ));
    assert!(evidence_detail_contains(&report, "denied_namespaces=[]"));
    assert!(evidence_detail_contains(
        &report,
        "history_limit_applied=true"
    ));
    assert!(evidence_detail_contains(
        &report,
        "visibility_contracted=true"
    ));
    assert!(evidence_detail_contains(
        &report,
        "matcher_strategy=AhoCorasickEventIndex"
    ));
    assert!(evidence_detail_contains(
        &report,
        "policy_mode=EnforceTrusted"
    ));
    assert!(evidence_detail_contains(
        &report,
        "policy_extension_kind=GerbilScheme"
    ));
    assert!(evidence_detail_contains(
        &report,
        "selection_agent_scope=SubAgent"
    ));
    assert!(evidence_detail_contains(&report, "live_llm=false"));
    assert!(e2e_messages.contains(&"hook pre-tool"));
    assert!(e2e_messages.contains(&"hook sub-agent-start"));
    assert!(e2e_messages.contains(&"hook sub-agent-stop"));
    assert!(
        e2e_messages
            .iter()
            .any(|message| message.contains("provider plan home /tmp/marlin-policy-e2e-home"))
    );
    assert!(
        e2e_messages
            .iter()
            .any(|message| message.contains("sub-agent review home /tmp/marlin-policy-e2e-home"))
    );
}

#[tokio::test]
async fn harness_e2e_preserves_graph_policy_visibility_when_budget_gate_fails_without_live_llm() {
    let graph_policy = accepted_gerbil_ir_graph_policy_proposal_fixture();
    let request = budgeted_graph_policy_execution_request_fixture(&graph_policy, 1);
    assert_accepted_gerbil_ir_graph_policy_proposal_fixture(&graph_policy);
    assert_budgeted_graph_policy_execution_request(&request, 1);

    let scenario = AgentScenario::new("graph-policy-budget-failure")
        .with_step(
            AgentScenarioStep::new("run")
                .expecting_event_topic(observability::TOPIC_KERNEL_EXECUTION)
                .expecting_span_name(observability::harness_result_span_name()),
        )
        .expecting_evidence(LoopEvidenceKind::Visibility);
    let kernel = TokioGraphLoopKernel::new(
        graph_policy.expected_run_id(),
        graph_policy.proposal().proposed_graph.graph_id.clone(),
    )
    .with_executor(
        "gerbil.rank",
        ProviderNodeAdapter::new(E2eProvider, |invocation: GraphNodeInvocation| {
            invocation.node_id.into_string()
        }),
    )
    .with_executor(
        "kernel.dispatch",
        ToolNodeAdapter::new(E2eTool, |invocation: GraphNodeInvocation| {
            invocation.node_id.into_string()
        }),
    );
    let mut harness = HarnessRuntime::new(16);
    harness.record_graph_policy_proposal_visibility(&graph_policy.compilation().receipt);

    let report = harness.execute_graph(&scenario, &kernel, request).await;
    let evaluated = AgentHarness::evaluate_execution_report(&scenario, &report);

    assert_eq!(report.result.status, GraphLoopExecutionStatus::Failed);
    assert_eq!(report.summary.status, GraphLoopExecutionStatus::Failed);
    assert!(report.result.visited_nodes.is_empty());
    assert_eq!(
        report.result.diagnostics,
        vec!["graph execution budget exceeded: planned node executions 2 > max 1"]
    );
    assert_eq!(report.summary.diagnostic_count, 1);
    assert!(report.assertion.is_none());
    assert!(evaluated.is_success());
    assert!(report.has_graph_policy_proposal_visibility_status(
        &graph_policy.proposal().strategy.strategy_id,
        graph_policy.compilation().receipt.status.clone(),
    ));
    assert!(!report.events.iter().any(|event| event.topic == "test.e2e"));

    let result_span = report
        .find_span(&observability::harness_result_span_name())
        .expect("expected budget failure harness result span");
    assert_eq!(
        result_span
            .fields
            .get(observability::FIELD_STATUS)
            .map(String::as_str),
        Some("Failed")
    );
    assert_eq!(
        result_span
            .fields
            .get(observability::FIELD_DIAGNOSTIC_COUNT)
            .map(String::as_str),
        Some("1")
    );
}

fn evidence_detail_contains(report: &HarnessExecutionReport, needle: &str) -> bool {
    report
        .evidence
        .iter()
        .filter_map(|evidence| evidence.detail.as_deref())
        .any(|detail| detail.contains(needle))
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

    let sub_agent_topic = observability::kernel_sub_agent_topic();
    let sub_agent_lifecycle_events = report
        .events_by_topic(&sub_agent_topic)
        .map(|event| event.message.as_str())
        .collect::<Vec<_>>();
    assert_eq!(sub_agent_lifecycle_events.len(), 2);
    assert!(
        sub_agent_lifecycle_events
            .iter()
            .any(|message| message.contains("Started"))
    );
    assert!(
        sub_agent_lifecycle_events
            .iter()
            .any(|message| message.contains("Stopped"))
    );
    assert!(
        sub_agent_lifecycle_events
            .iter()
            .all(|message| message.contains("ThreadSpawn"))
    );
    assert!(
        sub_agent_lifecycle_events
            .iter()
            .all(|message| message.contains("run-e2e"))
    );
    assert!(
        sub_agent_lifecycle_events
            .iter()
            .all(|message| message.contains("depth: 1"))
    );

    let execution_span = report
        .find_span(&observability::harness_execution_span_name())
        .expect("expected harness execution trace span");
    assert_eq!(
        execution_span
            .fields
            .get(observability::FIELD_SCENARIO_ID)
            .map(String::as_str),
        Some("provider-tool-sub-agent")
    );
    assert_eq!(
        execution_span
            .fields
            .get(observability::FIELD_RUN_ID)
            .map(String::as_str),
        Some("run-e2e")
    );
    assert_eq!(
        execution_span
            .fields
            .get(observability::FIELD_GRAPH_ID)
            .map(String::as_str),
        Some("graph")
    );

    let provider_span = report
        .find_span_with_field(
            &observability::agent_provider_span_name(),
            observability::FIELD_NODE_ID,
            "plan",
        )
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
    assert_eq!(
        report
            .spans_with_field(observability::FIELD_EXECUTOR, "provider")
            .count(),
        1
    );
    let sub_agent_span = report
        .find_span_with_field(
            &observability::agent_sub_agent_span_name(),
            observability::FIELD_NODE_ID,
            "review",
        )
        .expect("expected sub-agent trace span");
    assert_eq!(
        sub_agent_span
            .fields
            .get(observability::FIELD_EXECUTOR)
            .map(String::as_str),
        Some("sub-agent")
    );
    assert_eq!(
        sub_agent_span
            .fields
            .get(observability::FIELD_AGENT_REFERENCE)
            .map(String::as_str),
        Some("sub-agent")
    );
    assert_eq!(
        sub_agent_span
            .fields
            .get(observability::FIELD_SUB_AGENT_SOURCE)
            .map(String::as_str),
        Some(observability::SUB_AGENT_SOURCE_KERNEL_NODE)
    );
    assert_eq!(
        sub_agent_span
            .fields
            .get(observability::FIELD_PARENT_RUN_ID)
            .map(String::as_str),
        Some("run-e2e")
    );

    let result_span = report
        .find_span(&observability::harness_result_span_name())
        .expect("expected harness result trace span");
    assert_eq!(
        result_span
            .fields
            .get(observability::FIELD_RUN_ID)
            .map(String::as_str),
        Some("run-e2e")
    );
    assert_eq!(
        result_span
            .fields
            .get(observability::FIELD_GRAPH_ID)
            .map(String::as_str),
        Some("graph")
    );
    assert!(
        report
            .find_span_with_field(
                &observability::harness_result_span_name(),
                observability::FIELD_RUN_ID,
                "run-e2e",
            )
            .is_some()
    );
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
