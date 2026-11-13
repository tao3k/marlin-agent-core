// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Rust-owned admission for signed declarative workflow bundles.

mod denial;
mod evidence;
mod lifecycle;
mod release;
#[cfg(unix)]
mod workspace;

pub use denial::PluginDenial;
pub use evidence::{ClosedWorkflowEvidence, WorkflowEffectEvidence, WorkflowEvidenceError};
pub use lifecycle::{
    Activation, ActivationId, DisableReceipt, EffectAdmission, EffectId, EffectOutcome,
    EnableWorkflow, PluginHost, ProjectGrant, WorkflowRegistration, WorkflowSettingsValidator,
};
pub use release::{
    BundleDigest, CapabilityId, ContributionId, FileCapability, PluginId, PluginVersion,
    PolicyModuleId, PublisherId, ReleaseAttestation, ReleaseTrust, SchemaId, SignedRelease,
    VerifiedRelease, WorkflowPreset,
};

#[cfg(unix)]
pub use workspace::{WorkspaceCreate, WorkspaceEffect, WorkspaceEffectError, WorkspaceFileBroker};
