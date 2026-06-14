//! Invocation context shared by workspace operations.

use serde::{Deserialize, Serialize};

use crate::WorkspaceProjectId;

/// Actor and tracing metadata attached to a workspace call.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceCtx {
    pub actor: String,
    #[serde(default)]
    pub project_id: Option<WorkspaceProjectId>,
    pub run_id: Option<String>,
    pub trace_id: Option<String>,
}

impl WorkspaceCtx {
    pub fn new(actor: impl Into<String>) -> Self {
        Self {
            actor: actor.into(),
            project_id: None,
            run_id: None,
            trace_id: None,
        }
    }

    pub fn with_project_id(mut self, project_id: impl Into<WorkspaceProjectId>) -> Self {
        self.project_id = Some(project_id.into());
        self
    }

    pub fn with_run_id(mut self, run_id: impl Into<String>) -> Self {
        self.run_id = Some(run_id.into());
        self
    }

    pub fn with_trace_id(mut self, trace_id: impl Into<String>) -> Self {
        self.trace_id = Some(trace_id.into());
        self
    }
}
