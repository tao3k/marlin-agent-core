//! Typed model-route selection projection errors.

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

/// Error raised when an external typed selector chooses a route policy index
/// that cannot be projected into a runtime decision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModelRouteSelectionProjectionError {
    /// The selector returned an index outside the original route policy array.
    UnknownPolicyIndex { policy_index: usize },
    /// The selected rule no longer matches the request facts Rust is projecting.
    SelectedRuleDidNotMatch { policy_index: usize },
}

impl Display for ModelRouteSelectionProjectionError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownPolicyIndex { policy_index } => {
                write!(
                    formatter,
                    "model route policy index {policy_index} does not exist"
                )
            }
            Self::SelectedRuleDidNotMatch { policy_index } => write!(
                formatter,
                "model route policy index {policy_index} did not match the request"
            ),
        }
    }
}

impl Error for ModelRouteSelectionProjectionError {}
