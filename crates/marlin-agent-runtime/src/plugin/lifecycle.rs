// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Project grants and effect accounting across activation generations.

use std::{collections::BTreeMap, sync::Arc};

use marlin_agent_protocol::RuntimeWorkspaceProjectId;
use parking_lot::Mutex;
use ring::rand::{SecureRandom, SystemRandom};
use serde::{Deserialize, Serialize};

use super::{
    BundleDigest, CapabilityId, ContributionId, FileCapability, PluginDenial, PluginId,
    PluginVersion, PolicyModuleId, ReleaseTrust, SchemaId, SignedRelease, VerifiedRelease,
    WorkflowPreset,
};

pub(super) const MAX_EFFECTS_PER_ACTIVATION: usize = 4096;
const MAX_RETAINED_GENERATIONS: usize = 4096;
const MAX_INSTALLED_RELEASES: usize = 1024;

/// One operator-issued capability set for a project and signed release.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectGrant {
    pub project: RuntimeWorkspaceProjectId,
    pub bundle: BundleDigest,
    pub revision: u64,
    pub capabilities: BTreeMap<CapabilityId, FileCapability>,
}

/// A host-provisioned workflow. The validator returns the digest of the
/// effective settings under the registered policy revision.
pub struct WorkflowRegistration<'owner> {
    pub policy_module: PolicyModuleId,
    pub input_schema: SchemaId,
    pub settings_schema: SchemaId,
    pub required_capabilities: BTreeMap<CapabilityId, FileCapability>,
    pub policy_revision: [u8; 32],
    pub validate_settings: Box<WorkflowSettingsValidator<'owner>>,
}

/// A trusted policy adapter that validates settings and returns their digest.
pub type WorkflowSettingsValidator<'owner> =
    dyn Fn(&WorkflowPreset, &toml::Table) -> Result<[u8; 32], PluginDenial> + 'owner;

/// Selects an installed workflow for a project under its current grant.
pub struct EnableWorkflow<'a> {
    pub plugin: &'a PluginId,
    pub version: &'a PluginVersion,
    pub workflow: &'a ContributionId,
    pub project: &'a RuntimeWorkspaceProjectId,
    pub project_settings: &'a toml::Table,
}

/// Identity scoped to the host instance, project and workflow generation.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ActivationId {
    host_epoch: [u8; 16],
    project: RuntimeWorkspaceProjectId,
    plugin: PluginId,
    workflow: ContributionId,
    generation: u64,
}

impl ActivationId {
    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn workflow(&self) -> &ContributionId {
        &self.workflow
    }

    pub fn plugin(&self) -> &PluginId {
        &self.plugin
    }

    pub fn project(&self) -> &RuntimeWorkspaceProjectId {
        &self.project
    }
}

/// Admission evidence for the settings, policy and grant of one generation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Activation {
    pub id: ActivationId,
    pub bundle: BundleDigest,
    pub grant_revision: u64,
    pub policy_revision: [u8; 32],
    pub settings_digest: [u8; 32],
}

/// One effect admitted under an activation.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct EffectId(u64);

impl EffectId {
    pub fn ordinal(self) -> u64 {
        self.0
    }
}

/// Outcome reported by the host effect owner.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub enum EffectOutcome {
    Committed { output_digest: [u8; 32] },
    Denied,
    Failed,
    Indeterminate,
}

/// Closure observation with the pending and settled effects of a generation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisableReceipt {
    pub activation: Activation,
    pub pending: usize,
    pub outcomes: BTreeMap<EffectId, EffectOutcome>,
}

/// In-process owner of trusted releases, grants and workflow activations.
pub struct PluginHost<'owner> {
    epoch: [u8; 16],
    trust: ReleaseTrust,
    registrations: BTreeMap<PolicyModuleId, WorkflowRegistration<'owner>>,
    releases: BTreeMap<(PluginId, PluginVersion), VerifiedRelease>,
    grants: BTreeMap<(RuntimeWorkspaceProjectId, BundleDigest), ProjectGrant>,
    active: BTreeMap<WorkflowKey, ActivationId>,
    generations: BTreeMap<ActivationId, Arc<Mutex<Generation>>>,
    next_generation: u64,
}

type WorkflowKey = (RuntimeWorkspaceProjectId, PluginId, ContributionId);

struct Generation {
    activation: Activation,
    policy_module: PolicyModuleId,
    grant: ProjectGrant,
    required: BTreeMap<CapabilityId, FileCapability>,
    closed: bool,
    effects: BTreeMap<EffectId, AdmittedEffect>,
}

struct AdmittedEffect {
    capability: CapabilityId,
    outcome: Option<EffectOutcome>,
}

impl<'owner> PluginHost<'owner> {
    /// Provision trusted workflow validators before any release is installed.
    pub fn new(
        trust: ReleaseTrust,
        registrations: impl IntoIterator<Item = WorkflowRegistration<'owner>>,
    ) -> Result<Self, PluginDenial> {
        let mut epoch = [0; 16];
        SystemRandom::new()
            .fill(&mut epoch)
            .map_err(|_| PluginDenial::HostInitialization)?;
        let mut host = Self {
            epoch,
            trust,
            registrations: BTreeMap::new(),
            releases: BTreeMap::new(),
            grants: BTreeMap::new(),
            active: BTreeMap::new(),
            generations: BTreeMap::new(),
            next_generation: 0,
        };
        for registration in registrations {
            let id = registration.policy_module.clone();
            if host.registrations.insert(id, registration).is_some() {
                return Err(PluginDenial::ConflictingRegistration);
            }
        }
        Ok(host)
    }

    pub fn install(&mut self, signed: SignedRelease<'_>) -> Result<BundleDigest, PluginDenial> {
        let release = self.trust.verify(signed)?;
        let key = (release.id().clone(), release.version().clone());
        let digest = release.digest();
        let has_capacity = self.releases.len() < MAX_INSTALLED_RELEASES;
        match self.releases.entry(key) {
            std::collections::btree_map::Entry::Occupied(entry)
                if entry.get().digest() != digest =>
            {
                Err(PluginDenial::ConflictingRelease)
            }
            std::collections::btree_map::Entry::Occupied(_) => Ok(digest),
            std::collections::btree_map::Entry::Vacant(_) if !has_capacity => {
                Err(PluginDenial::CapacityExceeded)
            }
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(release);
                Ok(digest)
            }
        }
    }

    /// Called by the trusted project operator. A higher revision closes every
    /// active generation on the previous grant before the replacement is used.
    /// An empty capability map revokes the grant and retains its revision.
    pub fn replace_project_grant(&mut self, grant: ProjectGrant) -> Result<(), PluginDenial> {
        if grant.project.as_str().is_empty() || grant.revision == 0 {
            return Err(PluginDenial::ScopeMismatch);
        }
        let key = (grant.project.clone(), grant.bundle);
        if let Some(previous) = self.grants.get(&key) {
            if previous == &grant {
                return Ok(());
            }
            if grant.revision <= previous.revision {
                return Err(PluginDenial::ScopeMismatch);
            }
        } else if self.grants.len() >= MAX_RETAINED_GENERATIONS {
            return Err(PluginDenial::CapacityExceeded);
        }
        for generation in self
            .active
            .values()
            .filter(|id| id.project == grant.project)
        {
            let mut state = self.generation(generation)?.lock();
            if state.activation.bundle == grant.bundle {
                state.closed = true;
            }
        }
        self.grants.insert(key, grant);
        Ok(())
    }

    pub fn enable(&mut self, request: EnableWorkflow<'_>) -> Result<Activation, PluginDenial> {
        let (grant, registration, settings_digest) = self.validate_enable(&request)?;
        let key = (
            request.project.clone(),
            request.plugin.clone(),
            request.workflow.clone(),
        );
        if let Some(existing) = self.active.get(&key) {
            let state = self.generation(existing)?.lock();
            if !state.closed
                && &state.grant == grant
                && state.activation.policy_revision == registration.policy_revision
                && state.activation.settings_digest == settings_digest
            {
                return Ok(state.activation.clone());
            }
            if !state.closed
                || state
                    .effects
                    .values()
                    .any(|effect| effect.outcome.is_none())
            {
                return Err(PluginDenial::ActivationInUse);
            }
        }
        let policy_module = registration.policy_module.clone();
        let policy_revision = registration.policy_revision;
        let required = registration.required_capabilities.clone();
        self.publish_activation(
            key,
            grant.clone(),
            policy_module,
            policy_revision,
            settings_digest,
            required,
        )
    }

    fn validate_enable(
        &self,
        request: &EnableWorkflow<'_>,
    ) -> Result<(&ProjectGrant, &WorkflowRegistration<'owner>, [u8; 32]), PluginDenial> {
        let EnableWorkflow {
            plugin,
            version,
            workflow,
            project,
            project_settings,
        } = *request;
        let release = self
            .releases
            .get(&(plugin.clone(), version.clone()))
            .ok_or(PluginDenial::MissingRelease)?;
        let grant = self
            .grants
            .get(&(project.clone(), release.digest()))
            .ok_or(PluginDenial::ScopeMismatch)?;
        let preset = release
            .workflow(workflow)
            .ok_or(PluginDenial::UnregisteredWorkflow)?;
        let registration = self
            .registrations
            .get(&preset.policy_module)
            .ok_or(PluginDenial::UnregisteredWorkflow)?;
        if preset.input_schema != registration.input_schema
            || preset.settings_schema != registration.settings_schema
            || preset.requires.len() != registration.required_capabilities.len()
            || registration
                .required_capabilities
                .iter()
                .any(|(id, effect)| {
                    !preset.requires.contains(id) || release.capability(id) != Some(effect)
                })
        {
            return Err(PluginDenial::UnregisteredWorkflow);
        }
        for (id, effect) in &registration.required_capabilities {
            if grant.capabilities.get(id) != Some(effect) {
                return Err(PluginDenial::MissingCapability(id.clone()));
            }
        }
        Ok((
            grant,
            registration,
            (registration.validate_settings)(preset, project_settings)?,
        ))
    }

    fn publish_activation(
        &mut self,
        key: WorkflowKey,
        grant: ProjectGrant,
        policy_module: PolicyModuleId,
        policy_revision: [u8; 32],
        settings_digest: [u8; 32],
        required: BTreeMap<CapabilityId, FileCapability>,
    ) -> Result<Activation, PluginDenial> {
        if self.generations.len() >= MAX_RETAINED_GENERATIONS {
            return Err(PluginDenial::CapacityExceeded);
        }
        self.next_generation = self
            .next_generation
            .checked_add(1)
            .ok_or(PluginDenial::CapacityExceeded)?;
        let activation = Activation {
            id: ActivationId {
                host_epoch: self.epoch,
                project: key.0.clone(),
                plugin: key.1.clone(),
                workflow: key.2.clone(),
                generation: self.next_generation,
            },
            bundle: grant.bundle,
            grant_revision: grant.revision,
            policy_revision,
            settings_digest,
        };
        self.active.insert(key, activation.id.clone());
        self.generations.insert(
            activation.id.clone(),
            Arc::new(Mutex::new(Generation {
                activation: activation.clone(),
                policy_module,
                grant,
                required,
                closed: false,
                effects: BTreeMap::new(),
            })),
        );
        Ok(activation)
    }

    pub fn admit_effect(
        &self,
        project: &RuntimeWorkspaceProjectId,
        activation: &ActivationId,
        capability_id: &CapabilityId,
        effect: &FileCapability,
    ) -> Result<EffectAdmission, PluginDenial> {
        self.admit_registered_effect(project, activation, capability_id, |registered| {
            registered == effect
        })
        .map(|(admission, _)| admission)
    }

    pub(super) fn admit_registered_effect(
        &self,
        project: &RuntimeWorkspaceProjectId,
        activation: &ActivationId,
        capability_id: &CapabilityId,
        accepts: impl FnOnce(&FileCapability) -> bool,
    ) -> Result<(EffectAdmission, FileCapability), PluginDenial> {
        self.effect_scope(project, activation)?
            .admit(capability_id, accepts)
    }

    pub(super) fn effect_scope(
        &self,
        project: &RuntimeWorkspaceProjectId,
        activation: &ActivationId,
    ) -> Result<EffectScope, PluginDenial> {
        if activation.project() != project {
            return Err(PluginDenial::ScopeMismatch);
        }
        Ok(EffectScope(Arc::clone(self.generation(activation)?)))
    }

    /// Read the admission of a live generation for a trusted executor's policy.
    pub fn active_workflow(
        &self,
        id: &ActivationId,
        policy_module: &PolicyModuleId,
    ) -> Result<Activation, PluginDenial> {
        let state = self.generation(id)?.lock();
        if state.closed {
            return Err(PluginDenial::StaleActivation);
        }
        if &state.policy_module != policy_module {
            return Err(PluginDenial::UnregisteredWorkflow);
        }
        Ok(state.activation.clone())
    }

    /// Close admissions immediately. Call again to read the final receipt
    /// once all previously admitted effects have reported their outcomes.
    pub fn disable(&self, activation: &ActivationId) -> Result<DisableReceipt, PluginDenial> {
        let mut state = self.generation(activation)?.lock();
        state.closed = true;
        let outcomes: BTreeMap<_, _> = state
            .effects
            .iter()
            .filter_map(|(id, effect)| effect.outcome.map(|value| (*id, value)))
            .collect();
        Ok(DisableReceipt {
            activation: state.activation.clone(),
            pending: state.effects.len() - outcomes.len(),
            outcomes,
        })
    }

    /// Export a settled, closed generation with retained bytes for each committed effect.
    pub fn export_closed_effects(
        &self,
        activation: &ActivationId,
        mut retained: BTreeMap<EffectId, Vec<u8>>,
    ) -> Result<super::ClosedWorkflowEvidence, super::WorkflowEvidenceError> {
        use super::{ClosedWorkflowEvidence, WorkflowEffectEvidence, WorkflowEvidenceError};
        let state = self
            .generation(activation)
            .map_err(|_| WorkflowEvidenceError::Incomplete)?
            .lock();
        if !state.closed {
            return Err(WorkflowEvidenceError::Incomplete);
        }
        let effects = state
            .effects
            .iter()
            .map(|(&id, effect)| {
                let outcome = effect.outcome.ok_or(WorkflowEvidenceError::Incomplete)?;
                Ok(WorkflowEffectEvidence {
                    id,
                    capability: effect.capability.clone(),
                    outcome,
                    bytes: retained.remove(&id),
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        if !retained.is_empty() {
            return Err(WorkflowEvidenceError::Inconsistent);
        }
        let evidence =
            ClosedWorkflowEvidence::new(state.activation.clone(), state.grant.clone(), effects);
        evidence.check_effects()?;
        Ok(evidence)
    }

    fn generation(&self, id: &ActivationId) -> Result<&Arc<Mutex<Generation>>, PluginDenial> {
        self.generations
            .get(id)
            .ok_or(PluginDenial::StaleActivation)
    }
}

impl Drop for PluginHost<'_> {
    fn drop(&mut self) {
        for generation in self.generations.values() {
            generation.lock().closed = true;
        }
    }
}

// Grant replacement closes the existing generation under this same lock.
// Holding this scope never bypasses the live closure check.
pub(super) struct EffectScope(Arc<Mutex<Generation>>);

impl EffectScope {
    pub(super) fn admit(
        &self,
        capability_id: &CapabilityId,
        accepts: impl FnOnce(&FileCapability) -> bool,
    ) -> Result<(EffectAdmission, FileCapability), PluginDenial> {
        let mut state = self.0.lock();
        if state.closed {
            return Err(PluginDenial::StaleActivation);
        }
        let effect = state
            .required
            .get(capability_id)
            .filter(|effect| accepts(effect))
            .ok_or_else(|| PluginDenial::MissingCapability(capability_id.clone()))?
            .clone();
        if state.effects.len() >= MAX_EFFECTS_PER_ACTIVATION {
            return Err(PluginDenial::CapacityExceeded);
        }
        let id = EffectId(state.effects.len() as u64 + 1);
        state.effects.insert(
            id,
            AdmittedEffect {
                capability: capability_id.clone(),
                outcome: None,
            },
        );
        drop(state);
        Ok((
            EffectAdmission {
                generation: Arc::clone(&self.0),
                id,
                finished: false,
            },
            effect,
        ))
    }
}

/// Accounts for an admitted effect until the effect owner reports its outcome.
#[must_use]
pub struct EffectAdmission {
    generation: Arc<Mutex<Generation>>,
    id: EffectId,
    finished: bool,
}

impl EffectAdmission {
    pub fn id(&self) -> EffectId {
        self.id
    }

    pub fn finish(mut self, outcome: EffectOutcome) {
        self.set_outcome(outcome);
        self.finished = true;
    }

    fn set_outcome(&self, outcome: EffectOutcome) {
        if let Some(effect) = self.generation.lock().effects.get_mut(&self.id) {
            effect.outcome = Some(outcome);
        }
    }
}

impl Drop for EffectAdmission {
    fn drop(&mut self) {
        if !self.finished {
            self.set_outcome(EffectOutcome::Indeterminate);
        }
    }
}
