use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
};

use marlin_agent_protocol::{ExecutorName, NodeId};
use marlin_agent_runtime::{
    ProviderRuntime, RuntimeContext, RuntimeFuture, TokioAgentRuntime, observability,
};
use tracing::{
    Event, Metadata, Subscriber,
    span::{Attributes, Id, Record},
};

#[tokio::test]
async fn runtime_provider_spawn_creates_tracing_span() {
    let subscriber = RecordingSubscriber::new();
    let _guard = tracing::subscriber::set_default(subscriber.clone());
    let (runtime, _events) = TokioAgentRuntime::new(4);

    let output = runtime
        .spawn_provider(Arc::new(EchoProvider), "hello".to_owned())
        .join()
        .await
        .expect("provider task should finish");

    assert_eq!(output, "hello");
    assert!(
        subscriber
            .span_names()
            .contains(&observability::SPAN_RUNTIME_PROVIDER),
        "provider runtime span should be recorded"
    );
}

#[test]
fn observability_contract_names_kernel_hook_and_agent_surfaces() {
    let hook_event = observability::kernel_hook_event("hook observed");
    let sub_agent_event = observability::kernel_sub_agent_event("sub-agent observed");

    assert_eq!(hook_event.topic, observability::TOPIC_KERNEL_HOOK);
    assert_eq!(sub_agent_event.topic, observability::TOPIC_KERNEL_SUB_AGENT);
    assert_eq!(
        observability::agent_provider_span_name().as_str(),
        observability::SPAN_AGENT_PROVIDER
    );
    assert_eq!(
        observability::harness_result_span_name().as_str(),
        observability::SPAN_HARNESS_RESULT
    );
    assert_eq!(observability::FIELD_NODE_KIND, "node_kind");
    assert_eq!(observability::FIELD_HOOK_EVENT, "hook_event");
    assert_eq!(observability::FIELD_PARENT_RUN_ID, "parent_run_id");
    assert_eq!(observability::FIELD_CHILD_RUN_ID, "child_run_id");
    assert_eq!(observability::FIELD_SUB_AGENT_SOURCE, "sub_agent_source");
    assert_eq!(observability::FIELD_AGENT_REFERENCE, "agent_reference");
    assert_eq!(observability::FIELD_STATUS, "status");
    assert_eq!(observability::FIELD_DURATION_MS, "duration_ms");
    assert_eq!(observability::FIELD_DIAGNOSTIC_COUNT, "diagnostic_count");
    assert_eq!(observability::FIELD_EVENT_COUNT, "event_count");
    assert_eq!(
        observability::SUB_AGENT_SOURCE_KERNEL_NODE,
        "kernel.sub-agent-node"
    );

    let subscriber = RecordingSubscriber::new();
    let _guard = tracing::subscriber::set_default(subscriber.clone());

    let provider_node_id = NodeId::new("plan");
    let provider_executor = ExecutorName::new("provider");
    let tool_node_id = NodeId::new("apply");
    let tool_executor = ExecutorName::new("tool");
    let sub_agent_node_id = NodeId::new("review");
    let sub_agent_executor = ExecutorName::new("sub-agent");

    drop(observability::agent_provider_span(
        &provider_node_id,
        &provider_executor,
    ));
    drop(observability::agent_tool_span(
        &tool_node_id,
        &tool_executor,
    ));
    drop(observability::agent_sub_agent_span(
        &sub_agent_node_id,
        &sub_agent_executor,
    ));
    drop(observability::hook_dispatch_span("PreToolUse", 1));
    drop(observability::hook_run_span(
        observability::HookRegistrationId::borrowed("pre-tool"),
        "PreToolUse",
        "Sync",
        "Command",
    ));

    let span_names = subscriber.span_names();
    assert!(span_names.contains(&observability::SPAN_AGENT_PROVIDER));
    assert!(span_names.contains(&observability::SPAN_AGENT_TOOL));
    assert!(span_names.contains(&observability::SPAN_AGENT_SUB_AGENT));
    assert!(span_names.contains(&observability::SPAN_HOOK_DISPATCH));
    assert!(span_names.contains(&observability::SPAN_HOOK_RUN));
}

#[derive(Clone, Debug)]
struct EchoProvider;

impl ProviderRuntime for EchoProvider {
    type Request = String;
    type Response = String;

    fn run_provider(
        &self,
        request: Self::Request,
        _context: RuntimeContext,
    ) -> RuntimeFuture<Self::Response> {
        Box::pin(async move { request })
    }
}

#[derive(Clone, Default)]
struct RecordingSubscriber {
    spans: Arc<Mutex<Vec<&'static str>>>,
    next_id: Arc<AtomicU64>,
}

impl RecordingSubscriber {
    fn new() -> Self {
        Self {
            spans: Arc::new(Mutex::new(Vec::new())),
            next_id: Arc::new(AtomicU64::new(1)),
        }
    }

    fn span_names(&self) -> Vec<&'static str> {
        self.spans
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}

impl Subscriber for RecordingSubscriber {
    fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, attributes: &Attributes<'_>) -> Id {
        self.spans
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(attributes.metadata().name());
        Id::from_u64(self.next_id.fetch_add(1, Ordering::Relaxed))
    }

    fn record(&self, _span: &Id, _values: &Record<'_>) {}

    fn record_follows_from(&self, _span: &Id, _follows: &Id) {}

    fn event(&self, _event: &Event<'_>) {}

    fn enter(&self, _span: &Id) {}

    fn exit(&self, _span: &Id) {}
}
