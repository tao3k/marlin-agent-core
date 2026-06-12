//! Session binding for model route decisions.

use std::sync::Arc;

use marlin_agent_protocol::{
    ModelContextForkMode, ModelRouteDecision, ModelRouteReceipt, ModelRouteRequest,
    ModelSessionLifecycle, RuntimeEnvironmentActivationReceipt,
};
use marlin_agent_sessions::{
    AgentSessionContext, ContextNamespace, ContextVisibility, SessionId, SessionIsolationReceipt,
    SessionKind,
};

use crate::tokio_runtime::{RuntimeTask, SubAgentRuntime, TokioAgentRuntime};

use super::resolver::CompiledModelRouteResolver;

/// Resolved sub-agent spawn output for a matched model route request.
pub type RoutedSubAgentSpawn<T> = (RuntimeTask<T>, ModelRouteSessionBinding, ModelRouteDecision);

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

    pub fn child_session_id(&self) -> &SessionId {
        self.isolation_receipt.child_session_id()
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
