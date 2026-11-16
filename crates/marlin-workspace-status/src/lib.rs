// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Status report model for long-running workspace records.

mod report;
mod target;

pub use report::{
    ChecklistStatus, ContractStatus, DecisionTrace, EvidenceStatus, GoalState, GoalStatus,
    MetricTrace, ReleaseGateReceipt, ReleaseGateState, ReleaseGateStatus, ReleaseLandingReport,
    ReleaseStatus, ReleaseVisibilityStatus, SddStatus, WorkspaceStatusEvidence,
    WorkspaceStatusEvidenceKind, WorkspaceStatusReport,
};
pub use target::WorkspaceTarget;
