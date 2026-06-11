//! Stable `SessionId` and session kind contracts.

use serde::{Deserialize, Serialize};
use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

/// Stable identifier for one runtime session or delegated child session.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct SessionId(String);

impl SessionId {
    pub fn new(value: impl Into<String>) -> Self {
        Self::try_new(value).expect("session id must not be empty")
    }

    pub fn try_new(value: impl Into<String>) -> Result<Self, SessionIdError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(SessionIdError::Empty);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl Display for SessionId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl From<&str> for SessionId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for SessionId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

/// Validation errors for session identity construction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SessionIdError {
    Empty,
}

impl Display for SessionIdError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("session id must not be empty"),
        }
    }
}

impl Error for SessionIdError {}

/// Runtime work category owned by a session context.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SessionKind {
    Root,
    Provider,
    Tool,
    Hook,
    SubAgent,
}
