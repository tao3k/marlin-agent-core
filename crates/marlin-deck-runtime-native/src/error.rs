//! Error types for native Deck runtime route projection.

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

use marlin_agent_runtime::ModelRouteSelectionProjectionError;
use marlin_gerbil_scheme::GerbilDeckRuntimeNativeAbiError;

/// Error raised while resolving a model route through the native Deck runtime.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DeckRuntimeNativeRouteError {
    /// The typed native ABI call failed before a route decision could be projected.
    NativeAbi(GerbilDeckRuntimeNativeAbiError),
    /// The native selector returned an index outside the native policy order.
    UnknownNativePolicyIndex {
        native_policy_index: usize,
        policies_len: usize,
    },
    /// The selected original policy could not be projected into a runtime decision.
    Projection(ModelRouteSelectionProjectionError),
}

impl Display for DeckRuntimeNativeRouteError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::NativeAbi(source) => {
                write!(formatter, "native Deck runtime ABI failed: {source}")
            }
            Self::UnknownNativePolicyIndex {
                native_policy_index,
                policies_len,
            } => write!(
                formatter,
                "native Deck runtime policy index {native_policy_index} is outside {policies_len} policies"
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

impl Error for DeckRuntimeNativeRouteError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::NativeAbi(source) => Some(source),
            Self::Projection(source) => Some(source),
            Self::UnknownNativePolicyIndex { .. } => None,
        }
    }
}

impl From<GerbilDeckRuntimeNativeAbiError> for DeckRuntimeNativeRouteError {
    fn from(source: GerbilDeckRuntimeNativeAbiError) -> Self {
        Self::NativeAbi(source)
    }
}

impl From<ModelRouteSelectionProjectionError> for DeckRuntimeNativeRouteError {
    fn from(source: ModelRouteSelectionProjectionError) -> Self {
        Self::Projection(source)
    }
}
