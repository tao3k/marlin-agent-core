//! Error type shared by Gerbil dependency bootstrap modules.

use std::fmt;

/// Failure returned by the Gerbil dependency bootstrap CLI.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilDepsError {
    message: String,
}

impl GerbilDepsError {
    pub(super) fn message(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for GerbilDepsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for GerbilDepsError {}
