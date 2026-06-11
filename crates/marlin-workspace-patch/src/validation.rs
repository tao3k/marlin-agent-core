//! Validation report model for workspace patch results.

use serde::{Deserialize, Serialize};

/// Validation result produced before a patch is accepted.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceValidationReport {
    pub accepted: bool,
    pub diagnostics: Vec<ValidationDiagnostic>,
}

impl WorkspaceValidationReport {
    pub fn accepted() -> Self {
        Self {
            accepted: true,
            diagnostics: Vec::new(),
        }
    }
}

/// Single validation diagnostic.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ValidationDiagnostic {
    pub severity: ValidationSeverity,
    pub message: String,
}

/// Severity attached to a validation diagnostic.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ValidationSeverity {
    Info,
    Warning,
    Error,
}
