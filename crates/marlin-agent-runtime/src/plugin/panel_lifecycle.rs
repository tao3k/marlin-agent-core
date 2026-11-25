// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Stateful, Rust-owned lifecycle, grant, and effect accounting for signed
//! unified UI panel releases (`marlin.unified.release.v1`).

use std::collections::{BTreeMap, BTreeSet};

use ring::{
    digest,
    rand::{SecureRandom, SystemRandom},
};

use super::{
    BundleDigest, CapabilityId, ContributionId, PanelContribution, PanelRuntimeKind, PluginDenial,
    PluginId, ReleaseTrust, SignedPanelRelease, SlotId, VerifiedPanelRelease,
};

const MAX_INSTALLED_PANEL_RELEASES: usize = 512;
const MAX_PANEL_EFFECTS_PER_ACTIVATION: usize = 4096;

/// Domain identifier for a project scoped to unified UI panel grants and activations.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct PanelProjectId(String);

impl PanelProjectId {
    /// Wrap a project identifier string in the panel domain newtype.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Borrow the underlying project identifier string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for PanelProjectId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for PanelProjectId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

/// Domain identifier for one active or closed panel activation generation.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct PanelActivationId(String);

impl PanelActivationId {
    /// Wrap an activation identifier string in the panel domain newtype.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Borrow the underlying activation identifier string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for PanelActivationId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for PanelActivationId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

/// Active generation summary for one installed UI panel plugin.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PanelGenerationStatus {
    pub enabled: bool,
    pub active_generation: u64,
}

/// Typed admission request for one scoped UI panel action.
#[derive(Clone, Copy, Debug)]
pub struct ScopedPanelActionRequest<'a> {
    pub project_id: &'a PanelProjectId,
    pub plugin_id: &'a PluginId,
    pub contribution_id: &'a ContributionId,
    pub generation: u64,
    pub action_name: &'a str,
    pub args_json: &'a [u8],
    pub idempotency_key: &'a str,
}

/// One recorded scoped panel action admitted under a live activation generation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PanelActionRecord {
    pub ordinal: u64,
    pub action_name: String,
    pub idempotency_key: String,
    pub args_digest: [u8; 32],
    pub receipt_id: String,
    pub receipt_digest: [u8; 32],
}

/// Authoritative evidence for one panel activation generation (live or closed).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PanelActivationRecord {
    pub activation_id: String,
    pub project_id: String,
    pub plugin_id: PluginId,
    pub contribution_id: ContributionId,
    pub bundle_digest: BundleDigest,
    pub generation: u64,
    pub runtime: PanelRuntimeKind,
    pub slot_id: SlotId,
    pub declared_actions: BTreeSet<String>,
    pub read_scopes: BTreeSet<String>,
    pub settings_digest: [u8; 32],
    pub closed: bool,
    pub actions: Vec<PanelActionRecord>,
}

/// Receipt emitted when a panel activation generation is disabled and closed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PanelDisableReceipt {
    pub activation_id: String,
    pub closed_generation: u64,
    pub recorded_actions: usize,
    pub closure_digest: [u8; 32],
}

/// In-process Rust owner for unified UI panel releases, project grants,
/// monotonic activation generations, scoped action admission, and closure evidence.
pub struct PanelPluginHost {
    host_epoch: [u8; 16],
    trust: ReleaseTrust,
    releases: BTreeMap<PluginId, VerifiedPanelRelease>,
    grants: BTreeMap<(PanelProjectId, PluginId), BTreeSet<CapabilityId>>,
    active_by_key: BTreeMap<(PanelProjectId, PluginId, ContributionId), PanelActivationId>,
    activations: BTreeMap<PanelActivationId, PanelActivationRecord>,
    next_generation: u64,
    next_receipt_seq: u64,
}

impl PanelPluginHost {
    /// Construct a new in-process panel plugin host bound to deployment trust roots.
    pub fn new(trust: ReleaseTrust) -> Self {
        let mut host_epoch = [0u8; 16];
        let _ = SystemRandom::new().fill(&mut host_epoch);
        Self {
            host_epoch,
            trust,
            releases: BTreeMap::new(),
            grants: BTreeMap::new(),
            active_by_key: BTreeMap::new(),
            activations: BTreeMap::new(),
            next_generation: 0,
            next_receipt_seq: 0,
        }
    }

    /// Cryptographically verify and install a signed unified panel release.
    pub fn install(
        &mut self,
        release: SignedPanelRelease<'_>,
    ) -> Result<&VerifiedPanelRelease, PluginDenial> {
        let verified = self.trust.verify_panel(release)?;
        let id = verified.id().clone();
        if !self.releases.contains_key(&id) && self.releases.len() >= MAX_INSTALLED_PANEL_RELEASES {
            return Err(PluginDenial::CapacityExceeded);
        }
        self.releases.insert(id.clone(), verified);
        self.releases
            .get(&id)
            .ok_or(PluginDenial::InvalidDeclaration)
    }

    /// Iterate over all verified panel releases installed on this host.
    pub fn installed_releases(&self) -> impl Iterator<Item = &VerifiedPanelRelease> {
        self.releases.values()
    }

    /// Look up one verified panel release by plugin identifier.
    pub fn get_release(&self, id: &PluginId) -> Option<&VerifiedPanelRelease> {
        self.releases.get(id)
    }

    /// Record operator-approved capability grants for `(project_id, plugin_id)`.
    pub fn set_project_grants(
        &mut self,
        project_id: PanelProjectId,
        plugin_id: PluginId,
        granted: impl IntoIterator<Item = CapabilityId>,
    ) -> Result<Vec<CapabilityId>, PluginDenial> {
        if !self.releases.contains_key(&plugin_id) {
            return Err(PluginDenial::MissingRelease);
        }
        let set: BTreeSet<CapabilityId> = granted.into_iter().collect();
        let list: Vec<CapabilityId> = set.iter().cloned().collect();
        self.grants.insert((project_id, plugin_id), set);
        Ok(list)
    }

    /// Return granted capability names for `(project_id, plugin_id)`.
    pub fn granted_capabilities(
        &self,
        project_id: &PanelProjectId,
        plugin_id: &PluginId,
    ) -> Vec<String> {
        self.grants
            .get(&(project_id.clone(), plugin_id.clone()))
            .map(|set| set.iter().map(|c| c.as_str().to_owned()).collect())
            .unwrap_or_default()
    }

    /// Activate an installed panel contribution for `project_id`, closing any prior
    /// active generation atomically and returning the new authoritative record.
    pub fn enable_panel(
        &mut self,
        project_id: PanelProjectId,
        plugin_id: &PluginId,
        contribution_id: &ContributionId,
        settings_toml: &[u8],
    ) -> Result<PanelActivationRecord, PluginDenial> {
        let verified = self
            .releases
            .get(plugin_id)
            .ok_or(PluginDenial::MissingRelease)?;
        let contrib: &PanelContribution = verified
            .contribution(contribution_id)
            .ok_or(PluginDenial::UnregisteredWorkflow)?;

        let key = (
            project_id.clone(),
            plugin_id.clone(),
            contribution_id.clone(),
        );
        if let Some(prev_id) = self.active_by_key.get(&key).cloned()
            && let Some(prev) = self.activations.get_mut(&prev_id)
        {
            prev.closed = true;
        }

        self.next_generation = self.next_generation.saturating_add(1);
        let active_gen = self.next_generation;
        let activation_id = PanelActivationId::new(format!(
            "act-{}-{}-{}-{active_gen}",
            hex_prefix(&self.host_epoch),
            plugin_id.as_str(),
            contribution_id.as_str()
        ));
        let settings_digest = sha256_bytes(settings_toml);
        let record = PanelActivationRecord {
            activation_id: activation_id.as_str().to_owned(),
            project_id: project_id.as_str().to_owned(),
            plugin_id: plugin_id.clone(),
            contribution_id: contribution_id.clone(),
            bundle_digest: verified.digest(),
            generation: active_gen,
            runtime: contrib.panel.runtime,
            slot_id: contrib.panel.slot_id.clone(),
            declared_actions: contrib.panel.declared_actions.iter().cloned().collect(),
            read_scopes: contrib.panel.read_scopes.iter().cloned().collect(),
            settings_digest,
            closed: false,
            actions: Vec::new(),
        };
        self.active_by_key.insert(key, activation_id.clone());
        self.activations.insert(activation_id, record.clone());
        Ok(record)
    }

    /// Atomically disable and close an activation generation so subsequent
    /// actions on that generation are permanently denied with `StaleActivation`.
    pub fn disable_panel(
        &mut self,
        activation_id: &PanelActivationId,
    ) -> Result<PanelDisableReceipt, PluginDenial> {
        let rec = self
            .activations
            .get_mut(activation_id)
            .ok_or(PluginDenial::StaleActivation)?;
        rec.closed = true;
        let key = (
            PanelProjectId::new(rec.project_id.clone()),
            rec.plugin_id.clone(),
            rec.contribution_id.clone(),
        );
        if self
            .active_by_key
            .get(&key)
            .is_some_and(|id| id == activation_id)
        {
            self.active_by_key.remove(&key);
        }
        let closure_digest = sha256_bytes(
            format!(
                "{}:{}:{}",
                rec.activation_id,
                rec.generation,
                rec.actions.len()
            )
            .as_bytes(),
        );
        Ok(PanelDisableReceipt {
            activation_id: rec.activation_id.clone(),
            closed_generation: rec.generation,
            recorded_actions: rec.actions.len(),
            closure_digest,
        })
    }

    /// Look up active `read_scopes` for a live `(project, plugin, contribution, generation)`.
    pub fn active_read_scopes(
        &self,
        project_id: &PanelProjectId,
        plugin_id: &PluginId,
        contribution_id: &ContributionId,
        generation: u64,
    ) -> Result<BTreeSet<String>, PluginDenial> {
        let rec = self.resolve_live(project_id, plugin_id, contribution_id, generation)?;
        Ok(rec.read_scopes.clone())
    }

    /// Admit and record a scoped panel action, enforcing live non-closed generation,
    /// exact `declared_actions` membership, and bounded effect accounting.
    pub fn admit_scoped_action(
        &mut self,
        request: ScopedPanelActionRequest<'_>,
    ) -> Result<PanelActionRecord, PluginDenial> {
        let act_id = self.active_activation_id(
            request.project_id,
            request.plugin_id,
            request.contribution_id,
        )?;
        let next_seq = self.next_receipt_seq.saturating_add(1);
        let rec = self.live_activation_mut(&act_id, request.generation)?;
        if !rec.declared_actions.contains(request.action_name) {
            return Err(PluginDenial::ScopeMismatch);
        }
        if let Some(existing) = find_idempotent_action(&rec.actions, request.idempotency_key) {
            return Ok(existing);
        }
        if rec.actions.len() >= MAX_PANEL_EFFECTS_PER_ACTIVATION {
            return Err(PluginDenial::CapacityExceeded);
        }
        let entry = build_panel_action_record(
            next_seq,
            (rec.actions.len() + 1) as u64,
            request.action_name,
            request.args_json,
            request.idempotency_key,
        );
        rec.actions.push(entry.clone());
        self.next_receipt_seq = next_seq;
        Ok(entry)
    }

    /// Return the full audit/evidence record for a live or closed activation generation.
    pub fn evidence(&self, activation_id: &PanelActivationId) -> Option<&PanelActivationRecord> {
        self.activations.get(activation_id)
    }

    /// Summarize whether any non-closed activation generation exists for `plugin_id`.
    pub fn active_generation_for(&self, plugin_id: &PluginId) -> PanelGenerationStatus {
        let mut active_generation = 0u64;
        let mut enabled = false;
        for rec in self.activations.values() {
            if &rec.plugin_id == plugin_id && !rec.closed && rec.generation > active_generation {
                active_generation = rec.generation;
                enabled = true;
            }
        }
        PanelGenerationStatus {
            enabled,
            active_generation,
        }
    }

    fn active_activation_id(
        &self,
        project_id: &PanelProjectId,
        plugin_id: &PluginId,
        contribution_id: &ContributionId,
    ) -> Result<PanelActivationId, PluginDenial> {
        let key = (
            project_id.clone(),
            plugin_id.clone(),
            contribution_id.clone(),
        );
        self.active_by_key
            .get(&key)
            .cloned()
            .ok_or(PluginDenial::StaleActivation)
    }

    fn live_activation_mut(
        &mut self,
        activation_id: &PanelActivationId,
        generation: u64,
    ) -> Result<&mut PanelActivationRecord, PluginDenial> {
        let rec = self
            .activations
            .get_mut(activation_id)
            .ok_or(PluginDenial::StaleActivation)?;
        if rec.closed || rec.generation != generation {
            return Err(PluginDenial::StaleActivation);
        }
        Ok(rec)
    }

    fn resolve_live(
        &self,
        project_id: &PanelProjectId,
        plugin_id: &PluginId,
        contribution_id: &ContributionId,
        generation: u64,
    ) -> Result<&PanelActivationRecord, PluginDenial> {
        let act_id = self.active_activation_id(project_id, plugin_id, contribution_id)?;
        let rec = self
            .activations
            .get(&act_id)
            .ok_or(PluginDenial::StaleActivation)?;
        if rec.closed || rec.generation != generation {
            return Err(PluginDenial::StaleActivation);
        }
        Ok(rec)
    }
}

fn find_idempotent_action(
    actions: &[PanelActionRecord],
    idempotency_key: &str,
) -> Option<PanelActionRecord> {
    if idempotency_key.is_empty() {
        return None;
    }
    actions
        .iter()
        .find(|action| action.idempotency_key == idempotency_key)
        .cloned()
}

fn build_panel_action_record(
    receipt_seq: u64,
    ordinal: u64,
    action_name: &str,
    args_json: &[u8],
    idempotency_key: &str,
) -> PanelActionRecord {
    let args_digest = sha256_bytes(args_json);
    let receipt_id = format!("rcpt-panel-action-{receipt_seq}");
    let mut mat = Vec::with_capacity(64 + action_name.len());
    mat.extend_from_slice(receipt_id.as_bytes());
    mat.extend_from_slice(&args_digest);
    mat.extend_from_slice(action_name.as_bytes());
    let receipt_digest = sha256_bytes(&mat);
    PanelActionRecord {
        ordinal,
        action_name: action_name.to_owned(),
        idempotency_key: idempotency_key.to_owned(),
        args_digest,
        receipt_id,
        receipt_digest,
    }
}

fn sha256_bytes(input: &[u8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    out.copy_from_slice(digest::digest(&digest::SHA256, input).as_ref());
    out
}

fn hex_prefix(bytes: &[u8; 16]) -> String {
    let mut s = String::with_capacity(8);
    for b in &bytes[..4] {
        use std::fmt::Write;
        let _ = write!(&mut s, "{b:02x}");
    }
    s
}
