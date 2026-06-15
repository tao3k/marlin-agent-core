//! Harness adapter for Gerbil/POO graph-loop continuation projections.

use std::{
    fmt::{self, Display, Formatter},
    sync::Arc,
};

use marlin_agent_kernel::{
    GraphLoopContinuationInput, GraphLoopContinuationPlanner, GraphLoopNextAction,
};
use marlin_agent_runtime::RuntimeFuture;
use marlin_gerbil_scheme::{
    GerbilSchemeTypeDecodeError, GerbilSchemeTypeRegistry, GerbilSchemeTypedValue,
    gerbil_loop_graph_continuation_type_manifest,
    project_gerbil_loop_graph_continuation_native_action,
};

/// Runtime hook that invokes the Gerbil native projection plane for one continuation decision.
pub trait AgentHarnessGerbilLoopContinuationProjector: Send + Sync + 'static {
    fn project_continuation(
        &self,
        input: GraphLoopContinuationInput,
    ) -> RuntimeFuture<Result<GerbilSchemeTypedValue, AgentHarnessGerbilLoopContinuationError>>;
}

/// Error raised before Rust can compile a Gerbil continuation projection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AgentHarnessGerbilLoopContinuationError {
    NativeProjectionInvocation { message: String },
    NativeProjectionDecode { message: String },
}

impl AgentHarnessGerbilLoopContinuationError {
    pub fn native_projection_invocation(message: impl Into<String>) -> Self {
        Self::NativeProjectionInvocation {
            message: message.into(),
        }
    }
}

impl Display for AgentHarnessGerbilLoopContinuationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::NativeProjectionInvocation { message } => {
                write!(
                    formatter,
                    "Gerbil continuation native projection invocation failed: {message}"
                )
            }
            Self::NativeProjectionDecode { message } => {
                write!(
                    formatter,
                    "Gerbil continuation native projection decode failed: {message}"
                )
            }
        }
    }
}

impl From<GerbilSchemeTypeDecodeError> for AgentHarnessGerbilLoopContinuationError {
    fn from(error: GerbilSchemeTypeDecodeError) -> Self {
        Self::NativeProjectionDecode {
            message: error.to_string(),
        }
    }
}

/// `GraphLoopContinuationPlanner` backed by a Gerbil native typed projection.
#[derive(Clone)]
pub struct AgentHarnessGerbilLoopContinuationPlanner {
    registry: GerbilSchemeTypeRegistry,
    projector: Arc<dyn AgentHarnessGerbilLoopContinuationProjector>,
}

impl AgentHarnessGerbilLoopContinuationPlanner {
    /// Builds a planner with an explicit Scheme type registry.
    pub fn new<P>(registry: GerbilSchemeTypeRegistry, projector: P) -> Self
    where
        P: AgentHarnessGerbilLoopContinuationProjector,
    {
        Self {
            registry,
            projector: Arc::new(projector),
        }
    }

    /// Builds a planner using the upstream graph-loop continuation projection manifest.
    pub fn from_native_projection_manifest<P>(
        projector: P,
    ) -> Result<Self, GerbilSchemeTypeDecodeError>
    where
        P: AgentHarnessGerbilLoopContinuationProjector,
    {
        GerbilSchemeTypeRegistry::new(gerbil_loop_graph_continuation_type_manifest())
            .map(|registry| Self::new(registry, projector))
    }

    async fn next_action_result(
        registry: GerbilSchemeTypeRegistry,
        projector: Arc<dyn AgentHarnessGerbilLoopContinuationProjector>,
        input: GraphLoopContinuationInput,
    ) -> Result<GraphLoopNextAction, AgentHarnessGerbilLoopContinuationError> {
        let typed_value = projector.project_continuation(input).await?;
        project_gerbil_loop_graph_continuation_native_action(&registry, &typed_value)
            .map(|(_receipt, next_action)| next_action)
            .map_err(AgentHarnessGerbilLoopContinuationError::from)
    }
}

impl GraphLoopContinuationPlanner for AgentHarnessGerbilLoopContinuationPlanner {
    fn next_action(&self, input: GraphLoopContinuationInput) -> RuntimeFuture<GraphLoopNextAction> {
        let registry = self.registry.clone();
        let projector = Arc::clone(&self.projector);
        Box::pin(async move {
            Self::next_action_result(registry, projector, input)
                .await
                .unwrap_or_else(|error| GraphLoopNextAction::EscalateToHuman {
                    reason: error.to_string(),
                })
        })
    }
}
