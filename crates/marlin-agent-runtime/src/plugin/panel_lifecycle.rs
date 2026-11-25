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
    grants: BTreeMap<(String, PluginId), BTreeSet<CapabilityId>>,
    active_by_key: BTreeMap<(String, PluginId, ContributionId), String>,
    activations: BTreeMap<String, PanelActivationRecord>,
    next_generation: u64,
    next_receipt_seq: u64,
}

impl PanelPluginHost {
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

    pub fn installed_releases(&self) -> impl Iterator<Item = &VerifiedPanelRelease> {
        self.releases.values()
    }

    pub fn get_release(&self, id: &PluginId) -> Option<&VerifiedPanelRelease> {
        self.releases.get(id)
    }

    /// Record operator-approved capability grants for `(project_id, plugin_id)`.
    pub fn set_project_grants(
        &mut self,
        project_id: impl Into<String>,
        plugin_id: PluginId,
        granted: impl IntoIterator<Item = CapabilityId>,
    ) -> Result<Vec<CapabilityId>, PluginDenial> {
        if !self.releases.contains_key(&plugin_id) {
            return Err(PluginDenial::MissingRelease);
        }
        let set: BTreeSet<CapabilityId> = granted.into_iter().collect();
        let list: Vec<CapabilityId> = set.iter().cloned().collect();
        self.grants.insert((project_id.into(), plugin_id), set);
        Ok(list)
    }

    pub fn granted_capabilities(&self, project_id: &str, plugin_id: &PluginId) -> Vec<String> {
        self.grants
            .get(&(project_id.to_owned(), plugin_id.clone()))
            .map(|set| set.iter().map(|c| c.as_str().to_owned()).collect())
            .unwrap_or_default()
    }

    /// Activate an installed panel contribution for `project_id`, closing any prior
    /// active generation atomically and returning the new authoritative record.
    pub fn enable_panel(
        &mut self,
        project_id: impl Into<String>,
        plugin_id: &PluginId,
        contribution_id: &ContributionId,
        settings_toml: &[u8],
    ) -> Result<PanelActivationRecord, PluginDenial> {
        let project_id = project_id.into();
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
        let activation_id = format!(
            "act-{}-{}-{}-{active_gen}",
            hex_prefix(&self.host_epoch),
            plugin_id.as_str(),
            contribution_id.as_str()
        );
        let settings_digest = sha256_bytes(settings_toml);
        let record = PanelActivationRecord {
            activation_id: activation_id.clone(),
            project_id,
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
        activation_id: &str,
    ) -> Result<PanelDisableReceipt, PluginDenial> {
        let rec = self
            .activations
            .get_mut(activation_id)
            .ok_or(PluginDenial::StaleActivation)?;
        rec.closed = true;
        let key = (
            rec.project_id.clone(),
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
        project_id: &str,
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
        project_id: &str,
        plugin_id: &PluginId,
        contribution_id: &ContributionId,
        generation: u64,
        action_name: &str,
        args_json: &[u8],
        idempotency_key: &str,
    ) -> Result<PanelActionRecord, PluginDenial> {
        let key = (
            project_id.to_owned(),
            plugin_id.clone(),
            contribution_id.clone(),
        );
        let act_id = self
            .active_by_key
            .get(&key)
            .cloned()
            .ok_or(PluginDenial::StaleActivation)?;
        let next_seq = self.next_receipt_seq.saturating_add(1);
        let rec = self
            .activations
            .get_mut(&act_id)
            .ok_or(PluginDenial::StaleActivation)?;
        if rec.closed || rec.generation != generation {
            return Err(PluginDenial::StaleActivation);
        }
        if !rec.declared_actions.contains(action_name) {
            return Err(PluginDenial::ScopeMismatch);
        }
        if let Some(existing) = rec
            .actions
            .iter()
            .find(|a| !idempotency_key.is_empty() && a.idempotency_key == idempotency_key)
        {
            return Ok(existing.clone());
        }
        if rec.actions.len() >= MAX_PANEL_EFFECTS_PER_ACTIVATION {
            return Err(PluginDenial::CapacityExceeded);
        }
        self.next_receipt_seq = next_seq;
        let args_digest = sha256_bytes(args_json);
        let receipt_id = format!("rcpt-panel-action-{next_seq}");
        let mut mat = Vec::with_capacity(64 + action_name.len());
        mat.extend_from_slice(receipt_id.as_bytes());
        mat.extend_from_slice(&args_digest);
        mat.extend_from_slice(action_name.as_bytes());
        let receipt_digest = sha256_bytes(&mat);

        let entry = PanelActionRecord {
            ordinal: (rec.actions.len() + 1) as u64,
            action_name: action_name.to_owned(),
            idempotency_key: idempotency_key.to_owned(),
            args_digest,
            receipt_id,
            receipt_digest,
        };
        rec.actions.push(entry.clone());
        Ok(entry)
    }

    /// Return the full audit/evidence record for a live or closed activation generation.
    pub fn evidence(&self, activation_id: &str) -> Option<&PanelActivationRecord> {
        self.activations.get(activation_id)
    }

    pub fn active_generation_for(&self, plugin_id: &PluginId) -> (bool, u64) {
        let mut latest = 0u64;
        let mut enabled = false;
        for rec in self.activations.values() {
            if &rec.plugin_id == plugin_id && !rec.closed && rec.generation > latest {
                latest = rec.generation;
                enabled = true;
            }
        }
        (enabled, latest)
    }

    fn resolve_live(
        &self,
        project_id: &str,
        plugin_id: &PluginId,
        contribution_id: &ContributionId,
        generation: u64,
    ) -> Result<&PanelActivationRecord, PluginDenial> {
        let key = (
            project_id.to_owned(),
            plugin_id.clone(),
            contribution_id.clone(),
        );
        let act_id = self
            .active_by_key
            .get(&key)
            .ok_or(PluginDenial::StaleActivation)?;
        let rec = self
            .activations
            .get(act_id)
            .ok_or(PluginDenial::StaleActivation)?;
        if rec.closed || rec.generation != generation {
            return Err(PluginDenial::StaleActivation);
        }
        Ok(rec)
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use ring::signature::{Ed25519KeyPair, KeyPair};

    use super::*;
    use crate::plugin::{PublisherId, ReleaseAttestation};

    #[test]
    fn panel_plugin_host_enforces_generations_idempotency_and_disable_closure() {
        let seed = [11u8; 32];
        let kp = Ed25519KeyPair::from_seed_unchecked(&seed).unwrap();
        let pub_bytes: [u8; 32] = kp.public_key().as_ref().try_into().unwrap();
        let pub_id = PublisherId::try_from("tao3k.official".to_owned()).unwrap();
        let trust = ReleaseTrust::new(BTreeMap::from([(pub_id.clone(), pub_bytes)]));

        let manifest = br#"
id = "tao3k.graph-inspector"
version = "0.2.0"
contract_major = 1
requires_features = ["ui-slot-v1"]

[contributions.graph_panel]
kind = "ui-panel"

[contributions.graph_panel.panel]
runtime = "wasm-ui-ir"
slot_id = "workbench.inspector"
cardinality = "single"
entry_path = "panel.wasm"
read_scopes = ["graph.trajectory"]
declared_actions = ["graph.node.inspect"]
"#
        .to_vec();
        let att = ReleaseAttestation {
            publisher: pub_id,
            source_revision: [1u8; 20],
            build_evidence_digest: [2u8; 32],
        };
        let sig_bytes = kp.sign(&SignedPanelRelease::signing_bytes(&manifest, &att));
        let sig: [u8; 64] = sig_bytes.as_ref().try_into().unwrap();

        let mut host = PanelPluginHost::new(trust);
        host.install(SignedPanelRelease {
            manifest: &manifest,
            attestation: att,
            signature: &sig,
        })
        .unwrap();

        let pid = PluginId::try_from("tao3k.graph-inspector".to_owned()).unwrap();
        let cid = ContributionId::try_from("graph_panel".to_owned()).unwrap();
        let act = host.enable_panel("proj-1", &pid, &cid, b"").unwrap();
        assert_eq!(act.generation, 1);

        // Allowed action admitted & idempotent retry returns identical receipt.
        let r1 = host
            .admit_scoped_action(
                "proj-1",
                &pid,
                &cid,
                1,
                "graph.node.inspect",
                br#"{"node":"n1"}"#,
                "idem-1",
            )
            .unwrap();
        let r1_dup = host
            .admit_scoped_action(
                "proj-1",
                &pid,
                &cid,
                1,
                "graph.node.inspect",
                br#"{"node":"n1"}"#,
                "idem-1",
            )
            .unwrap();
        assert_eq!(r1.receipt_id, r1_dup.receipt_id);

        // Undeclared action denied with ScopeMismatch.
        assert_eq!(
            host.admit_scoped_action("proj-1", &pid, &cid, 1, "shell.exec", b"{}", "idem-2"),
            Err(PluginDenial::ScopeMismatch)
        );

        // Disable closes generation 1; subsequent actions fail with StaleActivation.
        let closed = host.disable_panel(&act.activation_id).unwrap();
        assert_eq!(closed.closed_generation, 1);
        assert_eq!(closed.recorded_actions, 1);
        assert_eq!(
            host.admit_scoped_action(
                "proj-1",
                &pid,
                &cid,
                1,
                "graph.node.inspect",
                b"{}",
                "idem-3"
            ),
            Err(PluginDenial::StaleActivation)
        );

        // Evidence remains queryable after closure with all admitted action receipts.
        let ev = host.evidence(&act.activation_id).unwrap();
        assert!(ev.closed);
        assert_eq!(ev.actions.len(), 1);
        assert_eq!(ev.actions[0].receipt_id, r1.receipt_id);
    }
}
