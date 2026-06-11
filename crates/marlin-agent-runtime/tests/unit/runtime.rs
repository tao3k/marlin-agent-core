use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
};

use marlin_agent_protocol::{ExecutorName, NodeId, RuntimeHome, RuntimeSandboxPolicy};
use marlin_agent_runtime::{
    CancellationToken, HookRuntime, ProviderRuntime, RuntimeContext, RuntimeEnvironment,
    RuntimeFuture, SubAgentRuntime, TokioAgentRuntime, observability,
};
use tokio_stream::StreamExt;
use tracing::{
    Event, Metadata, Subscriber,
    span::{Attributes, Id, Record},
};

#[tokio::test]
async fn runtime_emits_protocol_owned_events() {
    let (runtime, mut events) = TokioAgentRuntime::new(4);

    runtime
        .context()
        .emit(observability::runtime_event(
            "runtime.test".into(),
            "observed",
        ))
        .await
        .expect("event sink should be open");

    let event = events.next().await.expect("event should be emitted");
    assert_eq!(event.topic, "runtime.test");
    assert_eq!(event.message, "observed");
}

#[tokio::test]
async fn runtime_context_exposes_custom_environment() {
    let environment = RuntimeEnvironment::default()
        .with_home(RuntimeHome::custom("/tmp/marlin-home").with_profile("runtime"))
        .with_cwd("/tmp/workspace")
        .with_sandbox(RuntimeSandboxPolicy {
            writable_roots: vec!["/tmp/workspace".into()],
            network_access: true,
            exclude_tmpdir_env_var: false,
            exclude_slash_tmp: true,
        });

    let (runtime, _events) =
        TokioAgentRuntime::with_environment(4, CancellationToken::new(), environment.clone());

    assert_eq!(runtime.environment(), &environment);
    assert_eq!(runtime.context().environment(), &environment);
}

#[tokio::test]
async fn sub_agent_can_run_with_child_environment() {
    let parent_environment = RuntimeEnvironment::default().with_cwd("/tmp/parent");
    let child_environment = RuntimeEnvironment::default()
        .with_home(RuntimeHome::custom("/tmp/marlin-home/sub/reviewer"))
        .with_cwd("/tmp/child");
    let (runtime, _events) = TokioAgentRuntime::with_environment(
        4,
        CancellationToken::new(),
        parent_environment.clone(),
    );

    let output = runtime
        .spawn_sub_agent_with_environment(
            Arc::new(EnvironmentEchoSubAgent),
            (),
            child_environment.clone(),
        )
        .join()
        .await
        .expect("sub-agent task should finish");

    assert_eq!(runtime.environment(), &parent_environment);
    assert_eq!(output, child_environment);
}

#[tokio::test]
async fn hook_runtime_executes_with_runtime_environment() {
    let environment = RuntimeEnvironment::default()
        .with_home(RuntimeHome::custom("/tmp/marlin-home"))
        .with_cwd("/tmp/workspace");
    let (runtime, _events) =
        TokioAgentRuntime::with_environment(4, CancellationToken::new(), environment.clone());

    let (request, output_environment) = runtime
        .spawn_hook(Arc::new(EnvironmentEchoHook), "pre-tool".to_owned())
        .join()
        .await
        .expect("hook task should finish");

    assert_eq!(request, "pre-tool");
    assert_eq!(output_environment, environment);
}

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
    assert_eq!(observability::FIELD_NODE_KIND, "node_kind");
    assert_eq!(observability::FIELD_HOOK_EVENT, "hook_event");

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
struct EnvironmentEchoSubAgent;

impl SubAgentRuntime for EnvironmentEchoSubAgent {
    type Input = ();
    type Output = RuntimeEnvironment;

    fn run_sub_agent(
        &self,
        _input: Self::Input,
        context: RuntimeContext,
    ) -> RuntimeFuture<Self::Output> {
        let environment = context.environment().clone();
        Box::pin(async move { environment })
    }
}

#[derive(Clone, Debug)]
struct EnvironmentEchoHook;

impl HookRuntime for EnvironmentEchoHook {
    type Request = String;
    type Output = (String, RuntimeEnvironment);

    fn run_hook(
        &self,
        request: Self::Request,
        context: RuntimeContext,
    ) -> RuntimeFuture<Self::Output> {
        let environment = context.environment().clone();
        Box::pin(async move { (request, environment) })
    }
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
