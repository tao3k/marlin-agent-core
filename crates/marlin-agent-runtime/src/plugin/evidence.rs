// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Offline verification of a closed workflow generation's file effects.

use ring::digest;
use serde::{Deserialize, Serialize};

use super::{Activation, CapabilityId, EffectId, EffectOutcome, ProjectGrant, VerifiedRelease};

const SCHEMA: &str = "marlin.workflow-effects.v0";
const MAX_ENCODED_BYTES: usize = 16 * 1024 * 1024;

/// Offline evidence was incomplete, inconsistent, unauthenticated, or malformed.
#[derive(Debug, Eq, PartialEq)]
pub enum WorkflowEvidenceError {
    Incomplete,
    Inconsistent,
    Unauthenticated,
    InvalidEncoding,
}

/// The registered capability, outcome, and retained bytes for one admitted effect.
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowEffectEvidence {
    pub id: EffectId,
    pub capability: CapabilityId,
    pub outcome: EffectOutcome,
    pub bytes: Option<Vec<u8>>,
}

/// Evidence exported by the Rust owner after admission closes and effects settle.
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ClosedWorkflowEvidence {
    schema: String,
    activation: Activation,
    grant: ProjectGrant,
    effects: Vec<WorkflowEffectEvidence>,
}

impl ClosedWorkflowEvidence {
    pub(super) fn new(
        activation: Activation,
        grant: ProjectGrant,
        effects: Vec<WorkflowEffectEvidence>,
    ) -> Self {
        Self {
            schema: SCHEMA.into(),
            activation,
            grant,
            effects,
        }
    }

    pub fn effects(&self) -> &[WorkflowEffectEvidence] {
        &self.effects
    }

    /// The activation whose grant authorized the recorded effects.
    pub fn activation(&self) -> &Activation {
        &self.activation
    }

    pub fn encode(&self) -> Result<Vec<u8>, WorkflowEvidenceError> {
        let mut bytes = Vec::new();
        serde_json::to_writer(BoundedEncoding(&mut bytes), self)
            .map_err(|_| WorkflowEvidenceError::InvalidEncoding)?;
        Ok(bytes)
    }

    /// Verify against an independently authenticated project digest and release.
    /// Exact bytes are hashed before parsing. There is no effect execution path.
    pub fn verify(
        bytes: &[u8],
        expected_digest: &[u8; 32],
        release: &VerifiedRelease,
    ) -> Result<Self, WorkflowEvidenceError> {
        if bytes.len() > MAX_ENCODED_BYTES {
            return Err(WorkflowEvidenceError::InvalidEncoding);
        }
        if hash(bytes) != *expected_digest {
            return Err(WorkflowEvidenceError::Unauthenticated);
        }
        let evidence: Self =
            serde_json::from_slice(bytes).map_err(|_| WorkflowEvidenceError::InvalidEncoding)?;
        evidence.validate_release(release)?;
        Ok(evidence)
    }

    /// Check nested effect evidence after its containing archive is authenticated.
    pub fn validate_release(&self, release: &VerifiedRelease) -> Result<(), WorkflowEvidenceError> {
        self.check_effects()?;
        let Some(workflow) = release.workflow(self.activation.id.workflow()) else {
            return Err(WorkflowEvidenceError::Inconsistent);
        };
        if self.activation.bundle != release.digest()
            || self.activation.id.plugin() != release.id()
            || self
                .grant
                .capabilities
                .iter()
                .any(|(id, capability)| release.capability(id) != Some(capability))
            || workflow.requires.iter().any(|capability| {
                self.grant.capabilities.get(capability) != release.capability(capability)
            })
            || self
                .effects
                .iter()
                .any(|effect| !workflow.requires.contains(&effect.capability))
        {
            return Err(WorkflowEvidenceError::Inconsistent);
        }
        Ok(())
    }

    pub(super) fn check_effects(&self) -> Result<(), WorkflowEvidenceError> {
        if self.schema != SCHEMA
            || self.activation.bundle != self.grant.bundle
            || self.activation.grant_revision != self.grant.revision
            || self.grant.revision == 0
            || self.activation.id.project() != &self.grant.project
            || self.grant.project.as_str().is_empty()
            || self.activation.id.generation() == 0
            || self.effects.len() > super::lifecycle::MAX_EFFECTS_PER_ACTIVATION
        {
            return Err(WorkflowEvidenceError::Inconsistent);
        }
        for (index, effect) in self.effects.iter().enumerate() {
            if effect.id.ordinal() != index as u64 + 1
                || !self.grant.capabilities.contains_key(&effect.capability)
            {
                return Err(WorkflowEvidenceError::Inconsistent);
            }
            match (effect.outcome, effect.bytes.as_deref()) {
                (EffectOutcome::Committed { output_digest }, Some(bytes))
                    if hash(bytes) == output_digest => {}
                (EffectOutcome::Committed { .. }, None) => {
                    return Err(WorkflowEvidenceError::Incomplete);
                }
                (EffectOutcome::Committed { .. }, Some(_)) | (_, Some(_)) => {
                    return Err(WorkflowEvidenceError::Inconsistent);
                }
                (_, None) => {}
            }
        }
        Ok(())
    }
}

pub(super) fn hash(bytes: &[u8]) -> [u8; 32] {
    let mut output = [0; 32];
    output.copy_from_slice(digest::digest(&digest::SHA256, bytes).as_ref());
    output
}

struct BoundedEncoding<'a>(&'a mut Vec<u8>);

impl std::io::Write for BoundedEncoding<'_> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > MAX_ENCODED_BYTES - self.0.len() {
            return Err(std::io::Error::other(
                "workflow evidence exceeds encoding limit",
            ));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
