//! Runtime-owned model route admission.

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

use marlin_agent_protocol::{
    ModelRouteAdmissionRequest, ModelRouteAdmissionResponse, ModelRouteIntent,
};

use super::resolver::CompiledModelRouteResolver;

/// Error raised when model route admission cannot produce a decision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModelRouteAdmissionError {
    NoMatchingRoute { intent: ModelRouteIntent },
}

impl Display for ModelRouteAdmissionError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoMatchingRoute { intent } => write!(
                formatter,
                "model route admission found no deterministic route for task `{}`",
                intent.task_kind
            ),
        }
    }
}

impl Error for ModelRouteAdmissionError {}

/// Admits a model route request through the deterministic runtime resolver.
pub fn admit_model_route_with_resolver(
    resolver: &CompiledModelRouteResolver,
    request: ModelRouteAdmissionRequest,
) -> Result<ModelRouteAdmissionResponse, ModelRouteAdmissionError> {
    let intent = request.intent();
    let decision = resolver.resolve(&request.route_request).ok_or_else(|| {
        ModelRouteAdmissionError::NoMatchingRoute {
            intent: intent.clone(),
        }
    })?;

    Ok(ModelRouteAdmissionResponse::deterministic(intent, decision))
}
