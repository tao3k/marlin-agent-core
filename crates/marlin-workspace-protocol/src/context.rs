//! Invocation context shared by workspace operations.

use serde::{Deserialize, Serialize};

/// Actor and tracing metadata attached to a workspace call.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceCtx {
    pub actor: String,
    pub run_id: Option<String>,
    pub trace_id: Option<String>,
}

impl WorkspaceCtx {
    pub fn new(actor: impl Into<String>) -> Self {
        Self {
            actor: actor.into(),
            run_id: None,
            trace_id: None,
        }
    }
}
