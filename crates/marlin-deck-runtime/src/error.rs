//! Error types for native Deck runtime route projection.

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

use marlin_agent_runtime::ModelRouteSelectionProjectionError;
use marlin_gerbil_scheme::GerbilDeckRouteAbiError;

/// Error raised while resolving a model route through the native Deck runtime.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DeckLinkedRouteError {
    /// The typed native ABI call failed before a route decision could be projected.
    CAbi(GerbilDeckRouteAbiError),
    /// The native selector returned an index outside the native policy order.
    UnknownCandidateIndex {
        candidate_index: usize,
        policies_len: usize,
    },
    /// The selected original policy could not be projected into a runtime decision.
    Projection(ModelRouteSelectionProjectionError),
}

impl Display for DeckLinkedRouteError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::CAbi(source) => {
                write!(formatter, "native Deck runtime ABI failed: {source}")
            }
            Self::UnknownCandidateIndex {
                candidate_index,
                policies_len,
            } => write!(
                formatter,
                "native Deck runtime policy index {candidate_index} is outside {policies_len} policies"
            ),
            Self::Projection(source) => {
                write!(
                    formatter,
                    "native Deck runtime route projection failed: {source}"
                )
            }
        }
    }
}

impl Error for DeckLinkedRouteError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::CAbi(source) => Some(source),
            Self::Projection(source) => Some(source),
            Self::UnknownCandidateIndex { .. } => None,
        }
    }
}

impl From<GerbilDeckRouteAbiError> for DeckLinkedRouteError {
    fn from(source: GerbilDeckRouteAbiError) -> Self {
        Self::CAbi(source)
    }
}

impl From<ModelRouteSelectionProjectionError> for DeckLinkedRouteError {
    fn from(source: ModelRouteSelectionProjectionError) -> Self {
        Self::Projection(source)
    }
}
