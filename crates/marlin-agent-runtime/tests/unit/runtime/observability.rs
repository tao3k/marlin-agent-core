use std::{
    fmt,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use marlin_agent_protocol::{ExecutorName, NodeId};
use marlin_agent_runtime::{
    ProviderRuntime, RuntimeContext, RuntimeFuture, TokioAgentRuntime, observability,
};
use parking_lot::Mutex;
use tracing::{
    Event, Metadata, Subscriber,
    field::{Field, Visit},
    span::{Attributes, Id, Record},
};

#[path = "observability/process_cleanup.rs"]
mod process_cleanup;
#[path = "observability/process_lifecycle.rs"]
mod process_lifecycle;

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
    assert_eq!(observability::FIELD_OBSERVED_AT_MS, "observed_at_ms");
    assert_eq!(observability::TARGET_RUNTIME_PROCESS, "runtime.process");
    assert_eq!(observability::FIELD_PROCESS_ID, "pid");
    assert_eq!(observability::FIELD_PROCESS_EVENT, "process_event");
    assert_eq!(observability::FIELD_PROCESS_HANDLE, "process_handle");
    assert_eq!(observability::FIELD_PROCESS_KIND, "process_kind");
    assert_eq!(observability::FIELD_PROCESS_STATUS, "process_status");
    assert_eq!(observability::FIELD_OWNER_REFERENCE, "owner_reference");
    assert_eq!(observability::FIELD_CLEANUP_OUTCOME, "cleanup_outcome");
    assert_eq!(
        observability::FIELD_CLEANUP_CANDIDATE_COUNT,
        "cleanup_candidate_count"
    );
    assert_eq!(
        observability::FIELD_REMOVED_STALE_COUNT,
        "removed_stale_count"
    );
    assert_eq!(
        observability::FIELD_TERMINATION_REQUESTED_COUNT,
        "termination_requested_count"
    );
    assert_eq!(
        observability::FIELD_TERMINATION_FAILED_COUNT,
        "termination_failed_count"
    );
    assert_eq!(
        observability::FIELD_RETAINED_IN_REGISTRY,
        "retained_in_registry"
    );
    assert_eq!(
        observability::FIELD_REQUIRES_FOLLOW_UP,
        "requires_follow_up"
    );
    assert_eq!(
        observability::FIELD_FORCE_CLEANUP_RECOMMENDED,
        "force_cleanup_recommended"
    );
    assert_eq!(observability::PROCESS_EVENT_TRACKED, "tracked");
    assert_eq!(observability::PROCESS_EVENT_CLEANUP_SWEEP, "cleanup_sweep");
    assert_eq!(
        observability::PROCESS_EVENT_CLEANUP_REQUESTED,
        "cleanup_requested"
    );
    assert_eq!(
        observability::PROCESS_EVENT_TERMINATION_FAILED,
        "termination_failed"
    );
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

#[test]
fn runtime_tracing_subscriber_config_validates_receipt() {
    let receipt = observability::RuntimeTracingSubscriberConfig::new()
        .with_env_filter("marlin_agent_runtime=debug,info")
        .with_format(observability::RuntimeTracingSubscriberFormat::Json)
        .with_ansi(false)
        .with_target(true)
        .with_thread_ids(true)
        .validate()
        .expect("subscriber config should validate");

    assert_eq!(receipt.env_filter(), "marlin_agent_runtime=debug,info");
    assert_eq!(
        receipt.format(),
        observability::RuntimeTracingSubscriberFormat::Json
    );
    assert_eq!(receipt.format().as_str(), "json");
    assert!(receipt.target_enabled());
    assert!(receipt.thread_ids_enabled());
    assert_eq!(
        receipt.scope(),
        observability::RuntimeTracingSubscriberScope::Validated
    );
}

#[test]
fn runtime_tracing_subscriber_config_rejects_invalid_filter() {
    let error = observability::RuntimeTracingSubscriberConfig::new()
        .with_env_filter("=info")
        .validate()
        .expect_err("invalid filter should be rejected");

    assert!(matches!(
        error,
        observability::RuntimeTracingSubscriberConfigError::InvalidEnvFilter { .. }
    ));
}

#[test]
fn runtime_tracing_subscriber_uses_scoped_default_without_global_install() {
    let scoped = observability::RuntimeTracingSubscriberConfig::new()
        .with_env_filter("runtime.test=info,info")
        .with_format(observability::RuntimeTracingSubscriberFormat::Compact)
        .with_scoped_default(|receipt| {
            tracing::info!(
                target: "runtime.test",
                runtime_kind = observability::RUNTIME_KIND_PROVIDER,
                "scoped subscriber observed"
            );
            (receipt.scope(), receipt.format())
        })
        .expect("scoped subscriber should be installed for closure");

    assert_eq!(
        scoped,
        (
            observability::RuntimeTracingSubscriberScope::ScopedDefault,
            observability::RuntimeTracingSubscriberFormat::Compact,
        )
    );
    assert_eq!(
        observability::RuntimeTracingSubscriberScope::ScopedDefault.as_str(),
        "scoped-default"
    );
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
    events: Arc<Mutex<Vec<RecordedEvent>>>,
    next_id: Arc<AtomicU64>,
}

impl RecordingSubscriber {
    fn new() -> Self {
        Self {
            spans: Arc::new(Mutex::new(Vec::new())),
            events: Arc::new(Mutex::new(Vec::new())),
            next_id: Arc::new(AtomicU64::new(1)),
        }
    }

    fn span_names(&self) -> Vec<&'static str> {
        self.spans.lock().clone()
    }

    fn events(&self) -> Vec<RecordedEvent> {
        self.events.lock().clone()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RecordedEvent {
    target: &'static str,
    fields: Vec<RecordedField>,
}

impl RecordedEvent {
    fn has_field(&self, name: &str) -> bool {
        self.fields.iter().any(|field| field.name == name)
    }

    fn has_value(&self, name: &str, value: &str) -> bool {
        self.fields
            .iter()
            .any(|field| field.name == name && field.value == value)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RecordedField {
    name: &'static str,
    value: String,
}

#[derive(Default)]
struct EventFieldRecorder {
    fields: Vec<RecordedField>,
}

impl EventFieldRecorder {
    fn push(&mut self, field: &Field, value: String) {
        self.fields.push(RecordedField {
            name: field.name(),
            value,
        });
    }
}

impl Visit for EventFieldRecorder {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.push(field, format!("{value:?}"));
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.push(field, value.to_owned());
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.push(field, value.to_string());
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.push(field, value.to_string());
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.push(field, value.to_string());
    }
}

impl Subscriber for RecordingSubscriber {
    fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, attributes: &Attributes<'_>) -> Id {
        self.spans.lock().push(attributes.metadata().name());
        Id::from_u64(self.next_id.fetch_add(1, Ordering::Relaxed))
    }

    fn record(&self, _span: &Id, _values: &Record<'_>) {}

    fn record_follows_from(&self, _span: &Id, _follows: &Id) {}

    fn event(&self, event: &Event<'_>) {
        let mut recorder = EventFieldRecorder::default();
        event.record(&mut recorder);
        self.events.lock().push(RecordedEvent {
            target: event.metadata().target(),
            fields: recorder.fields,
        });
    }

    fn enter(&self, _span: &Id) {}

    fn exit(&self, _span: &Id) {}
}
