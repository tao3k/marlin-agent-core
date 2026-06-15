//! Session binding for model route decisions.

use std::{collections::BTreeMap, sync::Arc};

use marlin_agent_environment::{
    DirenvCommandRunner, RuntimeEnvironmentActivationRequest, RuntimeEnvironmentActivationResult,
    RuntimeEnvironmentActivator,
};
use marlin_agent_protocol::{
    AgentSessionFact, AgentSessionHistoryLimit, ContextPackReceipt, GraphQuerySecretVisibility,
    GraphQueryVisibility, GraphQueryVisibleSurface, ModelContextForkMode, ModelRouteDecision,
    ModelRouteReceipt, ModelRouteRequest, ModelSessionLifecycle, ProjectRuntimeContextPackId,
    ProjectRuntimeProjectId, ProjectRuntimeReceiptId, RuntimeEnvironmentActivationPolicy,
    RuntimeEnvironmentActivationReceipt, SubAgentSpawnConfig,
};
use marlin_agent_sessions::{
    AgentSessionContext, ContextNamespace, ContextVisibility, SessionId, SessionIsolationReceipt,
    SessionKind,
};

use crate::tokio_runtime::{RuntimeTask, SubAgentRuntime, TokioAgentRuntime};

use super::resolver::CompiledModelRouteResolver;

/// Resolved sub-agent spawn output for a matched model route request.
pub type RoutedSubAgentSpawn<T> = (RuntimeTask<T>, ModelRouteSessionBinding, ModelRouteDecision);

/// Request for spawning a routed sub-agent after applying profile environment activation.
pub struct ActivatedModelRouteProfileSpawnRequest<'a, A, R>
where
    A: SubAgentRuntime,
    R: DirenvCommandRunner,
{
    pub sub_agent: Arc<A>,
    pub input: A::Input,
    pub decision: &'a ModelRouteDecision,
    pub profile: &'a SubAgentSpawnConfig,
    pub activator: &'a RuntimeEnvironmentActivator<R>,
    pub base_environment: BTreeMap<String, String>,
}

/// Runtime receipt produced when a model route is bound to a child session.
#[derive(Clone, Debug)]
pub struct ModelRouteSessionBinding {
    route_receipt: ModelRouteReceipt,
    isolation_receipt: SessionIsolationReceipt,
}

impl ModelRouteSessionBinding {
    pub fn new(
        route_receipt: ModelRouteReceipt,
        isolation_receipt: SessionIsolationReceipt,
    ) -> Self {
        Self {
            route_receipt,
            isolation_receipt,
        }
    }

    pub fn route_receipt(&self) -> &ModelRouteReceipt {
        &self.route_receipt
    }

    pub fn isolation_receipt(&self) -> &SessionIsolationReceipt {
        &self.isolation_receipt
    }

    pub fn environment_activation_receipt(&self) -> Option<&RuntimeEnvironmentActivationReceipt> {
        self.route_receipt.environment_activation.as_ref()
    }

    pub fn with_environment_activation_receipt(
        mut self,
        receipt: RuntimeEnvironmentActivationReceipt,
    ) -> Self {
        self.route_receipt.environment_activation = Some(receipt);
        self
    }

    pub fn with_environment_activation_policy(
        self,
        activation: &RuntimeEnvironmentActivationPolicy,
    ) -> Self {
        self.with_environment_activation_receipt(RuntimeEnvironmentActivationReceipt::planned(
            activation,
        ))
    }

    pub fn with_optional_environment_activation_policy(
        self,
        activation: Option<&RuntimeEnvironmentActivationPolicy>,
    ) -> Self {
        match activation {
            Some(activation) => self.with_environment_activation_policy(activation),
            None => self,
        }
    }

    pub fn child_session_id(&self) -> &SessionId {
        self.isolation_receipt.child_session_id()
    }

    pub fn agent_session_fact(
        &self,
        project_id: ProjectRuntimeProjectId,
        child_session: &AgentSessionContext,
    ) -> AgentSessionFact {
        let mut fact = match child_session.kind() {
            SessionKind::Root => AgentSessionFact::root(
                project_id.as_str(),
                child_session.root_session_id().as_str(),
                child_session.session_id().as_str(),
            ),
            SessionKind::SubAgent => AgentSessionFact::sub_agent(
                project_id.as_str(),
                child_session.root_session_id().as_str(),
                child_session.session_id().as_str(),
                child_session
                    .parent_session_id()
                    .unwrap_or_else(|| self.isolation_receipt.parent_session_id())
                    .as_str(),
            ),
            SessionKind::Provider | SessionKind::Tool | SessionKind::Hook => {
                AgentSessionFact::child(
                    project_id.as_str(),
                    child_session.root_session_id().as_str(),
                    child_session.session_id().as_str(),
                    child_session
                        .parent_session_id()
                        .unwrap_or_else(|| self.isolation_receipt.parent_session_id())
                        .as_str(),
                )
            }
        }
        .with_visibility(graph_query_visibility_from_context(
            child_session.visibility(),
        ));

        if let Some(max_history_items) = child_session.visibility().max_history_items() {
            fact = fact.with_history_limit(AgentSessionHistoryLimit::new(
                max_history_items.min(u16::MAX as usize) as u16,
            ));
        }

        fact
    }

    pub fn context_pack_receipt(
        &self,
        receipt_id: ProjectRuntimeReceiptId,
        context_pack_id: ProjectRuntimeContextPackId,
        child_session: &AgentSessionContext,
    ) -> ContextPackReceipt {
        ContextPackReceipt::new(
            receipt_id.as_str(),
            context_pack_id.as_str(),
            child_session.root_session_id().as_str(),
            child_session.session_id().as_str(),
        )
        .with_visibility(graph_query_visibility_from_context(
            child_session.visibility(),
        ))
    }
}

impl TokioAgentRuntime {
    pub fn child_runtime_for_model_route(
        &self,
        decision: &ModelRouteDecision,
        kind: SessionKind,
    ) -> (Self, ModelRouteSessionBinding) {
        let child_session_id = child_session_id_for_route(decision);
        let requested_visibility =
            visibility_for_context_fork(self.session(), &decision.session.context);
        let (runtime, isolation_receipt) =
            self.child_runtime_for_session(kind, child_session_id, requested_visibility);
        (
            runtime,
            ModelRouteSessionBinding::new(decision.receipt.clone(), isolation_receipt),
        )
    }

    pub fn child_runtime_for_model_route_profile(
        &self,
        decision: &ModelRouteDecision,
        kind: SessionKind,
        profile: &SubAgentSpawnConfig,
    ) -> (Self, ModelRouteSessionBinding) {
        let (runtime, binding) = self.child_runtime_for_model_route(decision, kind);
        (
            runtime,
            binding.with_optional_environment_activation_policy(
                profile.environment_activation.as_ref(),
            ),
        )
    }

    pub fn spawn_sub_agent_with_model_route<A>(
        &self,
        sub_agent: Arc<A>,
        input: A::Input,
        decision: &ModelRouteDecision,
    ) -> (RuntimeTask<A::Output>, ModelRouteSessionBinding)
    where
        A: SubAgentRuntime,
    {
        let (runtime, binding) =
            self.child_runtime_for_model_route(decision, SessionKind::SubAgent);
        (runtime.spawn_sub_agent(sub_agent, input), binding)
    }

    pub fn spawn_sub_agent_with_model_route_profile<A>(
        &self,
        sub_agent: Arc<A>,
        input: A::Input,
        decision: &ModelRouteDecision,
        profile: &SubAgentSpawnConfig,
    ) -> (RuntimeTask<A::Output>, ModelRouteSessionBinding)
    where
        A: SubAgentRuntime,
    {
        let (runtime, binding) =
            self.child_runtime_for_model_route_profile(decision, SessionKind::SubAgent, profile);
        (runtime.spawn_sub_agent(sub_agent, input), binding)
    }

    pub async fn spawn_sub_agent_with_activated_model_route_profile<A, R>(
        &self,
        request: ActivatedModelRouteProfileSpawnRequest<'_, A, R>,
    ) -> (
        RuntimeTask<A::Output>,
        ModelRouteSessionBinding,
        RuntimeEnvironmentActivationResult,
    )
    where
        A: SubAgentRuntime,
        R: DirenvCommandRunner,
    {
        let activation_policy = request
            .profile
            .environment_activation
            .clone()
            .unwrap_or_default();
        let runtime_environment = self
            .environment()
            .clone()
            .with_activation(activation_policy);
        let activation_result = request
            .activator
            .activate(RuntimeEnvironmentActivationRequest::new(
                runtime_environment.clone(),
                request.base_environment,
            ))
            .await;
        let (runtime, binding) = self.child_runtime_for_model_route_profile(
            request.decision,
            SessionKind::SubAgent,
            request.profile,
        );
        let runtime = runtime.with_runtime_environment(runtime_environment);
        let binding =
            binding.with_environment_activation_receipt(activation_result.receipt.clone());

        (
            runtime.spawn_sub_agent(request.sub_agent, request.input),
            binding,
            activation_result,
        )
    }

    pub fn spawn_routed_sub_agent<A>(
        &self,
        sub_agent: Arc<A>,
        input: A::Input,
        resolver: &CompiledModelRouteResolver,
        request: &ModelRouteRequest,
    ) -> Option<RoutedSubAgentSpawn<A::Output>>
    where
        A: SubAgentRuntime,
    {
        let decision = resolver.resolve(request)?;
        let (task, binding) = self.spawn_sub_agent_with_model_route(sub_agent, input, &decision);
        Some((task, binding, decision))
    }
}

fn child_session_id_for_route(decision: &ModelRouteDecision) -> SessionId {
    if let Some(session_id) = &decision.session.requested_session_id {
        return SessionId::from(session_id.as_str().to_owned());
    }

    match &decision.session.lifecycle {
        ModelSessionLifecycle::Ephemeral => SessionId::from(format!(
            "model-route/{}/ephemeral",
            decision.receipt.rule_id.as_str()
        )),
        ModelSessionLifecycle::Persistent { key } => {
            SessionId::from(format!("model-route/persistent/{}", key.as_str()))
        }
        ModelSessionLifecycle::Pooled { pool } => {
            SessionId::from(format!("model-route/pooled/{}", pool.as_str()))
        }
    }
}

fn visibility_for_context_fork(
    parent: &AgentSessionContext,
    fork: &ModelContextForkMode,
) -> ContextVisibility {
    match fork {
        ModelContextForkMode::ForkSnapshot | ModelContextForkMode::SharedLive => {
            parent.visibility().clone()
        }
        ModelContextForkMode::Minimal => ContextVisibility::from_namespaces([
            ContextNamespace::System,
            ContextNamespace::Workspace,
        ])
        .with_max_history_items(Some(8)),
        ModelContextForkMode::Isolated => {
            ContextVisibility::from_namespaces([ContextNamespace::System])
                .with_max_history_items(Some(0))
        }
    }
}

fn graph_query_visibility_from_context(visibility: &ContextVisibility) -> GraphQueryVisibility {
    let mut surfaces = Vec::new();
    push_surface_if(
        &mut surfaces,
        visibility.contains(&ContextNamespace::Workspace),
        GraphQueryVisibleSurface::Workspace,
    );
    push_surface_if(
        &mut surfaces,
        visibility.contains(&ContextNamespace::Memory),
        GraphQueryVisibleSurface::Memory,
    );
    push_surface_if(
        &mut surfaces,
        visibility.contains(&ContextNamespace::Tools),
        GraphQueryVisibleSurface::Tools,
    );
    push_surface_if(
        &mut surfaces,
        visibility.contains(&ContextNamespace::Hooks)
            || visibility.contains(&ContextNamespace::SubAgents),
        GraphQueryVisibleSurface::Sessions,
    );
    push_surface_if(
        &mut surfaces,
        visibility.max_history_items() != Some(0),
        GraphQueryVisibleSurface::Content,
    );

    GraphQueryVisibility {
        surfaces,
        secrets: if visibility.contains(&ContextNamespace::Secrets) {
            GraphQuerySecretVisibility::Allowed
        } else {
            GraphQuerySecretVisibility::Denied
        },
    }
}

fn push_surface_if(
    surfaces: &mut Vec<GraphQueryVisibleSurface>,
    condition: bool,
    surface: GraphQueryVisibleSurface,
) {
    if condition && !surfaces.contains(&surface) {
        surfaces.push(surface);
    }
}
